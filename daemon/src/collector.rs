//! Delegates snapshot collection to the native implementation for this OS.

use crate::proto::Snapshot;
use crate::source::PlatformCollector;
use std::io;

pub struct Collector(PlatformCollector);

impl Collector {
    pub fn new() -> io::Result<Self> {
        PlatformCollector::new().map(Self)
    }

    pub fn sample(&mut self) -> Snapshot {
        self.0.sample()
    }
}
