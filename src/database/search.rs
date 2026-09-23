use crate::error::DatabaseError;
use crate::project::Project;
use crate::{AbletonVersion, KeySignature, Sample, TimeSignature};
use chrono::{Local, TimeZone};
use tracing::debug;
use rusqlite::{types::ToSql, OptionalExtension};
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

use super::ProjectDatabase;

#[derive(Debug, Default)]
#[allow(unused)]
pub struct SearchQuery {
    // File properties
    pub path: Option<String>,
    pub name: Option<String>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,

    // Musical properties
    pub version: Option<String>,
    pub key: Option<String>,
    pub bpm: Option<String>,
    pub time_signature: Option<String>,
    pub estimated_duration: Option<String>,

    // Content properties
    pub plugin: Option<String>,
    pub sample: Option<String>,
    pub tag: Option<String>,
    /// A collection's name, matched whole and case-insensitively. Collections are
    /// not in the FTS table, so this filters by membership instead of matching text.
    pub collection: Option<String>,

    // Full text search
    pub text: String,
}

#[derive(Debug)]
pub struct SearchResult {
    pub project: Project,
    pub rank: f64,
    pub match_reason: Vec<MatchReason>,
}

#[derive(Debug)]
pub enum MatchReason {
    Name(String),
    Path(String),
    Plugin(String),
    Sample(String),
    Tag(String),
    Collection(String),
    KeySignature(String),
    TimeSignature(String),
    Tempo(String),
    Version(String),
    Notes(String),
    DateCreated(String),
    DateModified(String),
}

impl SearchQuery {
    fn strip_quotes(value: &str) -> String {
        let value = value.trim();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value[1..value.len() - 1].to_string()
        } else {
            value.to_string()
        }
    }

    pub fn parse(input: &str) -> Self {
        debug!("Parsing search query input: '{}'", input);
        let mut query = SearchQuery::default();
        let mut remaining_text = Vec::new();
        let mut current_pos = 0;

        while current_pos < input.len() {
            let rest = &input[current_pos..];
            let mut term_end = rest.find(' ').unwrap_or(rest.len());

            // If we find a colon, check if this is a date operator
            if let Some(colon_pos) = rest[..term_end].find(':') {
                let operator = &rest[..colon_pos];
                match operator {
                    "dc" | "dm" => {
                        // For date operators, look for the next non-date character
                        // This allows spaces in the date value
                        let value_start = colon_pos + 1;
                        let mut value_end = value_start;
                        while value_end < rest.len() {
                            let c = rest.as_bytes()[value_end];
                            if !matches!(c, b'0'..=b'9' | b'-' | b':' | b' ') {
                                break;
                            }
                            value_end += 1;
                        }
                        term_end = value_end;

                        let value = &rest[value_start..value_end];
                        debug!("Found date operator '{}' with value '{}'", operator, value);
                        let cleaned_value = Self::strip_quotes(value);
                        match operator {
                            "dc" => query.date_created = Some(cleaned_value),
                            "dm" => query.date_modified = Some(cleaned_value),
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        // A quoted value runs to its closing quote, spaces and all, so
                        // `collection:"night drives"` and `plugin:"Pro-Q 3"` are one
                        // term each. Unterminated, it runs to the end of the input.
                        let value_start = colon_pos + 1;
                        if let Some(&quote) = rest.as_bytes().get(value_start) {
                            if quote == b'"' || quote == b'\'' {
                                term_end = rest[value_start + 1..]
                                    .find(quote as char)
                                    .map(|close| value_start + 1 + close + 1)
                                    .unwrap_or(rest.len());
                            }
                        }
                        let value = &rest[value_start..term_end];
                        debug!("Found operator '{}' with value '{}'", operator, value);
                        let mut cleaned_value = Self::strip_quotes(value);
                        if cleaned_value.len() == value.trim().len() {
                            // Unterminated: only the opening quote to drop.
                            cleaned_value = cleaned_value.trim_start_matches(['"', '\'']).to_string();
                        }
                        match operator {
                            "path" => query.path = Some(cleaned_value),
                            "name" => query.name = Some(cleaned_value),
                            "version" => query.version = Some(cleaned_value),
                            "key" => query.key = Some(cleaned_value),
                            "bpm" => query.bpm = Some(cleaned_value),
                            "ts" => query.time_signature = Some(cleaned_value),
                            "ed" => query.estimated_duration = Some(cleaned_value),
                            "plugin" => query.plugin = Some(cleaned_value),
                            "sample" => query.sample = Some(cleaned_value),
                            "tag" => query.tag = Some(cleaned_value),
                            "collection" => query.collection = Some(cleaned_value),
                            _ => {
                                debug!("Unknown operator '{}', treating as text", operator);
                                remaining_text.push(&rest[..term_end]);
                            }
                        }
                    }
                }
            } else {
                debug!(
                    "No operator found, adding to remaining text: '{}'",
                    &rest[..term_end]
                );
                remaining_text.push(&rest[..term_end]);
            }

            // Move past this term and any following whitespace
            current_pos += term_end;
            while current_pos < input.len() && input.as_bytes()[current_pos] == b' ' {
                current_pos += 1;
            }
        }

        query.text = remaining_text.join(" ");
        debug!("Final query state: {:?}", query);
        query
    }

    fn build_fts5_query(&self) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        // Helper function to add a column-specific condition
        let mut add_column_condition = |column: &str, value: &str| {
            if column == "created_at" || column == "modified_at" {
                // For dates, use a proper FTS5 prefix match
                // The * must be outside the quotes according to the docs
                conditions.push(format!("{} : \"{}\" *", column, value));
            } else {
                conditions.push(format!("{} : \"{}\"", column, value));
            }
            params.push(value.to_string());
        };

        // Add specific field conditions
        if let Some(ref path) = self.path {
            add_column_condition("path", path);
        }
        if let Some(ref name) = self.name {
            add_column_condition("name", name);
        }
        if let Some(ref version) = self.version {
            add_column_condition("version", version);
        }
        if let Some(ref key) = self.key {
            add_column_condition("key_signature", key);
        }
        if let Some(ref bpm) = self.bpm {
            add_column_condition("tempo", bpm);
        }
        if let Some(ref ts) = self.time_signature {
            add_column_condition("time_signature", ts);
        }
        if let Some(ref plugin) = self.plugin {
            add_column_condition("plugins", plugin);
        }
        if let Some(ref sample) = self.sample {
            add_column_condition("samples", sample);
        }
        if let Some(ref tag) = self.tag {
            add_column_condition("tags", tag);
        }
        if let Some(ref created) = self.date_created {
            add_column_condition("created_at", created);
        }
        if let Some(ref modified) = self.date_modified {
            add_column_condition("modified_at", modified);
        }

        // Add full text search if present
        if !self.text.is_empty() {
            conditions.push(format!("\"{}\"", self.text));
            params.push(self.text.clone());
        }

        let fts5_query = if conditions.is_empty() {
            String::new()
        } else {
            conditions.join(" AND ")
        };

        let query = format!(
            "SELECT project_id, rank, name, path, plugins, samples, tags, notes, created_at, modified_at, tempo, key_signature, time_signature, version
             FROM project_search 
             WHERE project_search MATCH ? 
             ORDER BY rank"
        );

        (query, vec![fts5_query])
    }
}

