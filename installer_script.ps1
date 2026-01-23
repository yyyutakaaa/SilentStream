<#
.SYNOPSIS
SilentStream Installer / Launcher

.DESCRIPTION
Checks for Virtual Audio Cable (VB-Cable). If missing, prompts to install.
Then launches SilentStream.
#>

$VBCableExists = Get-PnpDevice -FriendlyName "*VB-Audio Virtual Cable*" -ErrorAction SilentlyContinue

if (-not $VBCableExists) {
    Write-Host "VB-Cable Virtual Audio Device not found!" -ForegroundColor Yellow
    Write-Host "SilentStream requires a virtual output device to route audio to."
    
    $result = [System.Windows.MessageBox]::Show("VB-Cable is missing. Do you want to download and install it now?", "Missing Dependency", "YesNo", "Warning")
    
    if ($result -eq "Yes") {
        Write-Host "Downloading VB-Cable..."
        # Warning: Direct download link might change. Using a generic placeholder or instructions.
        # For this exercise, we'll open the website.
        Start-Process "https://vb-audio.com/Cable/"
        Write-Host "Please install VB-Cable driver and restart this script."
        Read-Host "Press Enter to exit..."
        exit
    } else {
        Write-Host "Cannot proceed without a virtual audio device."
        exit
    }
} else {
    Write-Host "Virtual Audio Device found." -ForegroundColor Green
}

# Launch the app
$ScriptPath = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ExePath = Join-Path $ScriptPath "target\release\silent_stream.exe"

if (Test-Path $ExePath) {
    Write-Host "Launching SilentStream..."
    Start-Process $ExePath
} else {
    Write-Host "Executable not found at $ExePath" -ForegroundColor Red
    Write-Host "Please build the project using 'cargo build --release'"
}
