#!/usr/bin/env bash
# rldyour-sysinfo installer
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Builds the daemon, installs it as a socket-activated user service, and links
# the extension into the shell's search path. Everything lands under $HOME; no
# step needs root.
set -euo pipefail

UUID="rldyour-sysinfo@nddev-opennetwork"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
UNIT_DIR="${HOME}/.config/systemd/user"
EXT_DIR="${HOME}/.local/share/gnome-shell/extensions/${UUID}"

say() { printf '\033[1m==>\033[0m %s\n' "$1"; }

say "Building the daemon"
cargo build --release --manifest-path "${ROOT}/daemon/Cargo.toml"

say "Installing the daemon into ${BIN_DIR}"
install -Dm755 "${ROOT}/daemon/target/release/rldyour-sysinfod" "${BIN_DIR}/rldyour-sysinfod"

say "Installing the user units into ${UNIT_DIR}"
install -Dm644 "${ROOT}/daemon/systemd/rldyour-sysinfod.socket" "${UNIT_DIR}/rldyour-sysinfod.socket"
install -Dm644 "${ROOT}/daemon/systemd/rldyour-sysinfod.service" "${UNIT_DIR}/rldyour-sysinfod.service"

say "Enabling the socket"
systemctl --user daemon-reload
# The socket carries the activation; the service starts on the first connection
# and stops again once the last client goes away.
systemctl --user enable --now rldyour-sysinfod.socket

say "Installing the extension into ${EXT_DIR}"
rm -rf "${EXT_DIR}"
mkdir -p "${EXT_DIR}"
cp -r "${ROOT}/extension/." "${EXT_DIR}/"

say "Done"
cat <<'NOTE'

The daemon is live now. The extension needs a new shell process, and Wayland
cannot restart one in place, so log out and back in, then run:

    gnome-extensions enable rldyour-sysinfo@nddev-opennetwork

NOTE
