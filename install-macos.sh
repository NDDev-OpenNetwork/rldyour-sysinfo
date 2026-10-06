#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
AGENT_DIR="${HOME}/Library/LaunchAgents"
APP_DIR="${HOME}/Applications/rldyour-sysinfo.app"
if [[ -f "${ROOT}/VERSION" ]]; then
  VERSION="$(cat "${ROOT}/VERSION")"
else
  VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${ROOT}/daemon/Cargo.toml" | head -1)"
fi
[[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid release version' >&2; exit 1; }
STAGE="$(mktemp -d)"
trap 'rm -rf "${STAGE}"' EXIT
STAGED_APP="${STAGE}/rldyour-sysinfo.app"
install -d "${STAGED_APP}/Contents/MacOS"

# launchd binds the activation socket at this path, so the directory must
# exist before the agent loads.
install -d "${BIN_DIR}" "${AGENT_DIR}" "${APP_DIR}/Contents/MacOS" \
  "${HOME}/Library/Application Support/rldyour-sysinfo"
if [[ -x "${ROOT}/prebuilt/rldyour-sysinfod" && -d "${ROOT}/prebuilt/rldyour-sysinfo.app" ]]; then
  install -m755 "${ROOT}/prebuilt/rldyour-sysinfod" "${STAGE}/rldyour-sysinfod"
  cp -R "${ROOT}/prebuilt/rldyour-sysinfo.app/." "${STAGED_APP}/"
else
  cargo build --release --locked --no-default-features --manifest-path "${ROOT}/daemon/Cargo.toml"
  install -m755 "${ROOT}/daemon/target/release/rldyour-sysinfod" "${STAGE}/rldyour-sysinfod"
  swiftc -target "$(uname -m)-apple-macosx12.0" -swift-version 6 -parse-as-library -warnings-as-errors -O -framework AppKit "${ROOT}"/macos/*.swift -o "${STAGED_APP}/Contents/MacOS/rldyour-sysinfo"
fi

/usr/libexec/PlistBuddy -c 'Clear dict' "${STAGED_APP}/Contents/Info.plist" 2>/dev/null || true
/usr/libexec/PlistBuddy -c 'Add :CFBundleExecutable string rldyour-sysinfo' "${STAGED_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string com.nddev-opennetwork.rldyour-sysinfo' "${STAGED_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleName string rldyour-sysinfo' "${STAGED_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string ${VERSION}" "${STAGED_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string ${VERSION}" "${STAGED_APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :LSUIElement bool true' "${STAGED_APP}/Contents/Info.plist"
codesign --force --sign - "${STAGED_APP}"

# Build and validate the complete replacement before touching a live process.
launchctl bootout "gui/$(id -u)/com.nddev-opennetwork.rldyour-sysinfo" 2>/dev/null || true
launchctl bootout "gui/$(id -u)/com.nddev-opennetwork.rldyour-sysinfod" 2>/dev/null || true
# Previous installers launched through `open`, so bootout did not own the app.
while IFS= read -r pid; do
  [[ -n "${pid}" ]] && kill -TERM "${pid}" 2>/dev/null || true
done < <(pgrep -f "^${APP_DIR}/Contents/MacOS/rldyour-sysinfo$" || true)
install -m755 "${STAGE}/rldyour-sysinfod" "${BIN_DIR}/rldyour-sysinfod.new"
mv -f "${BIN_DIR}/rldyour-sysinfod.new" "${BIN_DIR}/rldyour-sysinfod"
rm -rf "${APP_DIR}"
mv "${STAGED_APP}" "${APP_DIR}"

sed -e "s|@HOME@|${HOME}|g" "${ROOT}/macos/com.nddev-opennetwork.rldyour-sysinfod.plist" > "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfod.plist"
sed -e "s|@HOME@|${HOME}|g" "${ROOT}/macos/com.nddev-opennetwork.rldyour-sysinfo.plist" > "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfo.plist"
launchctl bootstrap "gui/$(id -u)" "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfod.plist"
launchctl bootstrap "gui/$(id -u)" "${AGENT_DIR}/com.nddev-opennetwork.rldyour-sysinfo.plist"
