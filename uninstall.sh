#!/usr/bin/env bash
# rldyour-sysinfo uninstaller
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

UUID="rldyour-sysinfo@nddev-opennetwork"

systemctl --user disable --now rldyour-sysinfod.socket 2>/dev/null || true
systemctl --user stop rldyour-sysinfod.service 2>/dev/null || true
rm -f "${HOME}/.config/systemd/user/rldyour-sysinfod.socket"
rm -f "${HOME}/.config/systemd/user/rldyour-sysinfod.service"
systemctl --user daemon-reload

rm -f "${HOME}/.local/bin/rldyour-sysinfod"
rm -rf "${HOME}/.local/share/gnome-shell/extensions/${UUID}"

printf 'Removed. The panel indicator disappears at the next login.\n'
