#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
AGENT_DIR="${HOME}/Library/LaunchAgents"
APP_DIR="${HOME}/Applications/rldyour-sysinfo.app"

# launchd binds the activation socket at this path, so the directory must
# exist before the agent loads.
install -d "${BIN_DIR}" "${AGENT_DIR}" "${APP_DIR}/Contents/MacOS" \
  "${HOME}/Library/Application Support/rldyour-sysinfo"
if [[ -x "${ROOT}/prebuilt/rldyour-sysinfod" && -d "${ROOT}/prebuilt/rldyour-sysinfo.app" ]]; then
  install -m755 "${ROOT}/prebuilt/rldyour-sysinfod" "${BIN_DIR}/rldyour-sysinfod"
  cp -R "${ROOT}/prebuilt/rldyour-sysinfo.app/." "${APP_DIR}/"
else
  cargo build --release --no-default-features --manifest-path "${ROOT}/daemon/Cargo.toml"
  install -m755 "${ROOT}/daemon/target/release/rldyour-sysinfod" "${BIN_DIR}/rldyour-sysinfod"
  swiftc -O -framework AppKit "${ROOT}/macos/RldyourSysinfo.swift" -o "${APP_DIR}/Contents/MacOS/rldyour-sysinfo"
fi

/usr/libexec/PlistBuddy -c 'Clear dict' "${APP_DIR}/Contents/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string rldyour-sysinfo' "${APP_DIR}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string com.nddev-opennetwork.rldyour-sysinfo' "${APP_DIR}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleName string rldyour-sysinfo' "${APP_DIR}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :LSUIElement bool true' "${APP_DIR}/Contents/Info.plist"
codesign --force --sign - "${APP_DIR}"

sed -e "s|@HOME@|${HOME}|g" "${ROOT}/macos/com.nddev-opennetwork.rldyour-sysinfod.plist" > "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfod.plist"
sed -e "s|@HOME@|${HOME}|g" "${ROOT}/macos/com.nddev-opennetwork.rldyour-sysinfo.plist" > "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfo.plist"
launchctl bootout "gui/$(id -u)/com.nddev-opennetwork.rldyour-sysinfod" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfod.plist"
launchctl bootout "gui/$(id -u)/com.nddev-opennetwork.rldyour-sysinfo" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfo.plist"
open "${APP_DIR}"
