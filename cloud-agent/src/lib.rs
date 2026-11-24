//! Pure status reducer. Not an uploader and not a durable queue.
//! Only a future rclone adapter may provide verified telemetry.
#[derive(Debug, PartialEq, Eq)]
pub enum CloudStatus {
    Unknown,
    Offline,
    Uploading,
    Pending,
    Error,
    Synced,
}

#[derive(Default)]
pub struct Telemetry {
    pub connected: bool,
    pub pending: u64,
    pub active: u64,
    pub error: bool,
    /// All required telemetry must have been read successfully and be fresh.
    pub verified: bool,
}

impl Telemetry {
    pub fn status(&self) -> CloudStatus {
        if !self.verified {
            CloudStatus::Unknown
        } else if self.error {
            CloudStatus::Error
        } else if !self.connected {
            CloudStatus::Offline
        } else if self.active > 0 {
            CloudStatus::Uploading
        } else if self.pending > 0 {
            CloudStatus::Pending
        } else {
            CloudStatus::Synced
        }
    }
    pub fn may_finish_upload_phase(&self) -> bool {
        self.status() == CloudStatus::Synced
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_telemetry_never_means_saved() {
        assert_eq!(Telemetry::default().status(), CloudStatus::Unknown);
        assert!(!Telemetry::default().may_finish_upload_phase());
    }
    #[test]
    fn offline_or_unfinished_uploads_block_upload_completion() {
        for t in [
            Telemetry {
                verified: true,
                ..Default::default()
            },
            Telemetry {
                verified: true,
                connected: true,
                pending: 1,
                ..Default::default()
            },
            Telemetry {
                verified: true,
                connected: true,
                active: 1,
                ..Default::default()
            },
            Telemetry {
                verified: true,
                connected: true,
                error: true,
                ..Default::default()
            },
        ] {
            assert!(!t.may_finish_upload_phase());
        }
    }
    #[test]
    fn only_verified_idle_connection_is_synced() {
        let t = Telemetry {
            verified: true,
            connected: true,
            ..Default::default()
        };
        assert_eq!(t.status(), CloudStatus::Synced);
        assert!(t.may_finish_upload_phase());
    }
}
