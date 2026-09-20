//! macOS physical interface throughput from BSD interface counters.

use crate::source::{delta, rate};
use libc::{c_char, c_int};
use std::ffi::CStr;
use std::io;
use std::ptr;

pub struct Network {
    previous: Option<Sample>,
}

#[derive(Clone, Copy)]
struct Sample {
    received: u64,
    transmitted: u64,
}

pub struct Throughput {
    pub rx: u64,
    pub tx: u64,
}

impl Network {
    pub fn new() -> Self {
        Self { previous: None }
    }

    /// Bytes received and transmitted per second across physical interfaces.
    ///
    /// `en*` covers Ethernet, Wi-Fi and USB tethering; loopback, `utun`
    /// tunnels and bridges never carry an `en` name, so they stay out.
    pub fn throughput(&mut self, seconds: f64) -> io::Result<Option<Throughput>> {
        let current = read_counters()?;
        let previous = self.previous.replace(current);

        Ok(previous.map(|previous| Throughput {
            rx: rate(delta(current.received, previous.received), seconds),
            tx: rate(delta(current.transmitted, previous.transmitted), seconds),
        }))
    }
}

fn read_counters() -> io::Result<Sample> {
    let mut addresses: *mut libc::ifaddrs = ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut addresses) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut sample = Sample {
        received: 0,
        transmitted: 0,
    };
    let mut current = addresses;
    while !current.is_null() {
        let item = unsafe { &*current };
        if !item.ifa_addr.is_null()
            && unsafe { (*item.ifa_addr).sa_family as c_int } == libc::AF_LINK
            && !item.ifa_data.is_null()
        {
            let name = unsafe { CStr::from_ptr(item.ifa_name as *const c_char) };
            let flags = item.ifa_flags as c_int;
            // Byte compare: interface names are ASCII, and the filter runs per
            // interface per tick — no reason to allocate a String for it.
            if name.to_bytes().starts_with(b"en") && flags & libc::IFF_UP != 0 {
                let data = unsafe { &*(item.ifa_data as *const libc::if_data) };
                sample.received = sample.received.saturating_add(data.ifi_ibytes as u64);
                sample.transmitted = sample.transmitted.saturating_add(data.ifi_obytes as u64);
            }
        }
        current = item.ifa_next;
    }
    unsafe { libc::freeifaddrs(addresses) };
    Ok(sample)
}
