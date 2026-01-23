@echo off
echo Building SilentStream Setup Wizard...

REM Check if ISCC is in PATH
where /q iscc
if %ERRORLEVEL% EQU 0 (
    iscc setup.iss
    goto :success
)

REM Check common installation paths
if exist "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" (
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" setup.iss
    goto :success
)

if exist "C:\Program Files\Inno Setup 6\ISCC.exe" (
    "C:\Program Files\Inno Setup 6\ISCC.exe" setup.iss
    goto :success
)

:error
echo.
echo [ERROR] Inno Setup Compiler (ISCC) not found!
echo.
echo To build the Install Wizard, you need to install Inno Setup 6.
echo Please download it from: https://jrsoftware.org/isdl.php
echo.
echo After installing, run this script again.
echo.
pause
exit /b 1

:success
echo.
echo [SUCCESS] Installer created successfully in the 'dist' folder!
echo.
pause
