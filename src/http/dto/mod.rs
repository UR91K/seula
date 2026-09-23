//! Hand-written serde wire types for the HTTP adapter (ADR-0024), one module per
//! domain, independent of the generated proto types.

pub mod collections;
pub mod config;
pub mod media;
pub mod plugins;
pub mod projects;
pub mod samples;
pub mod search;
pub mod system;
pub mod tags;
pub mod tasks;

use crate::database::ProjectScope;
use crate::http::error::ApiError;

/// The `scope` query parameter of the plugin and sample routes that count or list the
/// projects using something (ADR-0040): `active` (the default) or `all`, which counts
/// archived projects too. Anything else is a 400, not a silent default.
pub fn parse_project_scope(raw: Option<&str>) -> Result<ProjectScope, ApiError> {
    match raw {
        None | Some("active") => Ok(ProjectScope::Active),
        Some("all") => Ok(ProjectScope::All),
        Some(other) => Err(ApiError::InvalidRequest(format!(
            "scope must be \"active\" or \"all\", not {:?}",
            other
        ))),
    }
}
