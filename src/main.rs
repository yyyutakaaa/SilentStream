#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_engine;

use eframe::egui;
use crate::audio_engine::AudioEngine;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use sysinfo::{System, Pid, ProcessRefreshKind};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent, menu::{Menu, MenuItem, MenuEvent}};

// Global tray icon storage to keep it alive
static mut TRAY_ICON: Option<TrayIcon> = None;

// Get config path
fn get_config_path() -> Option<PathBuf> {
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let config_dir = PathBuf::from(app_data).join("SilentStream");
        Some(config_dir.join("settings.txt"))
    } else {
        None
    }
}

fn load_settings() -> (Option<String>, Option<String>, f32, bool, bool) {
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 5 {
                    let input = if lines[0].is_empty() { None } else { Some(lines[0].to_string()) };
                    let output = if lines[1].is_empty() { None } else { Some(lines[1].to_string()) };
                    let threshold = lines[2].parse().unwrap_or(0.1);
                    let enabled = lines[3] == "true";
                    let start_with_windows = lines[4] == "true";
                    return (input, output, threshold, enabled, start_with_windows);
                } else if lines.len() >= 4 {
                    let input = if lines[0].is_empty() { None } else { Some(lines[0].to_string()) };
                    let output = if lines[1].is_empty() { None } else { Some(lines[1].to_string()) };
                    let threshold = lines[2].parse().unwrap_or(0.1);
                    let enabled = lines[3] == "true";
                    return (input, output, threshold, enabled, false);
                }
            }
        }
    }
    (None, None, 0.1, true, false)
}

fn save_settings(input: &str, output: &str, threshold: f32, enabled: bool, start_with_windows: bool) {
    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = format!("{}\n{}\n{}\n{}\n{}", input, output, threshold, enabled, start_with_windows);
        let _ = fs::write(&path, content);
    }
}

fn set_autostart(enable: bool) {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE | KEY_QUERY_VALUE) {
        if enable {
            if let Ok(exe_path) = std::env::current_exe() {
                let _ = key.set_value("SilentStream", &exe_path.to_string_lossy().to_string());
            }
        } else {
            let _ = key.delete_value("SilentStream");
        }
    }
}

fn is_autostart_enabled() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
        key.get_value::<String, _>("SilentStream").is_ok()
    } else {
        false
    }
}

struct SilentStreamApp {
    audio_engine: AudioEngine,
    input_devices: Vec<String>,
    output_devices: Vec<String>,
    selected_input_index: usize,
    selected_output_index: usize,
    is_processing: bool,
    vad_threshold: f32,
    noise_suppression_enabled: bool,
    status_message: String,
    first_frame: bool,
    show_settings: bool,
    start_with_windows: bool,
    show_cpu_usage: bool,
    cpu_usage: f32,
    last_cpu_check: Instant,
    sysinfo: System,
    current_pid: Pid,
    start_time: Instant,
    smoothed_volume: f32,
    is_minimized_to_tray: bool,
    last_restore_time: Option<Instant>,

    tray_listener_started: bool,
    restore_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    window_hwnd: std::sync::Arc<std::sync::Mutex<Option<isize>>>,
    // Shared flag so tray listener thread knows whether app is in tray mode
    in_tray_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}


