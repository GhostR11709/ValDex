@echo off
cd /d "%~dp0.."
npm run dev > "%TEMP%\vivi-dev.log" 2>&1
