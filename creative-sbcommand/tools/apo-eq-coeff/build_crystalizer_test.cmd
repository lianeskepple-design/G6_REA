@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cl /nologo /TC /std:c11 /W4 /WX /fp:strict /I "%~dp0..\..\reference" "%~dp0crystalizer_chain_test.c" "%~dp0..\..\reference\g6_crystalizer.c" "%~dp0..\..\reference\g6_crystalizer_legacy.c" /Fe:"%~dp0crystalizer_chain_test.exe"
if errorlevel 1 exit /b 1
"%~dp0crystalizer_chain_test.exe" > "%~dp0crystalizer_out.txt"
if errorlevel 1 exit /b 2
exit /b 0
