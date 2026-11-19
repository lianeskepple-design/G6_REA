@echo off
call "%~dp0build_surround_test.cmd" || exit /b 1
"%~dp0..\audio-env\Scripts\python.exe" -I "%~dp0surround_ref.py" "%~dp0surround_out.txt"
exit /b %errorlevel%
