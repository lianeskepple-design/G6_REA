@echo off
"C:/Program Files/dotnet/dotnet.exe" --roll-forward Major "%~dp0ilspycmd/tools/net8.0/any/ilspycmd.dll" --disable-updatecheck %*
exit /b %errorlevel%
