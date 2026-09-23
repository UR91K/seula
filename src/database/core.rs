use crate::error::DatabaseError;
use chrono::{DateTime, Local, TimeZone};
use tracing::{debug, info};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// The schema this build writes and understands.
///
/// Stamped into `PRAGMA user_version`. Bump it for any breaking schema change; a
/// database at an older version is discarded and rebuilt (ADR-0011), and one at a newer
/// version is refused rather than destroyed.
pub const SCHEMA_VERSION: i32 = 3;

/// Every table, index and trigger, run on each open. A plain `.sql` file so tools
/// outside the Rust build, such as the mockup data generator, share the real schema.
const SCHEMA_SQL: &str = include_str!("schema.sql");

pub struct ProjectDatabase {
    pub conn: Connection,
}

impl ProjectDatabase {
    pub fn new(db_path: PathBuf) -> Result<Self, DatabaseError> {
        debug!("Opening database at {:?}", db_path);

        let conn = Self::open_at_current_schema(&db_path)?;
        let mut db = Self { conn };
        db.initialize()?;
        db.conn
            .pragma_update(None, "user_version", SCHEMA_VERSION)?;

        info!("Database initialized successfully at {:?}", db_path);
        Ok(db)
    }

    /// Open the database, discarding it first if it was written by an older schema.
    ///
    /// Seula has no migrations. Backwards compatibility is explicitly not required
    /// (ADR-0011), so a stale database is deleted and rebuilt — which destroys
    /// user-authored data that no rescan can recover, hence the loud warning.
    ///
    /// A *newer* database is a different matter: a downgrade must not silently wipe
    /// work done by a later build, so that case fails instead.
    fn open_at_current_schema(db_path: &Path) -> Result<Connection, DatabaseError> {
        let conn = Connection::open(db_path)?;
        let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if version == SCHEMA_VERSION {
            return Ok(conn);
        }

        if version > SCHEMA_VERSION {
            return Err(DatabaseError::ConnectionError(format!(
                "The database at {} was written by a newer version of Seula (schema {}, \
                 this build understands {}). Refusing to open it, because doing so would \
                 mean discarding it.",
                db_path.display(),
                version,
                SCHEMA_VERSION
            )));
        }

        // Version 0 is both "brand new file" and "written before user_version existed".
        // Only the latter has anything to discard.
        let has_tables: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;

        if has_tables == 0 {
            debug!("New database file, nothing to discard");
            return Ok(conn);
        }

        tracing::warn!(
            "The database at {} uses an older schema and cannot be migrated. Deleting and \
             rebuilding it. Projects will be recovered by the next scan; tags, collections, \
             tasks, notes and stored media are lost.",
            db_path.display()
        );

        // Windows will not remove a file whose handle is still open.
        drop(conn);
        Self::remove_database_files(db_path)?;

        Ok(Connection::open(db_path)?)
    }

    /// Remove the database and the write-ahead log files that belong to it.
    ///
    /// Leaving `-wal` or `-shm` behind next to a fresh database confuses SQLite.
    fn remove_database_files(db_path: &Path) -> Result<(), DatabaseError> {
        for suffix in ["", "-wal", "-shm"] {
            let path = if suffix.is_empty() {
                db_path.to_path_buf()
            } else {
                PathBuf::from(format!("{}{}", db_path.display(), suffix))
            };

            match std::fs::remove_file(&path) {
                Ok(()) => debug!("Removed {}", path.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(DatabaseError::ConnectionError(format!(
                        "Failed to remove the stale database file {}: {}",
                        path.display(),
                        e
                    )))
                }
            }
        }
        Ok(())
    }

    fn initialize(&mut self) -> Result<(), DatabaseError> {
        debug!("Initializing database tables and indexes");

        // Never set historically, so the ON DELETE CASCADE declarations below were
        // inert. The plugin child tables depend on real cascade, and every write path
        // upserts rather than INSERT OR REPLACE, so enabling it is safe.
        self.conn.pragma_update(None, "foreign_keys", "ON")?;

        self.conn.execute_batch(SCHEMA_SQL)?;

        debug!("Database schema initialized successfully");

        // Rebuild FTS5 table to fix any NULL values in existing data
        self.rebuild_fts5_table()?;

        Ok(())
    }

    /// Read a value from the application state store.
    pub fn get_app_state(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM app_state WHERE key = ?",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Write a value into the application state store.
    pub fn set_app_state(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT INTO app_state (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
            params![key, value, Local::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn get_last_scanned_time(
        &self,
        path: &Path,
    ) -> Result<Option<DateTime<Local>>, DatabaseError> {
        let path_str = path.to_string_lossy().to_string();

        let last_parsed: Option<i64> = self
            .conn
            .query_row(
                "SELECT last_parsed_at FROM projects WHERE path = ? AND is_active = true",
                params![path_str],
                |row| row.get(0),
            )
            .optional()?;

        Ok(last_parsed.map(|timestamp| {
            Local
                .timestamp_opt(timestamp, 0)
                .single()
                .expect("Invalid timestamp in database")
        }))
    }

    pub fn rebuild_fts5_table(&mut self) -> Result<(), DatabaseError> {
        debug!("Rebuilding FTS5 table to fix NULL values");

        // Clear and rebuild the FTS5 table
        self.conn.execute("DELETE FROM project_search", [])?;

        // Repopulate with corrected data
        self.conn.execute(
            r#"
            INSERT INTO project_search (
                project_id, name, path, plugins, samples, tags, notes, created_at, modified_at, tempo,
                key_signature, time_signature, version
            )
            SELECT 
                p.id,
                p.name,
                p.path,
                COALESCE((SELECT GROUP_CONCAT(pl.name || ' ' || COALESCE(pl.vendor, ''), ' ')
                 FROM plugins pl
                 JOIN project_plugins pp ON pp.plugin_id = pl.id
                 WHERE pp.project_id = p.id), ''),
                COALESCE((SELECT GROUP_CONCAT(s.name, ' ')
                 FROM samples s
                 JOIN project_samples ps ON ps.sample_id = s.id
                 WHERE ps.project_id = p.id), ''),
                COALESCE((SELECT GROUP_CONCAT(t.name, ' ')
                 FROM tags t
                 JOIN project_tags pt ON pt.tag_id = t.id
                 WHERE pt.project_id = p.id), ''),
                COALESCE(p.notes, ''),
                strftime('%Y-%m-%d %H:%M:%S', datetime(p.created_at, 'unixepoch')),
                strftime('%Y-%m-%d %H:%M:%S', datetime(p.modified_at, 'unixepoch')),
                CAST(p.tempo AS TEXT),
                CASE 
                    WHEN p.key_signature_tonic IS NOT NULL AND p.key_signature_scale IS NOT NULL 
                    THEN p.key_signature_tonic || ' ' || p.key_signature_scale
                    ELSE ''
                END,
                CAST(p.time_signature_numerator AS TEXT) || '/' || CAST(p.time_signature_denominator AS TEXT),
                p.daw_version_display
            FROM projects p
            WHERE p.is_active = true
            "#,
            [],
        )?;

        debug!("FTS5 table rebuilt successfully");
        Ok(())
    }
}