// Load Icon Helper
fn load_app_icon() -> (Vec<u8>, u32, u32) {
    let image = image::load_from_memory(include_bytes!("../icon_256.png"))
        .expect("Failed to load icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    (rgba, width, height)
}

impl Default for SilentStreamApp {
    fn default() -> Self {
        let engine = AudioEngine::new();
        let inputs = engine.get_input_devices();
        let outputs = engine.get_output_devices();
        
        let (saved_input, saved_output, threshold, enabled, _start_win) = load_settings();
        
        let selected_input_index = saved_input.as_ref()
            .and_then(|name| inputs.iter().position(|d| d == name))
            .unwrap_or(0);
            
        let selected_output_index = saved_output.as_ref()
            .and_then(|name| outputs.iter().position(|d| d == name))
            .unwrap_or(0);
        
        let start_with_windows = is_autostart_enabled();
        
        let mut sysinfo = System::new();
        sysinfo.refresh_cpu();
        let current_pid = Pid::from(std::process::id() as usize);
        sysinfo.refresh_process_specifics(current_pid, ProcessRefreshKind::new().with_cpu());
        
        let restore_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        
        // Setup Tray Icon
        let tray_menu = Menu::new();
        let tray_open = MenuItem::new("Open SilentStream", true, None);
        let _ = tray_menu.append(&tray_open);
        
        // Load icon for tray
        let (icon_rgba, icon_width, icon_height) = load_app_icon();
        
        if let Ok(icon) = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height) {
             let _ = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("SilentStream")
                .with_icon(icon)
                .build()
                .map(|t| unsafe { TRAY_ICON = Some(t) });
        }
        
        // Tray Event Loop in a separate thread to ensure we catch events?
        // No, tray-icon uses a channel. We just need to make sure we poll it reliably.
        // We can however use the channel info to set the atomic flag which is checked every frame.

        Self {
            audio_engine: engine,
            input_devices: inputs,
            output_devices: outputs,
            selected_input_index,
            selected_output_index,
            is_processing: false,
            vad_threshold: threshold,
            noise_suppression_enabled: enabled,
            status_message: "Starting...".to_string(),
            first_frame: true,
            show_settings: false,
            start_with_windows,
            show_cpu_usage: false,
            cpu_usage: 0.0,
            last_cpu_check: Instant::now(),
            sysinfo,
            current_pid,
            start_time: Instant::now(),
            smoothed_volume: 0.0,
            is_minimized_to_tray: false,
            last_restore_time: None,
            tray_listener_started: false,
            restore_requested: restore_flag,
            window_hwnd: std::sync::Arc::new(std::sync::Mutex::new(None)),
            in_tray_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl SilentStreamApp {
    fn draw_animated_background(&mut self, ui: &egui::Ui) {
        let rect = ui.max_rect();
        let painter = ui.painter();
        let time = self.start_time.elapsed().as_secs_f32();
        
        // Get volume for reactivity
        let current_vol = if let Ok(vol) = self.audio_engine.current_volume.lock() {
            *vol * 5.0 // Gain up a bit for visualization
        } else { 
            0.0 
        };
        
        // Smooth the volume (decay)
        self.smoothed_volume = self.smoothed_volume * 0.9 + current_vol * 0.1;
        
        // Pulse base
        let pulse = (time * 0.5).sin() * 0.5 + 0.5; 
        
        // Colors: Dark Purple / Blue theme
        // Center glow linked to volume
        
        // Orb 1: Breathing background orb - Purple Gradient (Circles)
        {
            let center = egui::pos2(rect.right() - rect.width() * 0.2, rect.top() + rect.height() * 0.3);
            // Make base radius reactive to volume too, but subtler
            let reactive_scale = 1.0 + (self.smoothed_volume * 0.5); 
            let base_radius = (160.0 + (pulse * 20.0)) * reactive_scale;
            
            // Draw multiple concentric circles for gradient effect
            let colors = [
                (egui::Color32::from_rgba_premultiplied(88, 28, 135, 30), 0.4),  // Outer
                (egui::Color32::from_rgba_premultiplied(88, 28, 135, 50), 0.7),  // Mid
                (egui::Color32::from_rgba_premultiplied(107, 33, 168, 60), 0.9), // Inner
            ];

            for (color, scale) in colors.iter() {
                painter.circle_filled(center, base_radius * scale, *color);
            }
        }
        
        // Orb 2: Volume Reactive Orb - Bright Violet Gradient
        if self.smoothed_volume > 0.001 {
            let center = egui::pos2(rect.center().x, rect.bottom() - 60.0);
            let radius = 80.0 + (self.smoothed_volume * 400.0).clamp(0.0, 300.0);
            let alpha_base = (self.smoothed_volume * 255.0).clamp(0.0, 255.0);
            
            if alpha_base > 5.0 {
                 let layers = [
                    (0.5, 40),  // Outer faint
                    (0.7, 80),  // Mid glow
                    (0.9, 120), // Core bright
                ];
                
                for (scale, a_offset) in layers.iter() {
                    let a = (alpha_base * (*a_offset as f32 / 255.0)) as u8;
                    let color = egui::Color32::from_rgba_premultiplied(139, 92, 246, a);
                    painter.circle_filled(center, radius * scale, color);
                }
            }
        }
    }
    
    fn apply_custom_theme(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        
        // Darker theme for background integration
        let bg_primary = egui::Color32::from_rgb(20, 20, 25);
        let bg_secondary = egui::Color32::from_rgba_premultiplied(35, 35, 40, 230); // Slight transparency
        let accent_purple = egui::Color32::from_rgb(139, 92, 246);
        
        visuals.panel_fill = bg_primary;
        visuals.window_fill = bg_primary;
        
        visuals.widgets.noninteractive.bg_fill = bg_secondary;
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(12.0);
        
        visuals.widgets.active.bg_fill = accent_purple;
        visuals.widgets.active.weak_bg_fill = accent_purple;
        visuals.selection.bg_fill = accent_purple;
        visuals.selection.stroke = egui::Stroke::new(1.0, accent_purple);
        
        ctx.set_visuals(visuals);
    }
    
    fn save_current_settings(&self) {
        let input = self.input_devices.get(self.selected_input_index).map(|s| s.as_str()).unwrap_or("");
        let output = self.output_devices.get(self.selected_output_index).map(|s| s.as_str()).unwrap_or("");
        save_settings(input, output, self.vad_threshold, self.noise_suppression_enabled, self.start_with_windows);
    }
    
    fn auto_start(&mut self) {
        if self.input_devices.is_empty() || self.output_devices.is_empty() {
            self.status_message = "No audio devices found".to_string();
            return;
        }
        
        if let Ok(mut bp) = self.audio_engine.bypass.lock() {
            *bp = !self.noise_suppression_enabled;
        }
        
        if let Ok(mut th) = self.audio_engine.vad_threshold.lock() {
            *th = self.vad_threshold;
        }
        
        match self.audio_engine.start(self.selected_input_index, self.selected_output_index) {
            Ok(_) => {
                self.is_processing = true;
                self.status_message = "Processing audio".to_string();
            },
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }
    
    fn restart_audio(&mut self) {
        self.audio_engine.stop();
        self.is_processing = false;
        
        match self.audio_engine.start(self.selected_input_index, self.selected_output_index) {
            Ok(_) => {
                self.is_processing = true;
                self.status_message = "Processing audio".to_string();
                self.save_current_settings();
            },
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }
    
    fn update_cpu_usage(&mut self) {
        if self.show_cpu_usage && self.last_cpu_check.elapsed() > Duration::from_millis(1000) {
            self.sysinfo.refresh_process_specifics(
                self.current_pid, 
                ProcessRefreshKind::new().with_cpu()
            );
            
            if let Some(process) = self.sysinfo.process(self.current_pid) {
                let usage = process.cpu_usage();
                let num_cpus = self.sysinfo.cpus().len() as f32;
                if num_cpus > 0.0 {
                    self.cpu_usage = usage / num_cpus;
                } else {
                    self.cpu_usage = usage;
                }
            }
            self.last_cpu_check = Instant::now();
        }
    }
    
    fn ensure_tray_listener(&mut self, ctx: &egui::Context) {
        if !self.tray_listener_started {
            self.tray_listener_started = true;
            let ctx_clone = ctx.clone();
            let restore_flag = self.restore_requested.clone();
            let hwnd_store = self.window_hwnd.clone();
            let in_tray = self.in_tray_flag.clone();

            std::thread::spawn(move || {
                loop {
                    let mut got_click = false;

                    // Drain all events (must always drain to avoid channel backup)
                    while let Ok(_) = MenuEvent::receiver().try_recv() {
                        got_click = true;
                    }
                    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                        if let TrayIconEvent::Click { .. } = &event {
                            got_click = true;
                        }
                    }

                    // Only restore if we're actually in tray mode
                    if got_click && in_tray.load(std::sync::atomic::Ordering::SeqCst) {
                         in_tray.store(false, std::sync::atomic::Ordering::SeqCst);
                         restore_flag.store(true, std::sync::atomic::Ordering::SeqCst);

                         // Restore window from background thread
                         if let Ok(guard) = hwnd_store.lock() {
                             if let Some(hwnd) = *guard {
                                 let hwnd_copy = hwnd;
                                 std::thread::spawn(move || {
                                     unsafe {
                                         use windows_sys::Win32::UI::WindowsAndMessaging::*;
                                         ShowWindow(hwnd_copy as isize, SW_SHOW as i32);
                                         ShowWindow(hwnd_copy as isize, SW_RESTORE as i32);
                                         SetForegroundWindow(hwnd_copy as isize);
                                     }
                                 });
                             }
                         }

                         ctx_clone.request_repaint();
                    }

                    std::thread::sleep(Duration::from_millis(100));
                }
            });
        }
    }
    
    fn check_restore_request(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.restore_requested.load(std::sync::atomic::Ordering::Relaxed) {
             self.restore_requested.store(false, std::sync::atomic::Ordering::Relaxed);
             self.is_minimized_to_tray = false;
             self.last_restore_time = Some(Instant::now());

             // Egui restore commands
             ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
             ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
             ctx.send_viewport_cmd(egui::ViewportCommand::Focus);

             // Force Win32 Restore
             if let Ok(guard) = self.window_hwnd.lock() {
                 if let Some(hwnd) = *guard {
                     unsafe {
                         use windows_sys::Win32::UI::WindowsAndMessaging::*;
                         ShowWindow(hwnd as _, SW_RESTORE as i32);
                         ShowWindow(hwnd as _, SW_SHOW as i32);
                         SetForegroundWindow(hwnd as _);
                         BringWindowToTop(hwnd as _);
                     }
                 }
             }

             ctx.request_repaint();
        }
    }
}

impl eframe::App for SilentStreamApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // CAPTURE HWND ONCE
        if self.window_hwnd.lock().unwrap().is_none() {
             use raw_window_handle::{HasWindowHandle, RawWindowHandle};
             if let Ok(handle) = frame.window_handle() {
                 if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                     let hwnd = handle.hwnd.get();
                     *self.window_hwnd.lock().unwrap() = Some(hwnd);
                 }
             }
        }

        // Tray listener must always run to handle restore clicks
        self.ensure_tray_listener(ctx);
        self.check_restore_request(ctx, frame);

        // When minimized to tray: skip ALL rendering and UI work.
        // eframe 0.26 has a bug where request_repaint_after is ignored on Windows,
        // so we also use ViewportCommand::Visible(false) to tell winit the window is hidden.
        if self.is_minimized_to_tray {
            return;
        }

        // --- Everything below only runs when window is visible ---

        self.apply_custom_theme(ctx);

        if self.first_frame {
            self.first_frame = false;
            self.auto_start();
        }

        self.update_cpu_usage();

        // Repaint at ~60fps for smooth animation
        ctx.request_repaint_after(Duration::from_millis(16));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(16.0))
            .show(ctx, |ui| {
                self.draw_animated_background(ui);
                
                // Push cursor down past the manual header
                ui.add_space(20.0);
                
                // Header - Just Title and Subtitle
                ui.vertical_centered(|ui| {
                     ui.add_space(8.0);
                     ui.heading(
                         egui::RichText::new("🎵 SilentStream")
                             .size(32.0)
                             .strong()
                             .color(egui::Color32::from_rgb(220, 221, 222))
                     );
                     ui.add_space(4.0);
                     ui.label(
                        egui::RichText::new("Real-time Noise Suppression")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(142, 146, 151))
                    );
                });
                
                ui.add_space(12.0);

                // Control Bar (Buttons moved here, right aligned)
                ui.horizontal(|ui| {
                     ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        // Apply rounded style locally
                        ui.scope(|ui| {
                             ui.visuals_mut().widgets.inactive.rounding = egui::Rounding::same(8.0);
                             ui.visuals_mut().widgets.hovered.rounding = egui::Rounding::same(8.0);
                             ui.visuals_mut().widgets.active.rounding = egui::Rounding::same(8.0);
                             
                             // Custom Settings Button (Centered Gear)
                             let (s_rect, s_res) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                             if s_res.clicked() {
                                self.show_settings = !self.show_settings;
                             }
                             let s_res = s_res.on_hover_text("Settings"); // Chain tooltip logic

                             let s_visuals = ui.style().interact(&s_res);
                             let s_bg = if s_res.hovered() { 
                                 egui::Color32::from_rgba_premultiplied(60, 60, 65, 255) 
                             } else { 
                                 egui::Color32::from_rgba_premultiplied(45, 45, 50, 255) 
                             };
                             ui.painter().rect(s_rect, egui::Rounding::same(8.0), s_bg, egui::Stroke::NONE);
                             
                             // Paint Gear Icon Centered
                             ui.painter().text(
                                 s_rect.center(),
                                 egui::Align2::CENTER_CENTER,
                                 "⚙",
                                 egui::FontId::proportional(16.0),
                                 s_visuals.text_color()
                             );
                             
                             ui.add_space(8.0);
                             
                             // Custom Hide Button (Arrow to South-East)
                             let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                             
                             // Handle interaction - Minimize to tray
                             if response.clicked() {
                                self.is_minimized_to_tray = true;
                                self.in_tray_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                                // Hide window: use both egui commands and Win32
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                if let Ok(guard) = self.window_hwnd.lock() {
                                    if let Some(hwnd) = *guard {
                                        unsafe {
                                            use windows_sys::Win32::UI::WindowsAndMessaging::*;
                                            ShowWindow(hwnd as isize, SW_HIDE as i32);
                                        }
                                    }
                                }
                             }
                             
                             // Paint button background
                             let visuals = ui.style().interact(&response);
                             let bg_color = if response.hovered() { 
                                 egui::Color32::from_rgba_premultiplied(60, 60, 65, 255) 
                             } else { 
                                 egui::Color32::from_rgba_premultiplied(45, 45, 50, 255) 
                             };
                             
                             ui.painter().rect(rect, egui::Rounding::same(8.0), bg_color, egui::Stroke::NONE);
                             
                             // Paint Arrow Icon (Diagonal Down-Right)
                             let center = rect.center();
                             let arrow_color = visuals.text_color();
                             let size = 6.0;
                             
                             // Diagonal line
                             ui.painter().line_segment(
                                 [egui::pos2(center.x - size, center.y - size), egui::pos2(center.x + size, center.y + size)], 
                                 egui::Stroke::new(1.5, arrow_color)
                             );
                             
                             // Arrow head (At bottom right)
                             ui.painter().line_segment(
                                 [egui::pos2(center.x + size, center.y + size), egui::pos2(center.x + size, center.y )], 
                                 egui::Stroke::new(1.5, arrow_color)
                             );
                             ui.painter().line_segment(
                                 [egui::pos2(center.x + size, center.y + size), egui::pos2(center.x, center.y + size)], 
                                 egui::Stroke::new(1.5, arrow_color)
                             );
                        });
                     });
                });
                
                ui.add_space(4.0);
                
                // Settings Panel
                if self.show_settings {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(38, 40, 43, 240))
                        .rounding(12.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label("⚙ Advanced Settings");
                            ui.add_space(8.0);
                            
                            let mut start_win = self.start_with_windows;
                            if ui.checkbox(&mut start_win, "Start with Windows").changed() {
                                self.start_with_windows = start_win;
                                set_autostart(self.start_with_windows);
                                self.save_current_settings();
                            }
                            
                            ui.add_space(4.0);
                            
                            if ui.checkbox(&mut self.show_cpu_usage, "Show CPU Usage").changed() {
                                self.last_cpu_check = Instant::now() - Duration::from_secs(2);
                            }
                            
                            if self.show_cpu_usage {
                                ui.label(format!("SilentStream CPU: {:.1}%", self.cpu_usage));
                            }
                        });
                    ui.add_space(10.0);
                }

                // Cards with slight transparency
                let card_fill = egui::Color32::from_rgba_premultiplied(43, 45, 49, 240);
                
                // Audio Devices
                egui::Frame::none()
                    .fill(card_fill)
                    .rounding(12.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Audio Devices").strong());
                        ui.add_space(8.0);

                        ui.label("Input:");
                        let selected_input = self.input_devices.get(self.selected_input_index).map(|s| s.as_str()).unwrap_or("No device");
                        let old_in = self.selected_input_index;
                        egui::ComboBox::from_id_source("input").selected_text(selected_input).width(ui.available_width()-8.0).show_ui(ui, |ui| {
                            for (i, name) in self.input_devices.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_input_index, i, name);
                            }
                        });
                        if old_in != self.selected_input_index { self.restart_audio(); }

                        ui.add_space(8.0);
                        ui.label("Output:");
                        let selected_output = self.output_devices.get(self.selected_output_index).map(|s| s.as_str()).unwrap_or("No device");
                        let old_out = self.selected_output_index;
                        egui::ComboBox::from_id_source("output").selected_text(selected_output).width(ui.available_width()-8.0).show_ui(ui, |ui| {
                            for (i, name) in self.output_devices.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_output_index, i, name);
                            }
                        });
                        if old_out != self.selected_output_index { self.restart_audio(); }
                    });

                ui.add_space(10.0);
                
                // Audio Settings
                egui::Frame::none()
                    .fill(card_fill)
                    .rounding(12.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Audio Settings").strong());
                        ui.add_space(8.0);
                        
                        if ui.checkbox(&mut self.noise_suppression_enabled, "Enable Noise Suppression").changed() {
                            if let Ok(mut bp) = self.audio_engine.bypass.lock() {
                                *bp = !self.noise_suppression_enabled;
                            }
                            self.save_current_settings();
                        }
                        
                        ui.add_space(10.0);
                        ui.label(format!("VAD Threshold: {:.2}", self.vad_threshold));
                        ui.add_space(4.0);
                        
                        // Slider
                        let slider_width = ui.available_width() - 8.0;
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(slider_width, 18.0), egui::Sense::click_and_drag());
                        
                        if response.dragged() || response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                self.vad_threshold = t * 0.5;
                                if let Ok(mut th) = self.audio_engine.vad_threshold.lock() { *th = self.vad_threshold; }
                            }
                        }
                        if response.drag_released() { self.save_current_settings(); }
                        
                        // Draw slider
                        let p = ui.painter();
                        p.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - 3.0), egui::vec2(rect.width(), 6.0)),
                            3.0, egui::Color32::from_rgb(54, 57, 63)
                        );
                        let fill_w = rect.width() * (self.vad_threshold / 0.5).clamp(0.0, 1.0);
                        p.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(rect.left(), rect.center().y - 3.0), egui::vec2(fill_w, 6.0)),
                            3.0, egui::Color32::from_rgb(139, 92, 246) // Purple
                        );
                        let kx = rect.left() + fill_w;
                        p.circle_filled(egui::pos2(kx.clamp(rect.left()+7.0, rect.right()-7.0), rect.center().y), 7.0, egui::Color32::WHITE);
                    });
                
                ui.add_space(12.0);
                
                // Bottom Status
                ui.vertical_centered(|ui| {
                    let color = if self.is_processing {
                        egui::Color32::from_rgb(67, 181, 129)
                    } else if self.status_message.contains("Error") {
                        egui::Color32::from_rgb(240, 71, 71)
                    } else {
                        egui::Color32::from_rgb(142, 146, 151)
                    };
                    
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Center), |ui| {
                             let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                             ui.painter().circle_filled(rect.center(), 3.0, color);
                             
                             ui.label(egui::RichText::new(&self.status_message).size(11.0).color(color));
                        });
                    });
                });
            });
    }
}

fn main() -> eframe::Result<()> {
    let (icon_rgba, icon_width, icon_height) = load_app_icon();
    let icon_data = egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([360.0, 480.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_title("SilentStream")
            .with_icon(icon_data),
        ..Default::default()
    };
    
    eframe::run_native(
        "SilentStream",
        options,
        Box::new(|_cc| Box::new(SilentStreamApp::default())),
    )
}
