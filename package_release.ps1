$ErrorActionPreference = "Stop"

$DistDir = Join-Path $PSScriptRoot "dist"
$ReleaseExe = Join-Path $PSScriptRoot "target\release\silent_stream.exe"
$SetupScript = Join-Path $PSScriptRoot "setup.ps1"
$InstallBat = Join-Path $PSScriptRoot "install.bat"
$ZipName = Join-Path $PSScriptRoot "SilentStream_Installer.zip"

Write-Host "Packaging SilentStream..."

# Check if release build exists
if (-not (Test-Path $ReleaseExe)) {
    Write-Error "Release build not found at $ReleaseExe. Please run 'cargo build --release' first."
}

# Clear old dist
if (Test-Path $DistDir) { Remove-Item $DistDir -Recurse -Force }
New-Item -ItemType Directory -Path $DistDir | Out-Null

# Copy files
Copy-Item $ReleaseExe -Destination $DistDir
Copy-Item $SetupScript -Destination $DistDir
Copy-Item $InstallBat -Destination $DistDir

Write-Host "Files copied to $DistDir"

# Wait a moment for file handles to close
Start-Sleep -Seconds 2

# Zip it
if (Test-Path $ZipName) { Remove-Item $ZipName -Force }
Compress-Archive -Path "$DistDir\*" -DestinationPath $ZipName

Write-Host "Package created successfully: $ZipName" -ForegroundColor Green
Write-Host "You can share this zip file with users."
