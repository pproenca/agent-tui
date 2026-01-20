@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "BINARY_NAME=agent-tui-win32-x64.exe"
set "BINARY_PATH=%SCRIPT_DIR%%BINARY_NAME%"

if not exist "%BINARY_PATH%" (
    echo Binary not found: %BINARY_PATH% >&2
    echo Please run 'npm install' to download the binary for your platform. >&2
    echo Or install via: cargo install agent-tui >&2
    exit /b 1
)

"%BINARY_PATH%" %*
