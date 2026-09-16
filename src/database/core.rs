use crate::error::DatabaseError;
use chrono::{DateTime, Local, TimeZone};
use log::{debug, info};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// The schema this build writes and understands.
///
/// Stamped into `PRAGMA user_version`. Bump it for any breaking schema change; a
/// database at an older version is discarded and rebuilt (ADR-0011), and one at a newer
/// version is refused rather than destroyed.
pub const SCHEMA_VERSION: i32 = 3;

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

        log::warn!(
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

        self.conn.execute_batch(
            r#"--sql
            -- Core tables
            CREATE TABLE IF NOT EXISTS projects (
                is_active BOOLEAN NOT NULL DEFAULT true,

                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                hash TEXT NOT NULL,
                notes TEXT,
                created_at DATETIME NOT NULL,
                modified_at DATETIME NOT NULL,
                last_parsed_at DATETIME NOT NULL,
                
                tempo REAL NOT NULL,
                time_signature_numerator INTEGER NOT NULL,
                time_signature_denominator INTEGER NOT NULL,
                key_signature_tonic TEXT,
                key_signature_scale TEXT,
                duration_seconds INTEGER,
                furthest_bar REAL,
                
                -- DAW identification (ADR-0015). daw_version_display is a plain string
                -- produced by that DAW's own Display impl; structured, queryable
                -- version data lives in a DAW-specific side table, never here.
                daw_type TEXT NOT NULL,
                daw_version_display TEXT NOT NULL,
                audio_file_id TEXT,
                FOREIGN KEY (audio_file_id) REFERENCES media_files(id) ON DELETE SET NULL
            );

            -- Ableton's structured version data (ADR-0015). Kept out of `projects`
            -- itself so the table stays generic across DAWs; queried via a join on
            -- this primary key rather than bare columns, e.g. for exact-match
            -- filtering and numeric sort, which daw_version_display cannot do.
            CREATE TABLE IF NOT EXISTS project_ableton_metadata (
                project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                version_major INTEGER NOT NULL,
                version_minor INTEGER NOT NULL,
                version_patch INTEGER NOT NULL,
                version_beta BOOLEAN NOT NULL
            );

            -- A row means the plugin exists on this machine and/or in a project
            -- (ADR-0007). Usage is expressed by project_plugins, exactly as before.
            CREATE TABLE IF NOT EXISTS plugins (
                id TEXT PRIMARY KEY,

                -- Identity (ADR-0005). plugin_kind is 'VST2' or 'VST3'; uid is
                -- lowercase hex, normalised through PluginKey::uid_hex(). Deliberately
                -- NOT `format`, which bakes in Ableton's instr/audiofx classification.
                plugin_kind TEXT NOT NULL,
                uid TEXT NOT NULL,

                name TEXT NOT NULL,
                format TEXT NOT NULL,        -- four-variant PluginFormat, display only
                vendor TEXT,
                version TEXT,

                -- NULL means no plugin scan has looked yet. 1 the last scan found it,
                -- 0 the last scan looked and did not.
                installed BOOLEAN,
                last_scanned_at DATETIME,

                -- A representative reference, kept for display. plugin_refs is the
                -- authoritative and exhaustive mapping (ADR-0009).
                dev_identifier TEXT,

                -- Scanner data. NULL when the plugin is referenced but not installed.
                path TEXT,                   -- one location of possibly several (ADR-0010)
                category TEXT,
                is_instrument BOOLEAN,
                audio_in_channels INTEGER,
                audio_out_channels INTEGER,
                audio_in_buses INTEGER,
                audio_out_buses INTEGER,
                has_midi_input BOOLEAN,
                has_midi_output BOOLEAN,
                presets INTEGER,
                parameters INTEGER,
                latency_samples INTEGER,
                has_gui BOOLEAN,
                vendor_url TEXT,
                vendor_email TEXT,
                is_shell BOOLEAN NOT NULL DEFAULT 0,
                shell_parent_uid TEXT,       -- plain column; uid alone is not a unique key

                -- VST2 extras
                fourcc TEXT,
                preset_chunks BOOLEAN,
                f64_precision BOOLEAN,
                silent_when_stopped BOOLEAN,
                midi_in_channels INTEGER,
                midi_out_channels INTEGER,

                -- VST3 extras
                factory_flags INTEGER,

                UNIQUE(plugin_kind, uid)
            );

            -- One row per distinct dev_identifier seen in any project, and the plugin
            -- it resolves to. A scalar column cannot hold this: a multi-class VST3
            -- bundle is referenced by whichever processor class the user instantiated,
            -- so one plugin can answer to several identifiers (ADR-0009).
            CREATE TABLE IF NOT EXISTS plugin_refs (
                dev_identifier TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                ableton_name TEXT,           -- <Name>/<PlugName> from the .als
                ableton_format TEXT,         -- Ableton's instr/audiofx call; never identity
                resolved_via TEXT NOT NULL,  -- 'uid' | 'class_id' | 'created'
                first_seen_at DATETIME NOT NULL,
                FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
            );

            -- Every class a VST3 bundle's factory exports. class_id is normalised the
            -- same way as plugins.uid so the matching fallback can join on it.
            CREATE TABLE IF NOT EXISTS plugin_classes (
                plugin_id TEXT NOT NULL,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                class_id TEXT NOT NULL,
                cardinality INTEGER NOT NULL,
                version TEXT NOT NULL,
                FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS plugin_buses (
                plugin_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                media TEXT NOT NULL,
                name TEXT NOT NULL,
                channel_count INTEGER NOT NULL,
                bus_type INTEGER NOT NULL,
                flags INTEGER NOT NULL,
                FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS samples (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                is_present BOOLEAN NOT NULL
            );

            CREATE TABLE IF NOT EXISTS media_files (
                id TEXT PRIMARY KEY,
                original_filename TEXT NOT NULL,
                file_extension TEXT NOT NULL,
                media_type TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                mime_type TEXT NOT NULL,
                uploaded_at DATETIME NOT NULL,
                checksum TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at DATETIME NOT NULL
            );

            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                notes TEXT,
                created_at DATETIME NOT NULL,
                modified_at DATETIME NOT NULL,
                cover_art_id TEXT,
                FOREIGN KEY (cover_art_id) REFERENCES media_files(id) ON DELETE SET NULL
            );

            -- Junction tables
            CREATE TABLE IF NOT EXISTS project_plugins (
                project_id TEXT NOT NULL,
                plugin_id TEXT NOT NULL,
                PRIMARY KEY (project_id, plugin_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS project_samples (
                project_id TEXT NOT NULL,
                sample_id TEXT NOT NULL,
                PRIMARY KEY (project_id, sample_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (sample_id) REFERENCES samples(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS project_tags (
                project_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                PRIMARY KEY (project_id, tag_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS collection_projects (
                collection_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                added_at DATETIME NOT NULL,
                PRIMARY KEY (collection_id, project_id),
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            -- State belonging to the application rather than to any entity.
            --
            -- Additive: `CREATE TABLE IF NOT EXISTS` means an existing database gains
            -- this on its next open, so it needs no SCHEMA_VERSION bump. Bumping the
            -- version would discard the user's data (ADR-0011), which would be an
            -- absurd price for one new table.
            CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at DATETIME NOT NULL
            );

            -- Additional features
            CREATE TABLE IF NOT EXISTS project_tasks (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                description TEXT NOT NULL,
                completed BOOLEAN NOT NULL DEFAULT FALSE,
                created_at DATETIME NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            -- Basic indexes for performance
            CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);
            CREATE INDEX IF NOT EXISTS idx_plugins_name ON plugins(name);
            CREATE INDEX IF NOT EXISTS idx_plugins_installed ON plugins(installed);
            CREATE INDEX IF NOT EXISTS idx_plugin_refs_plugin ON plugin_refs(plugin_id);
            CREATE INDEX IF NOT EXISTS idx_plugin_classes_class_id ON plugin_classes(class_id);
            CREATE INDEX IF NOT EXISTS idx_plugin_classes_plugin ON plugin_classes(plugin_id);
            CREATE INDEX IF NOT EXISTS idx_plugin_buses_plugin ON plugin_buses(plugin_id);
            CREATE INDEX IF NOT EXISTS idx_samples_path ON samples(path);
            CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
            CREATE INDEX IF NOT EXISTS idx_collection_projects_position ON collection_projects(collection_id, position);
            CREATE INDEX IF NOT EXISTS idx_projects_is_active ON projects(is_active);
            CREATE INDEX IF NOT EXISTS idx_media_files_type ON media_files(media_type);

            -- Full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS project_search USING fts5(
                project_id UNINDEXED,  -- Reference to projects table
                name,                  -- Project name
                path,                 -- Project path
                plugins,              -- Plugin list
                samples,              -- Sample list
                tags,                 -- Tags list
                notes,                -- Project notes
                created_at,           -- Creation timestamp
                modified_at,          -- Modification timestamp
                tempo,                -- Project tempo
                key_signature,        -- Key signature (C Major, F# Minor, etc.)
                time_signature,       -- Time signature (4/4, 3/4, etc.)
                version,              -- daw_version_display (11.0.0, 12.0.1, etc.)
                tokenize='porter unicode61'
            );

            -- FTS5 triggers for maintaining the search index
            CREATE TRIGGER IF NOT EXISTS projects_au AFTER UPDATE ON projects BEGIN
                DELETE FROM project_search WHERE project_id = old.id;
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
                WHERE p.id = new.id;
            END;

            CREATE TRIGGER IF NOT EXISTS projects_ad AFTER DELETE ON projects BEGIN
                DELETE FROM project_search WHERE project_id = old.id;
            END;

            -- Update FTS index after project insert (done manually to ensure all relations are set)
            CREATE TRIGGER IF NOT EXISTS projects_ai AFTER INSERT ON projects BEGIN
                INSERT INTO project_search (
                    project_id, name, path, plugins, samples, tags, notes, created_at, modified_at, tempo,
                    key_signature, time_signature, version
                )
                SELECT 
                    p.id,
                    p.name,
                    p.path,
                    '',  -- Empty plugins (will be updated after linking)
                    '',  -- Empty samples (will be updated after linking)
                    '',  -- Empty tags (will be updated after linking)
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
                WHERE p.id = new.id;
            END;

            -- Triggers to update FTS5 when project tags change
            CREATE TRIGGER IF NOT EXISTS project_tags_ai AFTER INSERT ON project_tags BEGIN
                UPDATE project_search SET
                    tags = (
                        SELECT GROUP_CONCAT(t.name, ' ')
                        FROM tags t
                        JOIN project_tags pt ON pt.tag_id = t.id
                        WHERE pt.project_id = new.project_id
                    )
                WHERE project_id = new.project_id;
            END;

            CREATE TRIGGER IF NOT EXISTS project_tags_ad AFTER DELETE ON project_tags BEGIN
                UPDATE project_search SET
                    tags = COALESCE((
                        SELECT GROUP_CONCAT(t.name, ' ')
                        FROM tags t
                        JOIN project_tags pt ON pt.tag_id = t.id
                        WHERE pt.project_id = old.project_id
                    ), '')
                WHERE project_id = old.project_id;
            END;
            "#,
        )?;

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
