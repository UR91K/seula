//! HTTP router adapter over the service layer (ADR-0024). A third caller of
//! `src/services/`, alongside gRPC and the CLI, built to the same "thin handler"
//! rule: parse the request, call one `Services` method, convert the result.
//!
//! This is the skeleton step of ADR-0024's incremental build order: state, error
//! mapping, and a single `/health` route. Domain routes (tags, projects, ...) are
//! deliberately not implemented yet.

pub mod error;
pub mod server;
pub mod state;
