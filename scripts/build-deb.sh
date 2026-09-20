#!/usr/bin/env bash
# rldyour-sysinfo — build the APT package
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Usage: scripts/build-deb.sh <rldyour-sysinfod binary> <output directory>
#
# The package places the daemon in /usr/bin, so the service unit is rendered
# from the repository copy with its ExecStart pointed at that path — the
# source tree's unit keeps pointing at the ~/.local/bin layout install.sh
# produces.
set -euo pipefail

binary="$1"
outdir="$2"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${root}/daemon/Cargo.toml" | head -1)"

stage="$(mktemp -d)"
trap 'rm -rf "${stage}"' EXIT

install -Dm755 "${binary}" "${stage}/usr/bin/rldyour-sysinfod"
install -Dm644 "${root}/daemon/systemd/rldyour-sysinfod.socket" \
    "${stage}/usr/lib/systemd/user/rldyour-sysinfod.socket"
sed 's|^ExecStart=.*|ExecStart=/usr/bin/rldyour-sysinfod|' \
    "${root}/daemon/systemd/rldyour-sysinfod.service" \
    > "${stage}/usr/lib/systemd/user/rldyour-sysinfod.service"
chmod 644 "${stage}/usr/lib/systemd/user/rldyour-sysinfod.service"

install -Dm644 "${root}/packaging/deb/control" "${stage}/DEBIAN/control"
sed -i "/^Package: rldyour-sysinfo$/a Version: ${version}" "${stage}/DEBIAN/control"
install -Dm755 "${root}/packaging/deb/postinst" "${stage}/DEBIAN/postinst"
install -Dm755 "${root}/packaging/deb/prerm" "${stage}/DEBIAN/prerm"
install -Dm755 "${root}/packaging/deb/postrm" "${stage}/DEBIAN/postrm"

install -d "${outdir}"
dpkg-deb --root-owner-group --build "${stage}" \
    "${outdir}/rldyour-sysinfo_${version}_amd64.deb"
