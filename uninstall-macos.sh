#!/usr/bin/env bash
set -euo pipefail

USER_DOMAIN="gui/$(id -u)"
launchctl bootout "${USER_DOMAIN}/com.nddev-opennetwork.rldyour-sysinfo" 2>/dev/null || true
launchctl bootout "${USER_DOMAIN}/com.nddev-opennetwork.rldyour-sysinfod" 2>/dev/null || true
rm -f "${HOME}/Library/LaunchAgents/com.nddev-opennetwork.rldyour-sysinfo.plist"
rm -f "${HOME}/Library/LaunchAgents/com.nddev-opennetwork.rldyour-sysinfod.plist"
rm -f "${HOME}/.local/bin/rldyour-sysinfod"
rm -f "${HOME}/Library/Caches/rldyour-sysinfo/rldyour-sysinfo.sock"
rm -rf "${HOME}/Applications/rldyour-sysinfo.app"
rmdir "${HOME}/Library/Caches/rldyour-sysinfo" 2>/dev/null || true
printf 'Removed rldyour-sysinfo from this macOS account.\n'
