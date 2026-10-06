#!/usr/bin/env bash
# Exercise the actual Debian helper in an isolated package root, never the
# host's system/user services. Fresh installs enable; disabled upgrades stay off.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -r "${fixture}"' EXIT
install -Dm644 "${root}/daemon/systemd/rldyour-sysinfod.socket" \
  "${fixture}/usr/lib/systemd/user/rldyour-sysinfod.socket"
export DPKG_ROOT="${fixture}"
export DPKG_MAINTSCRIPT_PACKAGE=rldyour-sysinfo
export DPKG_MAINTSCRIPT_NAME=postinst
bash "${root}/packaging/deb/postinst" configure
link="${fixture}/etc/systemd/user/sockets.target.wants/rldyour-sysinfod.socket"
test -L "${link}"
bash "${root}/packaging/deb/postinst" configure 0.3.1
test -L "${link}"
python3 - "${link}" <<'PY'
import pathlib, sys
pathlib.Path(sys.argv[1]).unlink()
PY
bash "${root}/packaging/deb/postinst" configure 0.3.1
test ! -L "${link}"
printf 'PASS: fresh socket enablement, enabled upgrade, explicit disablement preserved\n'
