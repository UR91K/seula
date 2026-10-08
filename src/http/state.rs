//! Shared state for the HTTP router (ADR-0024).
//!
//! `AppState` bundles `Services` with `SystemService` because `SystemService` is
//! not a field of the `Services` aggregator -- it is constructed separately in
//! `src/main.rs`. See ADR-0024's Consequences section: folding `SystemService` into
//! `Services` is a service-layer change, not an HTTP one (ADR-0046 does it).

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
