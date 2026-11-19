@echo off
call "%~dp0build_crystalizer_test.cmd" || exit /b 1
"%~dp0..\audio-env\Scripts\python.exe" -I "%~dp0crystalizer_chain_ref.py" "%~dp0crystalizer_out.txt"
exit /b %errorlevel%
