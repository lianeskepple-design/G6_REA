@echo off
setlocal
"%WINDIR%\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /nologo /target:exe /platform:x64 /optimize+ /out:"%~dp0process-loopback.exe" "%~dp0Program.cs"
exit /b %errorlevel%
