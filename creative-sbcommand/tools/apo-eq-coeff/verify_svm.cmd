@echo off
call "%~dp0build_svm_test.cmd" || exit /b 1
"%~dp0..\audio-env\Scripts\python.exe" -I "%~dp0svm_ref.py" "%~dp0svm_out.txt"
exit /b %errorlevel%
