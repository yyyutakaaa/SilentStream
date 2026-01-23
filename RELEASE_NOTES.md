# SilentStream v0.1.0 - Initial Release

SilentStream is a lightweight Windows application designed for real-time audio noise suppression and routing. It leverages advanced neural network algorithms to provide clean microphone audio for communication and recording.

## Key Features
- **Real-time Noise Suppression:** Utilizes the RNNoise algorithm to filter out background noise such as fans, keyboard clicks, and ambient sounds.
- **Audio Routing:** Seamlessly routes processed audio to a virtual output device (e.g., VB-Cable) for use in applications like Discord, Teams, Zoom, or OBS.
- **System Tray Integration:** Runs unobtrusively in the background with a minimize-to-tray feature.
- **Modern GUI:** Simple interface for monitoring audio levels, adjusting thresholds, and managing settings.
- **Automatic Startup:** Includes an option to launch automatically with Windows.

## Installation
1. Download `SilentStream_Setup.exe` from the assets below.
2. Run the installer and follow the on-screen instructions.
3. **Note:** A virtual audio driver is required. If you do not have one, it is recommended to install [VB-Audio Virtual Cable](https://vb-audio.com/Cable/).

## Package Options
- **SilentStream_Setup.exe:** Recommended Windows installer for most users.
- **SilentStream_Installer.zip:** Portable version containing the executable and an installation script (`install.bat`).

## System Requirements
- Windows 10 or 11 (64-bit).
- A virtual audio cable driver.

## Credits
Built with Rust and powered by open-source libraries including eframe, cpal, and nnnoiseless.
