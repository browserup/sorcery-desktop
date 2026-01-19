# Sorcery Desktop Windows Release Build
# Run from project root: .\scripts\build-windows-release.ps1

$ErrorActionPreference = "Stop"

Write-Host "==> Sorcery Desktop Windows Release Build" -ForegroundColor Cyan
Write-Host ""

# Check for Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Rust/Cargo not found. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Check for Tauri CLI
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Tauri CLI..."
    cargo install tauri-cli
}

Write-Host "==> Building release..." -ForegroundColor Cyan
Set-Location src-tauri
cargo tauri build

Write-Host ""
Write-Host "==> Build complete!" -ForegroundColor Green
Write-Host ""

# Show output locations
$bundleDir = "..\target\release\bundle"

Write-Host "Build artifacts:"
if (Test-Path "$bundleDir\msi\*.msi") {
    Get-ChildItem "$bundleDir\msi\*.msi" | ForEach-Object { Write-Host "  MSI: $_" }
}
if (Test-Path "$bundleDir\nsis\*.exe") {
    Get-ChildItem "$bundleDir\nsis\*.exe" | ForEach-Object { Write-Host "  NSIS: $_" }
}

Write-Host ""
Write-Host "To install, run the .msi or .exe installer."
