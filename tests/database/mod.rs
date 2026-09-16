//! Database tests
//!
//! This module contains all database-related tests

pub mod batch;
pub mod collections;
pub mod core;
pub mod media;
pub mod plugin_scan;
pub mod search;
pub mod tags;

// Common imports for database tests
use seula::database::ProjectDatabase;
use seula::project::Project;
// use crate::common::setup;
use std::path::PathBuf;
// use tempfile::tempdir;
