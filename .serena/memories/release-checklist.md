# Release checklist

Provenance: `.github/workflows`, installers, and local checks reviewed on
2026-10-08. Main is release 0.3.1.

1. Keep `daemon/Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and the tag version aligned.
2. Run `cargo fmt --check`, Clippy with warnings denied, and tests.
3. Check the Windows target; CI compiles the macOS menu client with
   `swiftc -O -framework AppKit` (package-internal APIs like `clamped(to:)`
   only fail there, not in docs).
4. Run `scripts/check-extension.sh` on Linux with `glib-compile-schemas` available.
5. Reinstall on the current macOS device and run the bounded live socket check;
   inspect only aggregate protocol metrics, never process lists or user data.
6. Push through a pull request; tag only the merged commit.
7. Run `cargo publish --dry-run` and build/check both PyPI distributions.
8. The tag workflow must produce Linux, GNOME, macOS, Windows, wheel, and
   source archives before publishing.
9. Verify the signed APT repository with `apt update` and a clean package
   installation before deploying it to GitHub Pages. The 0.3.1 publication
   workflow must run only after the matching stable GitHub release exists.

For the current 0.3.1 qualification, record Rust tests/Clippy, Python tests,
GJS and Swift checks, both installed socket regressions, package lifecycle
checks, and resource measurements. A socket-activated idle exit is expected;
do not treat it as a failed service. Keep final machine state in a metadata
receipt outside the repository rather than adding host paths or logs here.

GitHub Actions are pinned by full commit SHA. Re-check upstream release SHAs
when changing an action rather than replacing pins with movable tags.
