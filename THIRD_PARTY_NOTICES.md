# Third-party notices

The macOS GPU and HID sensor readers use the same documented IOKit registry
properties and IOHID event-system access pattern as the open-source Stats
project:

- Stats, copyright © Serhiy Mytrovtsiy, MIT License,
  <https://github.com/exelban/stats>
- MenuMeters, whose Apple Silicon hardware-reader approach is credited by
  Stats, GPL-2.0 License, <https://github.com/yujitach/MenuMeters>

No Stats binary, helper, bundle, or source file is distributed with
`rldyour-sysinfo`. Runtime Rust dependencies and their exact versions are
recorded in `daemon/Cargo.lock`; their license metadata remains available in
the corresponding crates.
