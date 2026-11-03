# Headless AMF/MySQL regression test. Creates and removes its own synthetic account.
param([switch]$ReferenceCatalog)
$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$dbState=Get-Content (Join-Path $taskRoot '.tools/database.json') -Raw | ConvertFrom-Json
$env:DATABASE_URL='mysql://'+$dbState.username+':'+$dbState.password+'@127.0.0.1:3307/ninja_rust_test'
$env:GAME_DATA_ROOT=if ($ReferenceCatalog) { Join-Path $taskRoot 'reference/upstream/Database/Laragon/ninjasage/public/game_data' } else { Join-Path $taskRoot 'tests/compatibility/gameplay' }
Push-Location $taskRoot
try {
    cargo test -p game-server --lib persistent_account_character_and_ownership_flow --locked -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Gameplay regression failed' }
} finally { Pop-Location }
