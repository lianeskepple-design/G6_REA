@echo off
set "GHIDRA=D:\tools\ghidra_12.1.4_PUBLIC"
set "PROJ=D:\tmp_ghidra_spi"
set "SCRIPT=%PROJ%\scripts"
set "OUT=%PROJ%\spi_dump.txt"
if exist "%PROJ%" rmdir /s /q "%PROJ%"
mkdir "%PROJ%"
mkdir "%SCRIPT%"
copy /y "%~dp0..\apo-eq-coeff\ghidra_scripts\DumpC.java" "%SCRIPT%\DumpC.java" >nul
"%GHIDRA%\support\analyzeHeadless.bat" "%PROJ%" SpiProj -import "C:\Windows\System32\KSUSBSPI64.dll" -scriptPath "%SCRIPT%" -postScript DumpC.java 0x44C1AC 0x44C62C 0x44C380 0x44C404 0x44C488 0x44C510 0x44C594 0x41B244 0x41B170 0x41B1CC -analysisTimeoutPerFile 600 > "%OUT%" 2>&1
copy /y "%OUT%" "%~dp0spi_dump.txt" >nul
exit /b %errorlevel%
