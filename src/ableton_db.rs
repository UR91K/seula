// /src/ableton_db.rs
use crate::error::DatabaseError;
use crate::models::PluginFormat;
use crate::utils::plugins::parse_plugin_format;
use rusqlite::{params, types::Type, Connection, Result as SqliteResult};
use std::path::PathBuf;

#[derive(Debug)]
pub struct DbPlugin {
    pub plugin_id: i32,
    pub module_id: Option<i32>,
    pub dev_identifier: String,
    pub name: String,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub sdk_version: Option<String>,
    pub flags: Option<i32>,
    pub parsestate: Option<i32>,
    pub enabled: Option<i32>,
}

pub struct AbletonDatabase {
    conn: Connection,
}

impl AbletonDatabase {
    pub fn new(db_path: PathBuf) -> Result<Self, DatabaseError> {
        let conn =
            Connection::open(db_path).map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn get_database_plugins(&self) -> Result<Vec<(String, PluginFormat)>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT name, dev_identifier FROM plugins WHERE scanstate = 1 AND enabled = 1",
        )?;
        let plugin_iter = stmt.query_map(params![], |row| {
            let name: String = row.get(0)?;
            let dev_identifier: String = row.get(1)?;
            let format = parse_plugin_format(&dev_identifier).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(1, "dev_identifier".to_string(), Type::Text)
            })?;
            Ok((name, format))
        })?;

        plugin_iter
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(DatabaseError::from)
    }

    pub fn get_plugin_by_dev_identifier(
        &self,
        dev_identifier: &str,
    ) -> Result<Option<DbPlugin>, DatabaseError> {
        // First, try to get the column names to build a robust query
        let column_info = self.get_plugin_table_columns()?;
        let query = self.build_plugin_query(&column_info);
        
        let mut stmt = self.conn.prepare(&query)?;
        let result: SqliteResult<DbPlugin> = stmt.query_row(params![dev_identifier], |row| {
            Ok(DbPlugin {
                plugin_id: row.get("plugin_id").unwrap_or_default(),
                module_id: row.get("module_id").ok(),
                dev_identifier: row.get("dev_identifier")?,
                name: row.get("name")?,
                vendor: row.get("vendor").ok(),
                version: row.get("version").ok(),
                sdk_version: row.get("sdk_version").ok(),
                flags: row.get("flags").ok(),
                parsestate: row.get("scanstate").or_else(|_| row.get("parsestate")).ok(),
                enabled: row.get("enabled").ok(),
            })
        });

        match result {
            Ok(plugin) => Ok(Some(plugin)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::QueryError(e.to_string())),
        }
    }

    /// Get the column names for the plugins table
    fn get_plugin_table_columns(&self) -> Result<Vec<String>, DatabaseError> {
        let mut stmt = self.conn.prepare("PRAGMA table_info(plugins)")?;
        let rows = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(1)?) // Column name is at index 1
        })?;
        
        let mut columns = Vec::new();
        for row in rows {
            columns.push(row?);
        }
        Ok(columns)
    }

    /// Build a plugin query that only includes columns that actually exist
    fn build_plugin_query(&self, available_columns: &[String]) -> String {
        let mut selected_columns = Vec::new();
        
        // Core columns that should always be there
        let core_columns = ["plugin_id", "dev_identifier", "name"];
        for col in &core_columns {
            if available_columns.contains(&col.to_string()) {
                selected_columns.push(*col);
            }
        }
        
        // Optional columns
        let optional_columns = [
            "module_id", "vendor", "version", "sdk_version", 
            "flags", "scanstate", "parsestate", "enabled"
        ];
        for col in &optional_columns {
            if available_columns.contains(&col.to_string()) {
                selected_columns.push(*col);
            }
        }
        
        format!(
            "SELECT {} FROM plugins WHERE dev_identifier = ?",
            selected_columns.join(", ")
        )
    }
}
