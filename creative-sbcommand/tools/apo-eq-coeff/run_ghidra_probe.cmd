@echo off
rem G6Probe: displacement scan / decompile / vtable / symbol / disasm for KSUSBAPO64.dll.
rem Usage: run_ghidra_probe.cmd scan 0xf18 0xf1c
rem        run_ghidra_probe.cmd dump 0x18008daf0 0x180092a50
rem        run_ghidra_probe.cmd disasm 0x1800dc0b0:220
rem First run imports and analyses (~4 min); later runs reuse the saved project.
set "GHIDRA=D:\tools\ghidra_12.1.4_PUBLIC"
set "PROJ=D:\tmp_ghidra_g6probe"
set "SCRIPT=%~dp0ghidra_scripts"
set "OUT=%~dp0g6probe.txt"
if not exist "%PROJ%" mkdir "%PROJ%"
if exist "%PROJ%\G6Probe.gpr" goto process
"%GHIDRA%\support\analyzeHeadless.bat" "%PROJ%" G6Probe -import "C:\Windows\System32\KSUSBAPO64.dll" -scriptPath "%SCRIPT%" -postScript G6Probe.java %* -analysisTimeoutPerFile 1200 > "%OUT%" 2>&1
goto done
:process
"%GHIDRA%\support\analyzeHeadless.bat" "%PROJ%" G6Probe -process KSUSBAPO64.dll -noanalysis -scriptPath "%SCRIPT%" -postScript G6Probe.java %* > "%OUT%" 2>&1
:done
exit /b %errorlevel%
