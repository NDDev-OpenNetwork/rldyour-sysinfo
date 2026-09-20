//! Delegates snapshot collection to the native implementation for this OS.

use crate::proto::Snapshot;
use crate::source::{MetricsSource, PlatformCollector};
use std::io;

pub struct Collector(PlatformCollector);

impl Collector {
    pub fn new() -> io::Result<Self> {
        MetricsSource::new().map(Self)
    }

    pub fn sample(&mut self) -> Snapshot {
        MetricsSource::sample(&mut self.0)
    }
}
