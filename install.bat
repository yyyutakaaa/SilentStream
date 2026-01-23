@echo off
title Install SilentStream
echo Requesting Administrator privileges...
powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File \"%~dp0setup.ps1\"'"
exit
