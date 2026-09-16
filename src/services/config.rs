use crate::config::{Config, CONFIG};
use crate::error::ConfigError;

/// Config mutation is on the static `CONFIG` lazy, not `ProjectDatabase` -- this
/// service holds no state of its own. It exists so gRPC and the future axum router
/// share one place for this logic; the CLI's `config` command reads `CONFIG`
/// directly today and has no mutation subcommands to route through this yet.
#[derive(Clone, Default)]
pub struct ConfigService;

impl ConfigService {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self) -> Result<&'static Config, ConfigError> {
        CONFIG.as_ref().map_err(|e| e.clone())
    }

    pub fn update_paths(&self, paths: Vec<String>) -> Result<(Config, Vec<String>), ConfigError> {
        let mut config = self.get()?.clone();
        let warnings = config.update_paths(paths)?;
        Ok((config, warnings))
    }

    pub fn add_path(&self, path: String) -> Result<(Config, Vec<String>), ConfigError> {
        let mut config = self.get()?.clone();
        let warnings = config.add_path(path)?;
        Ok((config, warnings))
    }

    pub fn remove_path(&self, path: &str) -> Result<Config, ConfigError> {
        let mut config = self.get()?.clone();
        config.remove_path(path)?;
        Ok(config)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_settings(
        &self,
        database_path: Option<String>,
        grpc_port: Option<u16>,
        log_level: Option<String>,
        media_storage_dir: Option<String>,
        max_cover_art_size_mb: Option<Option<u32>>,
        max_audio_file_size_mb: Option<Option<u32>>,
    ) -> Result<(Config, Vec<String>), ConfigError> {
        let mut config = self.get()?.clone();
        let warnings = config.update_settings(
            database_path,
            grpc_port,
            log_level,
            media_storage_dir,
            max_cover_art_size_mb,
            max_audio_file_size_mb,
        )?;
        Ok((config, warnings))
    }

    pub fn reload(&self) -> Result<(Config, Vec<String>), ConfigError> {
        Config::reload()
    }

    pub fn validate(&self) -> Result<Vec<String>, ConfigError> {
        self.get()?.validate()
    }
}
