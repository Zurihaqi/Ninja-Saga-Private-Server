param([switch]$IncludeRealtime)
$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
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
$env:DATABASE_URL='mysql://'+$dbState.username+':'+$dbState.password+'@127.0.0.1:3307/ninja_rust_test'
$env:GAME_DATA_ROOT=Join-Path $taskRoot 'reference/upstream/Database/Laragon/ninjasage/public/game_data'
# The patched client copy (.tools/client, produced by scripts/prepare-local-client.py)
# has loopback URL constants baked in; the upstream reference copy points at the
# live CDN and must not be served locally.
$env:CLIENT_ASSET_ROOT=Join-Path $taskRoot '.tools/client'
# Built React client (apps/web-client, served by the game server at /app).
$env:WEB_CLIENT_ROOT=Join-Path $taskRoot 'apps/web-client/dist'
$env:APP_URL='http://127.0.0.1:8080'
$env:RUST_LOG='info'
# Local-only administration has a unique persisted secret, never a built-in password.
if ([string]::IsNullOrWhiteSpace($env:ADMIN_SECRET)) {
    $taskSecretFile=Join-Path $taskRoot '.tools/admin-secret.txt'
    if (-not (Test-Path -LiteralPath $taskSecretFile)) {
        [IO.File]::WriteAllText($taskSecretFile,[Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32)).ToLowerInvariant())
    }
    $env:ADMIN_SECRET=[IO.File]::ReadAllText($taskSecretFile).Trim()
    if ($env:ADMIN_SECRET.Length -lt 32) { throw 'Local admin secret is empty or too short.' }
}
if ([string]::IsNullOrWhiteSpace($env:CHAT_ADMIN_SECRET)) { $env:CHAT_ADMIN_SECRET=$env:ADMIN_SECRET }

function Start-LocalService([string]$service,[int]$port) {
    $taskExecutable=(Resolve-Path -LiteralPath (Join-Path $taskRoot "target/debug/$service.exe")).Path
    $taskRunning=Get-Process -Name $service -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $taskExecutable }
    if ($taskRunning) { Write-Output "$service is already running; restart it to load rebuilt code or changed configuration."; return }
    $env:BIND_ADDR="127.0.0.1:$port"
    $taskService=Start-Process -FilePath $taskExecutable -WorkingDirectory $taskRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $taskRoot ".tools/$service.log") -RedirectStandardError (Join-Path $taskRoot ".tools/$service.err") -PassThru
    $taskService.Id | Set-Content (Join-Path $taskRoot ".tools/$service.pid")
    $taskReady=$false
    for ($taskAttempt=0; $taskAttempt -lt 40; $taskAttempt++) {
        if ($taskService.HasExited) { throw "$service exited during startup; inspect .tools/$service.err." }
        try {
            $taskHealth=Invoke-WebRequest "http://127.0.0.1:$port/health/live" -TimeoutSec 1
            if ($taskHealth.StatusCode -eq 200) { $taskReady=$true; break }
        } catch {}
        Start-Sleep -Milliseconds 250
    }
    if (-not $taskReady) { throw "$service failed its startup liveness check; inspect .tools/$service.err." }
    Write-Output "Started and verified $service (PID $($taskService.Id)) on port $port."
}
$taskPreviousBind=$env:BIND_ADDR
try {
    Start-LocalService 'game-server' 8080
    if ($IncludeRealtime) {
        $env:POLICY_BIND_ADDR='127.0.0.1:843'
        Start-LocalService 'chat-server' 3002
        Start-LocalService 'pvp-server' 3000
    }
} finally { $env:BIND_ADDR=$taskPreviousBind }
Write-Output 'Game: http://127.0.0.1:8080/app/ | Admin: http://127.0.0.1:8080/admin'
Write-Output 'Use an admin account, or the configured ADMIN_SECRET (local helper default: .tools/admin-secret.txt).'
