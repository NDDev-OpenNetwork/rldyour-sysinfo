//! The wire format between the daemon and its clients.
//!
//! One newline-terminated JSON object per tick. It is serialised by hand into
//! a buffer the daemon owns for its whole life, so publishing a sample costs
//! no allocation and pulls in no serialisation dependency. A metric the host
//! cannot supply is `null` rather than absent, so a client never has to
//! distinguish "missing" from "unsupported".

use std::fmt::Write;

/// Incremented only on an incompatible change to the object below.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Default)]
pub struct Snapshot {
    pub cpu: Option<f32>,
    pub cpu_temperature: Option<f32>,
    pub memory: Option<f32>,
    pub swap: Option<f32>,
    pub gpu: Option<f32>,
    pub gpu_memory: Option<f32>,
    pub gpu_temperature: Option<f32>,
    pub disk_read: Option<u64>,
    pub disk_write: Option<u64>,
    pub disk_temperature: Option<f32>,
    pub net_rx: Option<u64>,
    pub net_tx: Option<u64>,
}

impl Snapshot {
    /// Renders the snapshot into `out`, replacing any previous contents.
    pub fn encode(&self, out: &mut String) {
        out.clear();
        let _ = write!(out, r#"{{"v":{PROTOCOL_VERSION},"cpu":{{"usage":"#);
        percent(out, self.cpu);
        let _ = write!(out, r#","temp":"#);
        percent(out, self.cpu_temperature);
        let _ = write!(out, r#"}},"memory":{{"used":"#);
        percent(out, self.memory);
        let _ = write!(out, r#","swap":"#);
        percent(out, self.swap);
        let _ = write!(out, r#"}},"gpu":{{"usage":"#);
        percent(out, self.gpu);
        let _ = write!(out, r#","memory":"#);
        percent(out, self.gpu_memory);
        let _ = write!(out, r#","temp":"#);
        percent(out, self.gpu_temperature);
        let _ = write!(out, r#"}},"disk":{{"read":"#);
        counter(out, self.disk_read);
        let _ = write!(out, r#","write":"#);
        counter(out, self.disk_write);
        let _ = write!(out, r#","temp":"#);
        percent(out, self.disk_temperature);
        let _ = write!(out, r#"}},"net":{{"rx":"#);
        counter(out, self.net_rx);
        let _ = write!(out, r#","tx":"#);
        counter(out, self.net_tx);
        let _ = writeln!(out, "}}}}");
    }
}

/// One decimal is the finest resolution a panel label can actually show.
fn percent(out: &mut String, value: Option<f32>) {
    match value {
        Some(value) if value.is_finite() => {
            let _ = write!(out, "{value:.1}");
        }
        _ => out.push_str("null"),
    }
}

fn counter(out: &mut String, value: Option<u64>) {
    match value {
        Some(value) => {
            let _ = write!(out, "{value}");
        }
        None => out.push_str("null"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_every_metric() {
        let snapshot = Snapshot {
            cpu: Some(55.44),
            cpu_temperature: Some(88.6),
            memory: Some(54.7),
            // Exactly representable and exactly halfway, so it pins the
            // rounding rule: Rust formats ties to even, giving 4.2.
            swap: Some(4.25),
            gpu: Some(7.0),
            gpu_memory: Some(9.6),
            gpu_temperature: Some(69.0),
            disk_read: Some(10628),
            disk_write: Some(3789480),
            disk_temperature: Some(47.8),
            net_rx: Some(18360),
            net_tx: Some(342578),
        };

        let mut out = String::new();
        snapshot.encode(&mut out);

        assert_eq!(
            out,
            concat!(
                r#"{"v":1,"cpu":{"usage":55.4,"temp":88.6},"#,
                r#""memory":{"used":54.7,"swap":4.2},"#,
                r#""gpu":{"usage":7.0,"memory":9.6,"temp":69.0},"#,
                r#""disk":{"read":10628,"write":3789480,"temp":47.8},"#,
                r#""net":{"rx":18360,"tx":342578}}"#,
                "\n"
            )
        );
    }

    #[test]
    fn absent_metrics_are_null_not_missing() {
        let mut out = String::new();
        Snapshot::default().encode(&mut out);

        assert!(out.starts_with(r#"{"v":1,"cpu":{"usage":null,"temp":null}"#));
        assert!(out.contains(r#""gpu":{"usage":null,"memory":null,"temp":null}"#));
        assert!(out.ends_with("}\n"));
        // A client must never have to tell "unsupported" from "absent key".
        assert_eq!(out.matches("null").count(), 12);
    }

    #[test]
    fn a_non_finite_reading_is_reported_as_absent() {
        let mut out = String::new();
        Snapshot {
            cpu: Some(f32::NAN),
            ..Snapshot::default()
        }
        .encode(&mut out);
        assert!(out.contains(r#""usage":null"#));
    }

    #[test]
    fn encoding_reuses_the_buffer() {
        let mut out = String::from("stale contents");
        Snapshot::default().encode(&mut out);
        assert!(!out.contains("stale"));
    }
}
