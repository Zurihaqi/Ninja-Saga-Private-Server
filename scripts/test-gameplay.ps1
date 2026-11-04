# Headless AMF/MySQL regression test. Creates and removes its own synthetic account.
param([switch]$ReferenceCatalog)
$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$mysqlUp=$false
try {
    $probe=New-Object System.Net.Sockets.TcpClient('127.0.0.1', 3307)
    $mysqlUp=$probe.Connected
    $probe.Dispose()
} catch {}
if (-not $mysqlUp) {
    $mysqld=Join-Path $taskRoot '.tools/mysql-8.4.7-winx64/bin/mysqld.exe'
    $mysqlIni=Join-Path $taskRoot '.tools/mysql.ini'
    Start-Process -FilePath $mysqld -ArgumentList ('--defaults-file="'+$mysqlIni+'"') -WindowStyle Hidden | Out-Null
    for ($i=0; $i -lt 30; $i++) {
        try {
            $probe=New-Object System.Net.Sockets.TcpClient('127.0.0.1', 3307)
            if ($probe.Connected) { $probe.Dispose(); break }
            $probe.Dispose()
        } catch {}
        Start-Sleep -Milliseconds 250
    }
}
$dbState=Get-Content (Join-Path $taskRoot '.tools/database.json') -Raw | ConvertFrom-Json
$taskDatabase=if ($ReferenceCatalog) {'ninja_ci_reference_test'} else {'ninja_ci_test'}
# Extended regressions change global event/config fixtures as well as synthetic
# player rows. Always use a dedicated schema, never the playable database.
$taskPreviousMysqlPassword=$env:MYSQL_PWD
try {
    $env:MYSQL_PWD=$dbState.root_password
    & (Join-Path $taskRoot '.tools/mysql-8.4.7-winx64/bin/mysql.exe') --host=127.0.0.1 --port=3307 --user=root --execute="CREATE DATABASE IF NOT EXISTS $taskDatabase CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
    if ($LASTEXITCODE -ne 0) { throw 'Unable to prepare isolated test database.' }
} finally { $env:MYSQL_PWD=$taskPreviousMysqlPassword }
$env:DATABASE_URL='mysql://root:'+$dbState.root_password+'@127.0.0.1:3307/'+$taskDatabase
$env:BOOTSTRAP_GAMEPLAY_TEST_DB='1'
$env:GAME_DATA_ROOT=if ($ReferenceCatalog) { Join-Path $taskRoot 'reference/upstream/Database/Laragon/ninjasage/public/game_data' } else { Join-Path $taskRoot 'tests/compatibility/gameplay' }
Push-Location $taskRoot
try {
    if ($ReferenceCatalog) {
        cargo run -p game-server --bin import-reference --locked -- .tools/reference-catalogs.json
        if ($LASTEXITCODE -ne 0) { throw 'Reference catalog import into isolated schema failed.' }
    }
    cargo test -p game-server --lib persistent_account_character_and_ownership_flow --locked -- --ignored --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Gameplay regression failed' }
} finally { Pop-Location }
