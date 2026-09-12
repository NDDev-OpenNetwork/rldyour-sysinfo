$ErrorActionPreference = "Stop"

$BinDir = Join-Path $env:LOCALAPPDATA "rldyour-sysinfo"
$Startup = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\rldyour-sysinfod.cmd"
Get-Process rldyour-sysinfod -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item -Force $Startup -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $BinDir -ErrorAction SilentlyContinue
Write-Host "Removed rldyour-sysinfod from the current Windows user."
