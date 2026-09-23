-- The Seula database schema.
--
-- Loaded by `ProjectDatabase::initialize()` in src/database/core.rs through
-- `include_str!`, so it is compiled into the binary and there is no file to find at
-- runtime. Kept as plain SQL so tools outside the Rust build can use the same schema
-- as the program, e.g. mockup/data/generate.py, which seeds a mock database from it.
--
-- Every statement is `IF NOT EXISTS` and the whole file runs on every open. Adding a
-- table needs no SCHEMA_VERSION bump; changing an existing one does, and a bump
-- discards the user's database (ADR-0011).

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
-- The junction primary keys lead with project_id, so they cannot answer "how many
-- projects use this item". These serve the per-row project counts (ADR-0034).
CREATE INDEX IF NOT EXISTS idx_project_samples_sample ON project_samples(sample_id);
CREATE INDEX IF NOT EXISTS idx_project_plugins_plugin ON project_plugins(plugin_id);
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
