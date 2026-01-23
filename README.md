# SilentStream

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows&logoColor=white)

SilentStream is a powerful Windows application for audio noise suppression and routing. It utilizes advanced filtering (Neural Network Noise Suppression) to transfer microphone audio to a virtual cable without noise.

## Features
- **Real-time Noise Suppression:** Uses `nnnoiseless` for AI-driven noise suppression.
- **Microphone Routing:** Route audio from any input to a virtual output (e.g., VB-Cable).
- **System Tray Integration:** Minimizes to the system tray for unobtrusive usage.
- **Configuration:** Saves settings such as threshold values and autostart preferences.

## Requirements
- **OS:** Windows 10/11
- **Drivers:** [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) (or another virtual audio cable).

## Installation & Usage

### Recommended Installation (Recommended)
1. Download the latest **[SilentStream_Setup.zip](https://github.com/yyyutakaaa/SilentStream/releases/download/v1.0.0/SilentStream_Setup.zip)** from the [Releases](https://github.com/yyyutakaaa/SilentStream/releases/tag/v1.0.0) page.
2. Run the installer to set up SilentStream on your system.
3. **Note on Security:** As this is a new and unsigned application, Windows SmartScreen or other antivirus software may flag it as "unrecognized" or "unsafe". This is simply because the app hasn't built up a high download reputation yet. The application is completely virus-free and safe to use.

### Alternative Installation (Manual)
1. Download the `SilentStream_Installer.zip`.
2. Extract the zip file.
3. Double-click **`install.bat`**.
   - This will install the app to your AppData folder.
   - Creates shortcuts on your Desktop and Start Menu.
   - Checks if you have the required Virtual Cable driver.

### Building from Source
1. Ensure [Rust](https://www.rust-lang.org/) is installed.
2. Clone the repository.
3. Build the release version:
   ```powershell
   cargo build --release
   ```
4. You can package the zip installer by running `package_release.ps1`.

## Development
- **GUI Framework:** `eframe` (egui)
- **Audio Backend:** `cpal`

## Credits
Special thanks to the open-source community. Key libraries used:
- **[eframe](https://github.com/emilk/egui)** - Immediate mode GUI
- **[cpal](https://github.com/RustAudio/cpal)** - Cross-platform audio I/O
- **[nnnoiseless](https://github.com/gwy15/nnnoiseless)** - Rust port of RNNoise
- **[tray-icon](https://github.com/tauri-apps/tray-icon)** - System tray functionality
- **[sysinfo](https://github.com/GuillaumeGomez/sysinfo)** - System information
- **[rubato](https://github.com/HEnquist/rubato)** - Asynchronous audio resampling
- **[ringbuf](https://github.com/m-ou-se/ringbuf)** - Lock-free SPSC ring buffer
