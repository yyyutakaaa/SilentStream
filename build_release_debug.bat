@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
"%USERPROFILE%\.cargo\bin\cargo.exe" build --release > build_log.txt 2>&1
