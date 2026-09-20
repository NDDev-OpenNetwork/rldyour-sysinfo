#!/usr/bin/env bash
set -euo pipefail

USER_DOMAIN="gui/$(id -u)"
launchctl bootout "${USER_DOMAIN}/com.nddev-opennetwork.rldyour-sysinfo" 2>/dev/null || true
launchctl bootout "${USER_DOMAIN}/com.nddev-opennetwork.rldyour-sysinfod" 2>/dev/null || true
rm -f "${HOME}/Library/LaunchAgents/com.nddev-opennetwork.rldyour-sysinfo.plist"
rm -f "${HOME}/Library/LaunchAgents/com.nddev-opennetwork.rldyour-sysinfod.plist"
rm -f "${HOME}/.local/bin/rldyour-sysinfod"
rm -rf "${HOME}/Applications/rldyour-sysinfo.app"
# Current and pre-0.3 socket locations alike.
rm -rf "${HOME}/Library/Application Support/rldyour-sysinfo"
rm -rf "${HOME}/Library/Caches/rldyour-sysinfo"
printf 'Removed rldyour-sysinfo from this macOS account.\n'
