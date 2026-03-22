@echo off
REM Manual build for Windows without wasm-pack

echo Building WASM module...
cd /d "%~dp0"

REM Add wasm32 target if not already added
rustup target add wasm32-unknown-unknown

REM Build the WASM binary
cargo build --target wasm32-unknown-unknown --release

if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    exit /b 1
)

REM Create output directory
mkdir www\pkg 2>nul

REM Copy the WASM file
copy target\wasm32-unknown-unknown\release\dcl_wasm.wasm www\pkg\

echo.
echo Build complete! However, note that this is a basic WASM build.
echo For full functionality, install wasm-pack:
echo   cargo install wasm-pack
echo   wasm-pack build --target web --out-dir www/pkg
echo.
echo To run the demo:
echo   cd www
echo   python -m http.server 8080
echo   Open http://localhost:8080
