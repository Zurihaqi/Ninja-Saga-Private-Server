$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
$dbState=Get-Content (Join-Path $taskRoot '.tools/database.json') -Raw | ConvertFrom-Json
$env:DATABASE_URL='mysql://'+$dbState.username+':'+$dbState.password+'@127.0.0.1:3307/ninja_rust_test'
$env:GAME_DATA_ROOT=Join-Path $taskRoot 'reference/upstream/Database/Laragon/ninjasage/public/game_data'
$env:CLIENT_ASSET_ROOT=Join-Path $taskRoot 'reference/upstream/Custom Client/NS Custom Client V1'
$env:APP_URL='http://127.0.0.1:8080'
$env:RUST_LOG='info'
$server=Start-Process -FilePath (Join-Path $taskRoot 'target/debug/game-server.exe') -WorkingDirectory $taskRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $taskRoot '.tools/game-server.log') -RedirectStandardError (Join-Path $taskRoot '.tools/game-server.err') -PassThru
$server.Id | Set-Content (Join-Path $taskRoot '.tools/game-server.pid')
Write-Output "Started local Rust server (PID $($server.Id)); logs in .tools/game-server.log"
