$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$BinDir = Join-Path $env:LOCALAPPDATA "rldyour-sysinfo"
$Binary = Join-Path $BinDir "rldyour-sysinfod.exe"
$Startup = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\rldyour-sysinfod.cmd"

$Prebuilt = Join-Path $Root "rldyour-sysinfod.exe"
if (Test-Path $Prebuilt) {
    $SourceBinary = $Prebuilt
} else {
    cargo build --release --manifest-path (Join-Path $Root "daemon\Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    $SourceBinary = Join-Path $Root "daemon\target\release\rldyour-sysinfod.exe"
}
New-Item -ItemType Directory -Force $BinDir | Out-Null
Copy-Item -Force $SourceBinary $Binary
Set-Content -Encoding Ascii $Startup "@start `"`" /b `"$Binary`""
Start-Process -WindowStyle Hidden $Binary
Write-Host "Installed rldyour-sysinfod for the current Windows user."
