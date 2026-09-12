# Release checklist

Provenance: `.github/workflows`, installers, and local checks reviewed on
2026-09-12.

1. Keep `daemon/Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and the tag version aligned.
2. Run `cargo fmt --check`, Clippy with warnings denied, and tests.
3. Check the Windows target and type-check the macOS menu client.
4. Run `scripts/check-extension.sh` on Linux with `glib-compile-schemas` available.
5. Reinstall on the current macOS device and inspect at least two live protocol samples.
6. Push through a pull request; tag only the merged commit.
7. The tag workflow must produce Linux, GNOME, macOS, and Windows archives before publishing.

GitHub Actions are pinned by full commit SHA. Re-check upstream release SHAs
when changing an action rather than replacing pins with movable tags.
