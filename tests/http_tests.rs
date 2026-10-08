//! HTTP adapter tests
//!
//! The behaviour that lives in the adapter rather than the services: status codes, what
//! a response carries beyond the service's rows, and the routes that serve files. Most
//! of what the API does is the services' and is tested in `tests/services/`. These
//! replace what `tests/grpc/` checked only through the gRPC handlers (ADR-0046).

mod common;
mod http;
