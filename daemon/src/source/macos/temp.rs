//! macOS temperatures from AppleSMC keys, classified once at startup.
//!
//! SMC key names differ between Intel and Apple silicon generations, so the
//! reader enumerates every temperature key the machine reports once and sorts
//! them by prefix: `Tp*`/`Te*`/`Tf0*`/`TC*` are processor, `Tg*`/`Tf1*`/`Tf2*`/
//! `TGDD` are graphics, `TH*` is storage. A machine that reports none simply
//! yields `null`.

use super::mean;
use four_char_code::FourCharCode;

unsafe extern "C" {
    fn rldyour_gpu_hid_temperature() -> f64;
}

pub struct Temperatures {
    smc: Option<smc::SMC>,
    cpu_keys: Vec<FourCharCode>,
    gpu_keys: Vec<FourCharCode>,
    disk_keys: Vec<FourCharCode>,
}

impl Temperatures {
    pub fn new() -> Self {
        let smc = smc::SMC::new().ok();
        let (cpu_keys, gpu_keys, disk_keys) = smc
            .as_ref()
            .and_then(|smc| smc.keys().ok())
            .map(classify_keys)
            .unwrap_or_default();
        Self {
            smc,
            cpu_keys,
            gpu_keys,
            disk_keys,
        }
    }

    pub fn cpu(&self) -> Option<f32> {
        self.average(&self.cpu_keys)
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
            .filter_map(|key| smc.temperature(*key).ok())
            .filter(|value| (10.0..=120.0).contains(value))
            .fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));
        mean(sum, count)
    }
}

/// The IOHID GPU temperature sensor Apple silicon exposes when neither SMC
/// nor the accelerator answer.
pub fn gpu_hid_fallback() -> Option<f32> {
    let value = unsafe { rldyour_gpu_hid_temperature() };
    (value.is_finite() && (10.0..=120.0).contains(&value)).then_some(value as f32)
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
