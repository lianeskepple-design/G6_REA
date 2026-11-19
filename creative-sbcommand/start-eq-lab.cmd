@echo off
title Creative EQ audio lab
"%~dp0tools\audio-env\Scripts\python.exe" -X utf8 "%~dp0tools\serve-eq-lab.py"
if errorlevel 1 (
  echo Failed to start the local EQ page. Check the Python environment and port.
  pause
)
