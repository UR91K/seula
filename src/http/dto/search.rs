//! HTTP wire types for the search domain (ADR-0024).

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
