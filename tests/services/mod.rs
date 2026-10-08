//! Service-layer tests, one module per domain. See `tests/services_tests.rs`.

pub mod batch;
pub mod collections;
pub mod media;
pub mod plugins;
pub mod projects;
pub mod samples;
pub mod search;
pub mod system;
pub mod tags;
pub mod tasks;

pub use crate::common::fixtures::*;
