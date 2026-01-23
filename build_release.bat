@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if %ERRORLEVEL% neq 0 (
    echo Failed to load vcvars64.bat
    exit /b %ERRORLEVEL%
)
"%USERPROFILE%\.cargo\bin\cargo.exe" build --release
pause
