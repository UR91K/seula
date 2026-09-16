//! Dispatching a project file to the parser for its DAW.
//!
//! A closed enum, not `dyn DawParser` trait objects (ADR-0016) — this project has no
//! third-party parser ecosystem to support, and the codebase consistently prefers
//! compiler-enforced exhaustiveness over runtime-extensible erasure. Adding a DAW means
//! adding a variant here and fixing every match the compiler then flags; that is the
//! point, not a cost to minimize.
//!
//! Only Ableton Live is supported today (ADR-0014). `AnyDawParser` has exactly one
//! variant until a second DAW's parser is actually being built.

use std::path::{Path, PathBuf};

use crate::error::LiveSetError;
use crate::project::Project;

/// Parses Ableton Live's `.als` project files.
pub struct AbletonParser;

impl AbletonParser {
    pub fn parse(&self, path: &Path) -> Result<Project, LiveSetError> {
        Project::new(path.to_path_buf())
    }
}

/// Every DAW parser this build knows about.
pub enum AnyDawParser {
    Ableton(AbletonParser),
}

/// Failure of [`AnyDawParser::parse`], or of dispatch itself.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{0}")]
    Ableton(#[from] LiveSetError),
    #[error("unsupported file type: {0}")]
    UnsupportedFileType(PathBuf),
}

impl AnyDawParser {
    /// Pick a parser for `path` by extension, or `None` if no known DAW handles it.
    pub fn for_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("als") => Some(AnyDawParser::Ableton(AbletonParser)),
            _ => None,
        }
    }

    pub fn parse(&self, path: &Path) -> Result<Project, ParseError> {
        match self {
            AnyDawParser::Ableton(p) => p.parse(path).map_err(ParseError::Ableton),
        }
    }

    /// File extensions any known parser handles (lowercase, no dot).
    pub fn supported_extensions() -> &'static [&'static str] {
        &["als"]
    }
}
