//! Hand-written serde wire types for the HTTP adapter (ADR-0024), one module per
//! domain, independent of the generated proto types.

pub mod collections;
pub mod projects;
pub mod search;
pub mod tags;
pub mod tasks;
