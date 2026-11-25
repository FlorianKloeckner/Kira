# Compiles ..\Kira (unmodified) to WebAssembly and copies it into site\.
# One-time setup:  rustup target add wasm32-wasip1
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot 'engine')
try {
    cargo build --release --target wasm32-wasip1
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
} finally {
    Pop-Location
}
Copy-Item (Join-Path $PSScriptRoot 'engine\target\wasm32-wasip1\release\kira.wasm') (Join-Path $PSScriptRoot 'site\kira.wasm') -Force
Write-Host 'Built site\kira.wasm'
