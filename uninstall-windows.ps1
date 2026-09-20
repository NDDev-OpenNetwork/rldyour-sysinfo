$ErrorActionPreference = "Stop"

$BinDir = Join-Path $env:LOCALAPPDATA "rldyour-sysinfo"
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$LegacyShim = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup\rldyour-sysinfod.cmd"
Get-Process rldyour-sysinfod -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-ItemProperty -Path $RunKey -Name "rldyour-sysinfod" -ErrorAction SilentlyContinue
Remove-Item -Force $LegacyShim -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $BinDir -ErrorAction SilentlyContinue
Write-Host "Removed rldyour-sysinfod from the current Windows user."
