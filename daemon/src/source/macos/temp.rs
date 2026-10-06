//! macOS temperatures from AppleSMC keys, classified once at startup.
//!
//! SMC key names differ between Intel and Apple silicon generations, so the
//! reader enumerates every temperature key the machine reports once and sorts
//! them by prefix: `Tp*`/`Te*`/`Tf0*`/`TC*` are processor, `Tg*`/`Tf1*`/`Tf2*`/
//! `TGDD` are graphics, `TH*` is storage. A machine that reports none simply
//! yields `null`.

use super::hid::Hid;
use super::mean;
use four_char_code::FourCharCode;
use std::time::{Duration, Instant};

pub struct Temperatures {
    smc: Option<smc::SMC>,
    cpu_keys: Vec<FourCharCode>,
    gpu_keys: Vec<FourCharCode>,
    disk_keys: Vec<FourCharCode>,
    hid: Option<Hid>,
    hid_discovered: Option<Instant>,
}

impl Temperatures {
    pub fn new() -> Self {
        let smc = smc::SMC::new().ok();
        let (mut cpu_keys, mut gpu_keys, mut disk_keys) = smc
            .as_ref()
            .and_then(|smc| smc.keys().ok())
            .map(classify_keys)
            .unwrap_or_default();
        // Reject unsupported key types once. read_key below then needs one
        // metadata lookup and one value read, versus temperature's two lookups
        // and its per-call key-name allocation.
        if let Some(smc) = &smc {
            for keys in [&mut cpu_keys, &mut gpu_keys, &mut disk_keys] {
                keys.retain(|key| smc.temperature(*key).is_ok());
            }
        }
        Self {
            smc,
            cpu_keys,
            gpu_keys,
            disk_keys,
            hid: None,
            hid_discovered: None,
        }
    }

    pub fn cpu(&mut self) -> Option<f32> {
        self.average(&self.cpu_keys).or_else(|| self.hid()?.cpu())
    }

    pub fn gpu(&self) -> Option<f32> {
        self.average(&self.gpu_keys)
    }

    pub fn disk(&self) -> Option<f32> {
        self.average(&self.disk_keys)
    }

    fn average(&self, keys: &[FourCharCode]) -> Option<f32> {
        let smc = self.smc.as_ref()?;
        let (sum, count) = keys
            .iter()
            .filter_map(|key| smc.read_key::<f64>(*key).ok())
            .filter(|value| (10.0..=120.0).contains(value))
            .fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));
        mean(sum, count)
    }

    pub fn gpu_hid(&mut self) -> Option<f32> {
        self.hid()?.gpu()
    }

    fn hid(&mut self) -> Option<&Hid> {
        // Refresh discovery occasionally to recover after wake or permission
        // changes; an unavailable provider is not retried on every tick.
        if self
            .hid_discovered
            .is_none_or(|then| then.elapsed() >= Duration::from_secs(30))
        {
            self.hid = Hid::new();
            self.hid_discovered = Some(Instant::now());
        }
        self.hid.as_ref()
    }
}

fn classify_keys(
    keys: Vec<FourCharCode>,
) -> (Vec<FourCharCode>, Vec<FourCharCode>, Vec<FourCharCode>) {
    let mut cpu = Vec::new();
    let mut gpu = Vec::new();
    let mut disk = Vec::new();
    for key in keys {
        let name = key.to_string();
        if name.starts_with("Tp")
            || name.starts_with("Te")
            || name.starts_with("Tf0")
            || name.starts_with("TC")
        {
            cpu.push(key);
        } else if name.starts_with("Tg")
            || name.starts_with("Tf1")
            || name.starts_with("Tf2")
            || name == "TGDD"
        {
            gpu.push(key);
        } else if name.starts_with("TH") {
            disk.push(key);
        }
    }
    (cpu, gpu, disk)
}
