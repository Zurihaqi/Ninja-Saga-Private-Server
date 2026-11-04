# Explicit additive migrations for the configured playable development database.
$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$taskDb=Get-Content (Join-Path $taskRoot '.tools/database.json') -Raw | ConvertFrom-Json
$env:DATABASE_URL='mysql://'+$taskDb.username+':'+$taskDb.password+'@127.0.0.1:3307/ninja_rust_test'
Push-Location $taskRoot
try {
    cargo run -p game-server --bin migrate --locked
    if ($LASTEXITCODE -ne 0) { throw 'Local additive migration failed.' }
} finally { Pop-Location }
