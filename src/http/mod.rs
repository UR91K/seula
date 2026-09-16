//! HTTP router adapter over the service layer (ADR-0024). A third caller of
//! `src/services/`, alongside gRPC and the CLI, built to the same "thin handler"
//! rule: parse the request, call one `Services` method, convert the result.
//!
//! Tags is the first domain implemented, per ADR-0024's incremental build order:
//! skeleton and health check first, then tags end-to-end as the pattern proof,
//! then the remaining CRUD domains, then streaming last.

pub mod dto;
pub mod error;
pub mod handlers;
pub mod server;
pub mod state;
