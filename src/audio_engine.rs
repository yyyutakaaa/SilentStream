use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use ringbuf::HeapRb;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Constant for RNNoise frame size
const RNNOISE_FRAME_SIZE: usize = 480;

pub struct AudioEngine {
    _input_stream: Option<Stream>,
    _output_stream: Option<Stream>,
    _processing_handle: Option<thread::JoinHandle<()>>,
    is_running: Arc<Mutex<bool>>,
    pub vad_threshold: Arc<Mutex<f32>>,
    pub bypass: Arc<Mutex<bool>>,
    pub current_volume: Arc<Mutex<f32>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            _input_stream: None,
            _output_stream: None,
            _processing_handle: None,
            is_running: Arc::new(Mutex::new(false)),
            vad_threshold: Arc::new(Mutex::new(0.5)), 
            bypass: Arc::new(Mutex::new(false)),
            current_volume: Arc::new(Mutex::new(0.0)),
        }
    }

    pub fn get_input_devices(&self) -> Vec<String> {
        let host = cpal::default_host();
        match host.input_devices() {
            Ok(devices) => devices.map(|d| d.name().unwrap_or("Unknown".to_string())).collect(),
            Err(_) => vec![],
        }
    }

    pub fn get_output_devices(&self) -> Vec<String> {
        let host = cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.map(|d| d.name().unwrap_or("Unknown".to_string())).collect(),
            Err(_) => vec![],
        }
    }

    pub fn start(&mut self, input_device_index: usize, output_device_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let input_devices: Vec<_> = host.input_devices()?.collect();
        let output_devices: Vec<_> = host.output_devices()?.collect();

        // Basic selection logic
        let input_device = input_devices.get(input_device_index).ok_or("Invalid input device index")?;
        let output_device = output_devices.get(output_device_index).ok_or("Invalid output device index")?;

        // Standard logic: Input -> RingBuffer -> Processing Thread -> RingBuffer -> Output
        // Capacity: Enough for ~100ms of audio
        let ring_buffer_size = 8192; 
        
        let rb_in = HeapRb::<f32>::new(ring_buffer_size);
        let (mut in_prod, mut in_cons) = rb_in.split();
        
        let rb_out = HeapRb::<f32>::new(ring_buffer_size);
        let (mut out_prod, mut out_cons) = rb_out.split();

        // Configure Input Stream
        let input_config: StreamConfig = input_device.default_input_config()?.into();
        let input_channels = input_config.channels as usize;
        
        let input_sample_rate = input_config.sample_rate.0;
        
        // Input Callback
        let input_stream = input_device.build_input_stream(
            &input_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks(input_channels) {
                    let sample = frame[0]; // Take first channel (Left)
                    let _ = in_prod.push(sample); // Ignore if full
                }
            },
            |err| eprintln!("Input stream error: {}", err),
            None
        )?;

        // Output Callback
        let output_config: StreamConfig = output_device.default_output_config()?.into();
        let output_channels = output_config.channels as usize;
        
        let output_stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(output_channels) {
                    let sample = out_cons.pop().unwrap_or(0.0);
                    for channel in frame {
                        *channel = sample; 
                    }
                }
            },
            |err| eprintln!("Output stream error: {}", err),
            None
        )?;

        // Processing Thread
        let is_running_clone = self.is_running.clone();
        let vad_threshold_clone = self.vad_threshold.clone();
        let bypass_clone = self.bypass.clone();
        let current_volume_clone = self.current_volume.clone();
        
        let target_sample_rate = 48000;
        
        let processing_handle = thread::spawn(move || {
            let mut denoise_state = nnnoiseless::DenoiseState::new();
            
            // Buffers
            let mut raw_buffer = [0.0; RNNOISE_FRAME_SIZE]; // 480 samples
            let mut processed_buffer = [0.0; RNNOISE_FRAME_SIZE];
            
            // Resampler setup
            let mut resampler: Option<rubato::FftFixedOut<f32>> = if input_sample_rate != target_sample_rate {
                 use rubato::{Resampler, FftFixedOut};
                 match FftFixedOut::<f32>::new(
                    input_sample_rate as usize, 
                    target_sample_rate as usize, 
                    RNNOISE_FRAME_SIZE, 
                    2, 
                    1
                ) {
                    Ok(r) => Some(r),
                    Err(e) => { eprintln!("Resampler init failed: {}", e); None }
                }
            } else { None };
            
            let mut resampler_input: Vec<Vec<f32>> = vec![vec![]; 1];

            while *is_running_clone.lock().unwrap() {
                // Get current control values
                let threshold = *vad_threshold_clone.lock().unwrap();
                let is_bypassed = *bypass_clone.lock().unwrap();

                if let Some(ref mut r) = resampler {
                    use rubato::Resampler;
                    let frames_needed = r.input_frames_next();
                    
                    if in_cons.len() >= frames_needed {
                         let mut input_chunk = vec![0.0; frames_needed];
                         for i in 0..frames_needed {
                             input_chunk[i] = in_cons.pop().unwrap_or(0.0);
                         }
                         

                         resampler_input[0] = input_chunk;
                         
                         match r.process(&resampler_input, None) {
                             Ok(resampler_output_new) => {
                                 // rubato returns new buffers
                                 let chunk = &resampler_output_new[0];
                                 
                                 if is_bypassed {
                                     for sample in chunk.iter() {
                                         let _ = out_prod.push(*sample);
                                     }
                                 } else {
                                     // Scale up for RNNoise
                                     let mut scaled_input = [0.0; RNNOISE_FRAME_SIZE];
                                     for (i, s) in chunk.iter().enumerate().take(RNNOISE_FRAME_SIZE) {
                                         scaled_input[i] = s * 32768.0;
                                     }

                                     let vad_prob = denoise_state.process_frame(&mut processed_buffer, &scaled_input);
                                     
                                     if vad_prob < threshold {
                                         for _ in 0..RNNOISE_FRAME_SIZE {
                                             let _ = out_prod.push(0.0);
                                         }
                                     } else {
                                          for sample in processed_buffer.iter() {
                                             let _ = out_prod.push(sample / 32768.0);
                                         }
                                         
                                         // Calculate volume from PROCESSED output
                                         let mut sum_sq = 0.0;
                                         for sample in processed_buffer.iter() {
                                             let s = sample / 32768.0;
                                             sum_sq += s * s;
                                         }
                                         let rms = (sum_sq / RNNOISE_FRAME_SIZE as f32).sqrt();
                                         if let Ok(mut vol) = current_volume_clone.lock() {
                                             *vol = rms;
                                         }
                                     }
                                 }
                             },
                             Err(e) => eprintln!("Resampling error: {}", e),
                         }
                    } else {
                        thread::sleep(Duration::from_millis(5));
                    }
                } else {
                     if in_cons.len() >= RNNOISE_FRAME_SIZE {
                         for i in 0..RNNOISE_FRAME_SIZE {
                             raw_buffer[i] = in_cons.pop().unwrap_or(0.0);
                         }
                         
                         
                         if is_bypassed {
                             for sample in raw_buffer.iter() {
                                 let _ = out_prod.push(*sample);
                             }
                         } else {
                             let mut scaled_input = [0.0; RNNOISE_FRAME_SIZE];
                             for (i, s) in raw_buffer.iter().enumerate() {
                                 scaled_input[i] = s * 32768.0;
                             }

                             let vad_prob = denoise_state.process_frame(&mut processed_buffer, &scaled_input);
                             
                             if vad_prob < threshold {
                                 for _ in 0..RNNOISE_FRAME_SIZE {
                                     let _ = out_prod.push(0.0);
                                 }
                             } else {
                                 for sample in processed_buffer.iter() {
                                     let _ = out_prod.push(sample / 32768.0);
                                 }

                                 // Calculate volume from PROCESSED output
                                 let mut sum_sq = 0.0;
                                 for sample in processed_buffer.iter() {
                                     let s = sample / 32768.0;
                                     sum_sq += s * s;
                                 }
                                 let rms = (sum_sq / RNNOISE_FRAME_SIZE as f32).sqrt();
                                 if let Ok(mut vol) = current_volume_clone.lock() {
                                     *vol = rms;
                                 }
                             }
                         }
                    } else {
                        thread::sleep(Duration::from_millis(5));
                    }
                }
            }
        });

        input_stream.play()?;
        output_stream.play()?;
        
        *self.is_running.lock().unwrap() = true;
        
        self._input_stream = Some(input_stream);
        self._output_stream = Some(output_stream);
        self._processing_handle = Some(processing_handle);

        Ok(())
    }
    
    pub fn stop(&mut self) {
        *self.is_running.lock().unwrap() = false;
    }
}