impl ProjectDatabase {
    /// The ids of the projects in the collection with this name, in tracklist order.
    /// Empty when no collection has the name.
    fn collection_member_ids(&mut self, name: &str) -> Result<Vec<String>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT cp.project_id FROM collection_projects cp
             JOIN collections c ON c.id = cp.collection_id
             WHERE c.name = ? COLLATE NOCASE
             ORDER BY cp.position",
        )?;
        let ids = stmt
            .query_map([name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    pub fn search_simple(&mut self, query: &str) -> Result<Vec<Project>, DatabaseError> {
        debug!("Performing search with query: {}", query);
        let tx = self.conn.transaction()?;

        // First, get all matching project paths
        let project_paths = {
            let mut stmt = tx.prepare(
                r#"
                SELECT DISTINCT p.path 
                FROM projects p
                LEFT JOIN project_plugins pp ON pp.project_id = p.id
                LEFT JOIN plugins pl ON pl.id = pp.plugin_id
                LEFT JOIN project_samples ps ON ps.project_id = p.id
                LEFT JOIN samples s ON s.id = ps.sample_id
                WHERE 
                    p.name LIKE ?1 OR
                    pl.name LIKE ?1 OR
                    s.name LIKE ?1 OR
                    pl.vendor LIKE ?1
                "#,
            )?;

            let pattern = format!("%{}%", query);
            debug!("Using search pattern: {}", pattern);

            let paths: Vec<String> = stmt
                .query_map([&pattern], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            debug!("Found {} matching project paths", paths.len());
            paths
        };

        let mut results = Vec::new();

        // For each path, get the full project details
        for path in project_paths {
            let project = {
                let mut stmt = tx.prepare(
                    r#"
                    SELECT
                        p.id, p.path, p.name, p.hash, p.created_at, p.modified_at, p.last_parsed_at,
                        p.tempo, p.time_signature_numerator, p.time_signature_denominator,
                        p.key_signature_tonic, p.key_signature_scale, p.duration_seconds, p.furthest_bar,
                        p.daw_type, p.daw_version_display,
                        a.version_major, a.version_minor, a.version_patch, a.version_beta
                    FROM projects p
                    JOIN project_ableton_metadata a ON a.project_id = p.id
                    WHERE p.path = ?
                    "#,
                )?;

                stmt.query_row([&path], |row| {
                    let project_id: String = row.get(0)?;
                    debug!("Found project with ID: {}", project_id);

                    let duration_secs: Option<i64> = row.get(12)?;
                    let created_timestamp: i64 = row.get(4)?;
                    let modified_timestamp: i64 = row.get(5)?;
                    let parsed_timestamp: i64 = row.get(6)?;

                    let mut live_set = Project {
                        is_active: true,
                        id: Uuid::parse_str(&project_id).map_err(|_| {
                            rusqlite::Error::InvalidParameterName("Invalid UUID".into())
                        })?,
                        file_path: PathBuf::from(row.get::<_, String>(1)?),
                        name: row.get(2)?,
                        file_hash: row.get(3)?,
                        created_time: Local
                            .timestamp_opt(created_timestamp, 0)
                            .single()
                            .ok_or_else(|| {
                                rusqlite::Error::InvalidParameterName("Invalid timestamp".into())
                            })?,
                        modified_time: Local
                            .timestamp_opt(modified_timestamp, 0)
                            .single()
                            .ok_or_else(|| {
                                rusqlite::Error::InvalidParameterName("Invalid timestamp".into())
                            })?,
                        last_parsed_timestamp: Local
                            .timestamp_opt(parsed_timestamp, 0)
                            .single()
                            .ok_or_else(|| {
                                rusqlite::Error::InvalidParameterName("Invalid timestamp".into())
                            })?,

                        tempo: row.get(7)?,
                        time_signature: TimeSignature {
                            numerator: row.get(8)?,
                            denominator: row.get(9)?,
                        },
                        key_signature: match (
                            row.get::<_, Option<String>>(10)?,
                            row.get::<_, Option<String>>(11)?,
                        ) {
                            (Some(tonic), Some(scale)) => {
                                debug!("Found key signature: {} {}", tonic, scale);
                                Some(KeySignature {
                                    tonic: tonic
                                        .parse()
                                        .map_err(|e| rusqlite::Error::InvalidParameterName(e))?,
                                    scale: scale
                                        .parse()
                                        .map_err(|e| rusqlite::Error::InvalidParameterName(e))?,
                                })
                            }
                            _ => None,
                        },
                        furthest_bar: row.get(13)?,

                        daw_type: row.get(14)?,
                        daw_version_display: row.get(15)?,
                        ableton_metadata: AbletonVersion {
                            major: row.get(16)?,
                            minor: row.get(17)?,
                            patch: row.get(18)?,
                            beta: row.get(19)?,
                        },

                        estimated_duration: duration_secs.map(chrono::Duration::seconds),
                        plugins: HashSet::new(),
                        samples: HashSet::new(),
                        tags: HashSet::new(),
                    };

                    // Get plugins in a new scope
                    {
                        let mut stmt = tx.prepare(
                            r#"
                            SELECT p.* 
                            FROM plugins p
                            JOIN project_plugins pp ON pp.plugin_id = p.id
                            WHERE pp.project_id = ?
                            "#,
                        )?;

                        let plugins = stmt
                            .query_map([&project_id], |row| {
                                crate::database::helpers::row_to_plugin(row)
                            })?
                            .filter_map(|r| r.ok())
                            .collect::<HashSet<_>>();

                        debug!("Retrieved {} plugins", plugins.len());
                        live_set.plugins = plugins;
                    }

                    // Get samples in a new scope
                    {
                        let mut stmt = tx.prepare(
                            r#"
                            SELECT s.* 
                            FROM samples s
                            JOIN project_samples ps ON ps.sample_id = s.id
                            WHERE ps.project_id = ?
                            "#,
                        )?;

                        let samples = stmt
                            .query_map([&project_id], |row| {
                                let name: String = row.get(1)?;
                                debug!("Found sample: {}", name);
                                Ok(Sample {
                                    id: Uuid::new_v4(),
                                    name,
                                    path: PathBuf::from(row.get::<_, String>(2)?),
                                    is_present: row.get(3)?,
                                })
                            })?
                            .filter_map(|r| r.ok())
                            .collect::<HashSet<_>>();

                        debug!("Retrieved {} samples", samples.len());
                        live_set.samples = samples;
                    }

                    Ok(live_set)
                })
                .optional()?
            };

            if let Some(live_set) = project {
                debug!("Retrieved project: {}", live_set.name);
                results.push(live_set);
            }
        }

        tx.commit()?;
        debug!("Successfully retrieved {} matching projects", results.len());
        Ok(results)
    }

    pub fn search_fts(&mut self, query: &SearchQuery) -> Result<Vec<SearchResult>, DatabaseError> {
        debug!("Performing FTS5 search with query: {:?}", query);

        let members = match &query.collection {
            Some(name) => Some(self.collection_member_ids(name)?),
            None => None,
        };

        // Check if query is effectively empty
        let (sql_query, params) = query.build_fts5_query();
        if params.is_empty() || params[0].is_empty() {
            // `collection:` alone is a whole query: the collection's projects, in
            // its order. Archived ones are left out, as the FTS path leaves them.
            let (Some(name), Some(members)) = (&query.collection, members) else {
                debug!("Empty query detected, returning empty results");
                return Ok(Vec::new());
            };
            let mut results = Vec::new();
            for id in members {
                if let Some(project) = self.get_project_by_id(&id)? {
                    results.push(SearchResult {
                        project,
                        rank: 0.0,
                        match_reason: vec![MatchReason::Collection(name.clone())],
                    });
                }
            }
            return Ok(results);
        }
        let members: Option<HashSet<String>> = members.map(|ids| ids.into_iter().collect());

        // First collect all matching paths in a transaction
        let matching_paths = {
            let tx = self.conn.transaction()?;

            debug!("FTS5 query: {}", sql_query);
            debug!("Query params: {:?}", params);

            let results = {
                let mut stmt = tx.prepare(&sql_query)?;
                let param_refs: Vec<&dyn ToSql> = params.iter().map(|p| p as &dyn ToSql).collect();

                // Collect all results into a vector
                let mut results = Vec::new();
                let mut rows = stmt.query(param_refs.as_slice())?;
                while let Some(row) = rows.next()? {
                    let plugins: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                    debug!("Checking row with plugins: {:?}", plugins);
                    results.push((
                        row.get::<_, String>(0)?,                             // project_id
                        row.get::<_, f64>(1)?,                                // rank
                        row.get::<_, String>(2)?,                             // name
                        row.get::<_, String>(3)?,                             // path
                        plugins,                                              // plugins
                        row.get::<_, Option<String>>(5)?.unwrap_or_default(), // samples
                    ));
                }
                debug!("Found {} potential matches", results.len());
                results
            };

            tx.commit()?;
            results
        };

        // Now get full project details and build search results
        let mut search_results = Vec::new();
        #[allow(unused)]
        for (project_id, rank, name, path, plugins, samples) in matching_paths {
            if members.as_ref().is_some_and(|m| !m.contains(&project_id)) {
                continue;
            }
            debug!("Processing match: {} ({})", name, path);
            if let Ok(Some(project)) = self.get_project_by_path(&path) {
                let mut match_reason = Vec::new();

                // Add match reasons based on what matched
                if let Some(plugin_query) = &query.plugin {
                    let plugin_query = plugin_query.to_lowercase();
                    let plugins_lower = plugins.to_lowercase();
                    debug!(
                        "  Checking if '{}' exists in '{}'",
                        plugin_query, plugins_lower
                    );
                    if plugins_lower.contains(&plugin_query) {
                        debug!("  Found plugin match!");
                        match_reason.push(MatchReason::Plugin(plugin_query.clone()));
                    }
                }
                if let Some(collection) = &query.collection {
                    match_reason.push(MatchReason::Collection(collection.clone()));
                }
                if let Some(bpm) = &query.bpm {
                    match_reason.push(MatchReason::Tempo(bpm.clone()));
                }
                if let Some(date_created) = &query.date_created {
                    match_reason.push(MatchReason::DateCreated(date_created.clone()));
                }
                if let Some(date_modified) = &query.date_modified {
                    match_reason.push(MatchReason::DateModified(date_modified.clone()));
                }

                search_results.push(SearchResult {
                    project,
                    rank,
                    match_reason,
                });
            }
        }

        debug!("Successfully built {} search results", search_results.len());
        Ok(search_results)
    }
}
