@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I "%~dp0..\..\reference" "%~dp0apo_parity.c" "%~dp0..\..\reference\g6_eq_apo.c" /Fe:"%~dp0apo_parity.exe"
if errorlevel 1 exit /b 1
"%~dp0apo_parity.exe" > "%~dp0c_out.txt"
if errorlevel 1 exit /b 2
exit /b 0
