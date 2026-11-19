@echo off
set "PY=%~dp0..\audio-env\Scripts\python.exe"
"%PY%" -I "%~dp0measure-scout.py" --self-test || exit /b 1
"%PY%" -I "%~dp0usbpcap-single-band.py" --self-test || exit /b 2
exit /b 0
