//! Progress reporting shared by CLI and UI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Progress {
    pub written: u64,
    pub total: u64,
    /// Bytes per second measured by the backend; 0 when unknown.
    pub bytes_per_sec: u64,
}

impl Progress {
    /// 0..=100; saturates and never divides by zero.
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        let p = self.written.saturating_mul(100) / self.total;
        u8::try_from(p.min(100)).unwrap_or(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_bounds() {
        assert_eq!(Progress::default().percent(), 0);
        assert_eq!(
            Progress {
                written: 5,
                total: 10,
                bytes_per_sec: 0
            }
            .percent(),
            50
        );
        assert_eq!(
            Progress {
                written: u64::MAX,
                total: 10,
                bytes_per_sec: 0
            }
            .percent(),
            100
        );
    }
}
