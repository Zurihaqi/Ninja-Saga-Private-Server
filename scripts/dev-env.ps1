# Dot-source from any directory: . ./scripts/dev-env.ps1
$taskRoot = Split-Path $PSScriptRoot -Parent
$env:RUSTUP_HOME = Join-Path $taskRoot '.tools/rustup'
$env:CARGO_HOME = Join-Path $taskRoot '.tools/cargo'
$env:PATH = (Join-Path $env:CARGO_HOME 'bin') + [IO.Path]::PathSeparator + $env:PATH
$env:PATH = (Join-Path $taskRoot '.tools/w64devkit/bin') + [IO.Path]::PathSeparator + $env:PATH
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = '-C link-self-contained=yes'
$env:CARGO_INCREMENTAL = '0'
