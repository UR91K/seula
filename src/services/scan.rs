//! What `SystemService` reports about scans and the file watcher, in types of its own.
//!
//! These used to be the gRPC-generated messages, which tied the services (and the HTTP
//! layer over them) to `src/grpc/` and to tonic. ADR-0046 removes that server.

/// Where the one running scan is. Project scans, plugin scans and sample checks share
/// it, so one runs at a time (ADR-0038, ADR-0041).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScanStatus {
    /// No scan has run since the daemon started.
    #[default]
    Unknown,
    Starting,
    /// The first-run plugin scan, which runs before projects are discovered.
    ScanningPlugins,
    /// Checking that each sample file is still there, and its size (ADR-0041).
    CheckingSamples,
    Discovering,
    Parsing,
    Inserting,
    Completed,
    Error,
}

impl ScanStatus {
    /// A scan holds the status in any of these, and no other may start.
    pub fn is_running(self) -> bool {
        matches!(
            self,
            ScanStatus::Starting
                | ScanStatus::ScanningPlugins
                | ScanStatus::CheckingSamples
                | ScanStatus::Discovering
                | ScanStatus::Parsing
                | ScanStatus::Inserting
        )
    }

    /// The last update of a scan: it carries the result.
    pub fn is_final(self) -> bool {
        matches!(self, ScanStatus::Completed | ScanStatus::Error)
    }

    /// The name the API reports, and the front end matches on.
    pub fn name(self) -> &'static str {
        match self {
            ScanStatus::Unknown => "unknown",
            ScanStatus::Starting => "starting",
            ScanStatus::ScanningPlugins => "scanning_plugins",
            ScanStatus::CheckingSamples => "checking_samples",
            ScanStatus::Discovering => "discovering",
            ScanStatus::Parsing => "parsing",
            ScanStatus::Inserting => "inserting",
            ScanStatus::Completed => "completed",
            ScanStatus::Error => "error",
        }
    }
}

/// One progress update from a scan.
#[derive(Clone, Debug, PartialEq)]
pub struct ScanProgress {
    pub completed: u32,
    pub total: u32,
    /// 0 to 1.
    pub progress: f32,
    pub message: String,
    pub status: ScanStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

impl WatcherEventType {
    pub fn name(self) -> &'static str {
        match self {
            WatcherEventType::Created => "created",
            WatcherEventType::Modified => "modified",
            WatcherEventType::Deleted => "deleted",
            WatcherEventType::Renamed => "renamed",
        }
    }
}

/// A change the file watcher saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherEvent {
    pub event_type: WatcherEventType,
    pub path: String,
    /// Where a rename went.
    pub new_path: Option<String>,
    pub timestamp: i64,
}
