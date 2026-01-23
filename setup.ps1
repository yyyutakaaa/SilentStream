$ErrorActionPreference = "Stop"

$AppName = "SilentStream"
$ExeName = "silent_stream.exe"
$InstallDir = "$env:LOCALAPPDATA\Programs\$AppName"
$SourceExe = Join-Path $PSScriptRoot $ExeName

# 1. Create Installation Directory
Write-Host "Installing $AppName to $InstallDir..."
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

# 2. Copy Executable
if (Test-Path $SourceExe) {
    Copy-Item -Path $SourceExe -Destination $InstallDir -Force
    Write-Host "Executable copied." -ForegroundColor Green
} else {
    Write-Error "Installer Error: $ExeName not found in the current directory."
}

# 3. Create Shortcuts
$WshShell = New-Object -comObject WScript.Shell

# Desktop Shortcut
$DesktopPath = [Environment]::GetFolderPath("Desktop")
$ShortcutPath = Join-Path $DesktopPath "$AppName.lnk"
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = Join-Path $InstallDir $ExeName
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Description = "Launch SilentStream"
$Shortcut.Save()
Write-Host "Desktop shortcut created." -ForegroundColor Green

# Start Menu Shortcut
$StartMenuPath = [Environment]::GetFolderPath("StartMenu") # System.Environment.SpecialFolder.StartMenu usually points to Roaming/Microsoft/Windows/Start Menu/Programs
$ProgramShortcutDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\$AppName"
if (-not (Test-Path $ProgramShortcutDir)) {
    New-Item -ItemType Directory -Force -Path $ProgramShortcutDir | Out-Null
}
$StartMenuShortcutPath = Join-Path $ProgramShortcutDir "$AppName.lnk"
$SMShortcut = $WshShell.CreateShortcut($StartMenuShortcutPath)
$SMShortcut.TargetPath = Join-Path $InstallDir $ExeName
$SMShortcut.WorkingDirectory = $InstallDir
$SMShortcut.Description = "Launch SilentStream"
$SMShortcut.Save()
Write-Host "Start Menu shortcut created." -ForegroundColor Green

# 4. Check for Dependencies (VB-Cable)
Write-Host "Checking for Virtual Audio Cable..."
$VBCable = Get-PnpDevice -FriendlyName "*VB-Audio Virtual Cable*" -ErrorAction SilentlyContinue

if (-not $VBCable) {
    Write-Host "WARNING: VB-Audio Virtual Cable not found." -ForegroundColor Yellow
    $result = [System.Windows.MessageBox]::Show("VB-Cable driver is missing. SilentStream needs this to function. Download now?", "Missing Driver", "YesNo", "Warning")
    if ($result -eq "Yes") {
        Start-Process "https://vb-audio.com/Cable/"
    }
} else {
    Write-Host "Virtual Audio Cable found." -ForegroundColor Green
}

Write-Host "Installation Complete!" -ForegroundColor Cyan
Write-Host "You can now launch SilentStream from your Desktop."

# Optional: Launch now
$Launch = [System.Windows.MessageBox]::Show("Installation finished. Launch SilentStream now?", "Installed", "YesNo", "Information")
if ($Launch -eq "Yes") {
    Start-Process (Join-Path $InstallDir $ExeName)
}
