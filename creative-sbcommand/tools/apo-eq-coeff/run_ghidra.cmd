@echo off
set "GHIDRA=D:\tools\ghidra_12.1.4_PUBLIC"
set "PROJ=D:\tmp_ghidra_cryst"
set "SCRIPT=D:\工程\逆向工程\creative-sbcommand\tools\apo-eq-coeff\ghidra_scripts"
set "OUT=D:\工程\逆向工程\creative-sbcommand\tools\apo-eq-coeff\ghidra_dump.txt"
if exist "%PROJ%" rmdir /s /q "%PROJ%"
"%GHIDRA%\support\analyzeHeadless.bat" "%PROJ%" CrystProj -import "C:\Windows\System32\KSUSBAPO64.dll" -scriptPath "%SCRIPT%" -postScript DumpC.java 0x18008e920 0x18008ec00 0x18008dec0 0x18008e030 0x180146190 0x180145f30 0x1801468c0 0x1801469e0 -analysisTimeoutPerFile 600 > "%OUT%" 2>&1
exit /b %errorlevel%
