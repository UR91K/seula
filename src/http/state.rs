//! Shared state for the HTTP router (ADR-0024).
//!
//! `AppState` bundles `Services` with `SystemService` because `SystemService` is
//! not a field of the `Services` aggregator -- it is constructed separately in
//! `src/grpc/server.rs::SeulaServer::new()` and reached only through
//! `SystemHandler` there. See ADR-0024's Consequences section: folding
//! `SystemService` into `Services` is a service-layer change, not an HTTP one, and
//! is deliberately out of scope here.

use crate::services::{Services, SystemService};

#[derive(Clone)]
pub struct AppState {
    pub services: Services,
    pub system: SystemService,
}

impl AppState {
    pub fn new(services: Services, system: SystemService) -> Self {
        Self { services, system }
    }
}
