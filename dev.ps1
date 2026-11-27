#!/usr/bin/env pwsh
# caper dev server - Windows
#
#   .\dev.ps1                 build if needed, serve on the port in rustex1.json
#   .\dev.ps1 4000            override the port
#   .\dev.ps1 --rebuild       force a fresh wasm build on start
#
# The harness runs inside caper-server: one process, no second daemon.

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'caper: cargo not found - install Rust first (https://rustup.rs).' -ForegroundColor Red
    exit 1
}

if (-not (Test-Path 'vendor\agl-rs\Cargo.toml')) {
    Write-Host 'caper: vendor\agl-rs is missing - caper-server cannot build.' -ForegroundColor Red
    Write-Host '        put the Rust agl crate there (see vendor\README.md) and re-run.'
    exit 1
}

# The wasm half is built by the dev server on the first request; it needs the
# target and a wasm-bindgen CLI whose version matches the kit's pin exactly.
$installed = & rustup target list --installed
if ($installed -notcontains 'wasm32-unknown-unknown') {
    Write-Host 'caper: adding the wasm32-unknown-unknown target'
    & rustup target add wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$pin = (Select-String -Path 'vendor\rustex1\kit\Cargo.toml' -Pattern '^wasm-bindgen = "=([0-9.]+)"' | Select-Object -First 1).Matches.Groups[1].Value
if (-not $pin) { $pin = '0.2.129' }

$cli = $env:RUSTEX1_WASM_BINDGEN
if (-not $cli -or -not (Test-Path $cli)) {
    $cmd = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
    if ($cmd) { $cli = $cmd.Source }
}
if (-not $cli -or -not (Test-Path $cli)) {
    $candidate = Join-Path $env:USERPROFILE '.cargo\bin\wasm-bindgen.exe'
    if (Test-Path $candidate) {
        $cli = $candidate
    } else {
        Write-Host "caper: installing wasm-bindgen $pin (a few minutes)"
        & cargo install wasm-bindgen-cli --version $pin
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $cli = $candidate
    }
}
if (-not (Test-Path $cli)) {
    Write-Host "caper: wasm-bindgen.exe not found at $cli" -ForegroundColor Red
    exit 1
}
$env:RUSTEX1_WASM_BINDGEN = $cli

# Honour an exported key first; read MUSE_API_KEY from .caper\.env without
# sourcing arbitrary commands from it.
if (-not $env:MUSE_API_KEY -and -not $env:META_API_KEY) {
    $envFile = Join-Path (Get-Location) '.caper\.env'
    if (-not (Test-Path $envFile)) {
        $legacyEnvFile = Join-Path (Get-Location) '.angela\.env'
        if (Test-Path $legacyEnvFile) { $envFile = $legacyEnvFile }
    }
    if (Test-Path $envFile) {
        foreach ($line in Get-Content $envFile) {
            if ($line -match '^\s*(?:export\s+)?MUSE_API_KEY\s*=\s*(.*)$') {
                $value = $Matches[1].Trim()
                if ($value.Length -ge 2 -and (($value[0] -eq '"' -and $value[-1] -eq '"') -or ($value[0] -eq "'" -and $value[-1] -eq "'"))) {
                    $value = $value.Substring(1, $value.Length - 2)
                }
                $env:MUSE_API_KEY = $value
                break
            }
        }
    }
}

# blaze if it is here, cargo if not
$build = $env:CAPER_BUILD
if (-not $build) {
    if (Get-Command blaze -ErrorAction SilentlyContinue) { $build = 'blaze' } else { $build = 'cargo' }
}
& $build build -p caper-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& '.\target\debug\caper-server.exe' @args
exit $LASTEXITCODE
