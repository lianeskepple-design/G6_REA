@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I "%~dp0..\..\reference" "%~dp0svm_test.c" "%~dp0..\..\reference\g6_svm.c" /Fe:"%~dp0svm_test.exe"
if errorlevel 1 exit /b 1
"%~dp0svm_test.exe" > "%~dp0svm_out.txt"
exit /b %errorlevel%
