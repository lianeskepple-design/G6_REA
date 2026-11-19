@echo off
set "GHIDRA=D:\tools\ghidra_12.1.4_PUBLIC"
set "PROJ=D:\tmp_ghidra_dialog"
set "SCRIPT=%PROJ%\scripts"
set "OUT=%PROJ%\dialog_dump.txt"
if exist "%PROJ%" rmdir /s /q "%PROJ%"
mkdir "%PROJ%"
mkdir "%SCRIPT%"
copy /y "%~dp0..\apo-eq-coeff\ghidra_scripts\DumpC.java" "%SCRIPT%\DumpC.java" >nul
"%GHIDRA%\support\analyzeHeadless.bat" "%PROJ%" DialogProj -import "C:\Windows\System32\KSUSBAPO64.dll" -scriptPath "%SCRIPT%" -postScript DumpC.java 0x180077b20 0x180077860 0x180077ee0 0x1800d77e8 -analysisTimeoutPerFile 900 > "%OUT%" 2>&1
copy /y "%OUT%" "%~dp0dialog_dump.txt" >nul
exit /b %errorlevel%
