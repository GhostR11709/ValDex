@echo off
cd /d "%~dp0"
title Vivi
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\play.ps1"
