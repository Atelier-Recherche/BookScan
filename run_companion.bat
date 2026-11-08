@echo off
cd /d "%~dp0bookscan-companion"
set "EXE=target\release\bookscan-companion.exe"

if /I "%~1"=="--rebuild" goto rebuild
if not exist "%EXE%" goto rebuild

echo Lancement rapide (exe existant). Pour recompiler: run_companion.bat --rebuild
"%EXE%"
goto :eof

:rebuild
echo Build release (une fois, peut prendre quelques minutes)...
cargo build --release
if errorlevel 1 exit /b 1
"%EXE%"
