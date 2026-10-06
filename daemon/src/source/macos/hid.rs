//! Read-only Apple silicon temperature fallback. The event-system client and
//! matching services are retained across ticks, not rebuilt for each reading.
//! All Create/Copy/Retain results have one RAII owner, including error paths.

use super::mean;
use core_foundation_sys::{array::*, base::*, dictionary::*, number::*, string::*};
use std::ffi::CStr;
use std::ptr;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDEventSystemClientCreate(allocator: CFAllocatorRef) -> CFTypeRef;
    fn IOHIDEventSystemClientSetMatching(client: CFTypeRef, matching: CFDictionaryRef);
    fn IOHIDEventSystemClientCopyServices(client: CFTypeRef) -> CFArrayRef;
    fn IOHIDServiceClientCopyProperty(service: CFTypeRef, key: CFStringRef) -> CFTypeRef;
    fn IOHIDServiceClientCopyEvent(
        service: CFTypeRef,
        event_type: i64,
        options: i32,
        timestamp: i64,
    ) -> CFTypeRef;
    fn IOHIDEventGetFloatValue(event: CFTypeRef, field: i32) -> f64;
}

struct Owned(CFTypeRef);

impl Owned {
    // SAFETY: only +1 references from Create/Copy/Retain enter this wrapper.
    unsafe fn take(value: CFTypeRef) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }
    fn string(value: &CStr) -> Option<Self> {
        unsafe {
            Self::take(
                CFStringCreateWithCString(ptr::null(), value.as_ptr(), kCFStringEncodingUTF8)
                    .cast(),
            )
        }
    }
    fn integer(value: i32) -> Option<Self> {
        unsafe {
            Self::take(
                CFNumberCreate(
                    ptr::null(),
                    kCFNumberSInt32Type,
                    (&value as *const i32).cast(),
                )
                .cast(),
            )
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

pub struct Hid {
    _system: Owned,
    cpu: Vec<Owned>,
    gpu: Vec<Owned>,
}

impl Hid {
    pub fn new() -> Option<Self> {
        let page_key = Owned::string(c"PrimaryUsagePage")?;
        let usage_key = Owned::string(c"PrimaryUsage")?;
        let product_key = Owned::string(c"Product")?;
        let page = Owned::integer(0xff00)?;
        let usage = Owned::integer(5)?;
        let keys = [page_key.0, usage_key.0];
        let values = [page.0, usage.0];
        let matching = unsafe {
            Owned::take(
                CFDictionaryCreate(
                    ptr::null(),
                    keys.as_ptr(),
                    values.as_ptr(),
                    2,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                )
                .cast(),
            )?
        };
        let system = unsafe { Owned::take(IOHIDEventSystemClientCreate(ptr::null()))? };
        unsafe { IOHIDEventSystemClientSetMatching(system.0, matching.0.cast()) };
        let services = unsafe { Owned::take(IOHIDEventSystemClientCopyServices(system.0).cast())? };
        let mut cpu = Vec::new();
        let mut gpu = Vec::new();
        for index in 0..unsafe { CFArrayGetCount(services.0.cast()) } {
            let service = unsafe { CFArrayGetValueAtIndex(services.0.cast(), index) };
            let Some(product) = (unsafe {
                Owned::take(IOHIDServiceClientCopyProperty(
                    service,
                    product_key.0.cast(),
                ))
            }) else {
                continue;
            };
            if unsafe { CFGetTypeID(product.0) } != unsafe { CFStringGetTypeID() } {
                continue;
            }
            let mut bytes = [0i8; 128];
            if unsafe {
                CFStringGetCString(
                    product.0.cast(),
                    bytes.as_mut_ptr(),
                    bytes.len() as isize,
                    kCFStringEncodingUTF8,
                )
            } == 0
            {
                continue;
            }
            let name = unsafe { CStr::from_ptr(bytes.as_ptr()) }.to_bytes();
            let target = if name.starts_with(b"GPU MTR Temp") {
                &mut gpu
            } else if name.starts_with(b"pACC MTR Temp") || name.starts_with(b"eACC MTR Temp") {
                &mut cpu
            } else {
                continue;
            };
            target.push(unsafe { Owned::take(CFRetain(service))? });
        }
        Some(Self {
            _system: system,
            cpu,
            gpu,
        })
    }

    pub fn cpu(&self) -> Option<f32> {
        average(&self.cpu)
    }
    pub fn gpu(&self) -> Option<f32> {
        average(&self.gpu)
    }
}

fn average(services: &[Owned]) -> Option<f32> {
    let mut sum = 0.0;
    let mut count = 0;
    for service in services {
        let Some(event) =
            (unsafe { Owned::take(IOHIDServiceClientCopyEvent(service.0, 15, 0, 0)) })
        else {
            continue;
        };
        let value = unsafe { IOHIDEventGetFloatValue(event.0, 15 << 16) };
        if value.is_finite() && (10.0..=120.0).contains(&value) {
            sum += value;
            count += 1;
        }
    }
    mean(sum, count)
}
