# Design Document: VST Parser Subprocess

## Overview

This document describes the design for a standalone VST plugin parser that runs as a separate subprocess. The parser scans VST2 and VST3 plugins, extracts metadata, and outputs JSON compatible with Seula's existing plugin database schema. The subprocess architecture isolates the main application from crashes caused by unstable plugins.

## Architecture

The system consists of two main components:

1. **vst-parser** - A standalone Rust binary in a separate Cargo workspace member
2. **Main Process Integration** - Code in the main Seula application to spawn and communicate with the parser

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Seula Process                        │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │   Scanner    │───▶│  Subprocess  │───▶│    Database      │  │
│  │   Manager    │    │   Spawner    │    │    Updater       │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│                             │                                    │
└─────────────────────────────┼────────────────────────────────────┘
                              │ spawn + stdin/stdout
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      vst-parser Subprocess                       │
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │     CLI      │───▶│   Plugin     │───▶│     JSON         │  │
│  │   Parser     │    │   Loader     │    │    Output        │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│                             │                                    │
│                      ┌──────┴──────┐                            │
│                      ▼             ▼                            │
│               ┌──────────┐  ┌──────────┐                        │
│               │   VST2   │  │   VST3   │                        │
│               │  Loader  │  │  Loader  │                        │
│               └──────────┘  └──────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### VST Parser Binary (vst-parser)

A separate Cargo workspace member located at `crates/vst-parser/`.

#### CLI Interface

```
vst-parser [OPTIONS] <COMMAND>

Commands:
  scan      Scan a single plugin file
  discover  Discover and scan all plugins in a directory

Options:
  -h, --help     Print help
  -V, --version  Print version

scan <PATH> [OPTIONS]
  Arguments:
    <PATH>  Path to the plugin file or bundle
  
  Options:
    --format <FORMAT>  Force format detection (vst2, vst3)
    --timeout <SECS>   Timeout for plugin loading (default: 10)

discover <PATH> [OPTIONS]
  Arguments:
    <PATH>  Directory to scan for plugins
  
  Options:
    --recursive        Scan subdirectories (default: true)
    --progress         Output progress to stderr
```

#### Plugin Loader Trait

```rust
pub trait PluginLoader {
    /// Load a plugin and extract its metadata
    fn load(&self, path: &Path) -> Result<PluginMetadata, LoadError>;
    
    /// Check if this loader can handle the given path
    fn can_load(&self, path: &Path) -> bool;
}
```

#### VST2 Loader

Uses the `vst` crate (vst-rs) to load VST2 plugins:

```rust
pub struct Vst2Loader;

impl PluginLoader for Vst2Loader {
    fn load(&self, path: &Path) -> Result<PluginMetadata, LoadError> {
        // 1. Create minimal host implementation
        // 2. Load plugin via PluginLoader::load()
        // 3. Call instance.get_info() to extract metadata
        // 4. Determine instrument vs effect from category
        // 5. Close plugin immediately
        // 6. Return PluginMetadata
    }
    
    fn can_load(&self, path: &Path) -> bool {
        match path.extension() {
            Some(ext) => ext == "dll" || ext == "so" || ext == "vst",
            None => false,
        }
    }
}
```

#### VST3 Loader

Uses the `vst3` crate for VST3 plugin loading:

```rust
pub struct Vst3Loader;

impl PluginLoader for Vst3Loader {
    fn load(&self, path: &Path) -> Result<PluginMetadata, LoadError> {
        // 1. Load the VST3 module from the bundle
        // 2. Get the plugin factory
        // 3. Enumerate class info to find audio processors
        // 4. Extract name, vendor, version, category from class info
        // 5. Generate dev_identifier from class ID (GUID)
        // 6. Close module immediately
        // 7. Return PluginMetadata
    }
    
    fn can_load(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| ext == "vst3")
    }
}
```

### Main Process Integration

#### Subprocess Spawner

```rust
pub struct VstParserSpawner {
    parser_path: PathBuf,
    timeout: Duration,
}

impl VstParserSpawner {
    /// Spawn the parser to scan a single plugin
    pub async fn scan_plugin(&self, plugin_path: &Path) -> Result<ScanResult, SpawnError>;
    
    /// Spawn the parser to discover plugins in a directory
    pub async fn discover_plugins(&self, dir: &Path) -> Result<Vec<ScanResult>, SpawnError>;
}
```

#### Database Updater

```rust
impl LiveSetDatabase {
    /// Insert or update a plugin from scan results
    pub fn upsert_plugin_from_scan(&mut self, result: &ScanResult) -> Result<(), DatabaseError>;
    
    /// Batch insert/update plugins from scan results
    pub fn batch_upsert_plugins(&mut self, results: &[ScanResult]) -> Result<BatchResult, DatabaseError>;
}
```

## Data Models

### PluginMetadata (Parser Output)

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin display name
    pub name: String,
    
    /// Unique developer identifier (used as database key)
    pub dev_identifier: String,
    
    /// Plugin format (VST2Instrument, VST2AudioFx, VST3Instrument, VST3AudioFx)
    pub format: String,
    
    /// Vendor/manufacturer name
    pub vendor: Option<String>,
    
    /// Version string
    pub version: Option<String>,
    
    /// SDK version (VST3 only)
    pub sdk_version: Option<String>,
    
    /// Original file path
    pub path: String,
}
```

### ScanResult (JSON Output)

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ScanResult {
    #[serde(rename = "success")]
    Success {
        plugin: PluginMetadata,
    },
    #[serde(rename = "error")]
    Error {
        path: String,
        error: String,
        error_type: ErrorType,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorType {
    FileNotFound,
    InvalidFormat,
    LoadFailed,
    Timeout,
    Crashed,
}
```

### Example JSON Output

Success case:
```json
{
  "status": "success",
  "plugin": {
    "name": "Serum",
    "dev_identifier": "com.xferrecords.serum",
    "format": "VST3Instrument",
    "vendor": "Xfer Records",
    "version": "1.35b1",
    "sdk_version": "3.7.4",
    "path": "C:\\Program Files\\Common Files\\VST3\\Serum.vst3"
  }
}
```

Error case:
```json
{
  "status": "error",
  "path": "C:\\VST\\broken.dll",
  "error": "Failed to load plugin: invalid PE header",
  "error_type": "LoadFailed"
}
```

### Dev Identifier Generation

For VST3 plugins, the dev_identifier is derived from the class ID (GUID):
```rust
fn generate_vst3_dev_identifier(class_id: &[u8; 16]) -> String {
    // Format: "vst3:{GUID}" where GUID is lowercase hex
    format!("vst3:{}", hex::encode(class_id))
}
```

For VST2 plugins, the dev_identifier uses the unique ID:
```rust
fn generate_vst2_dev_identifier(unique_id: i32, name: &str) -> String {
    // Format: "vst2:{unique_id}:{sanitized_name}"
    let sanitized = name.to_lowercase().replace(' ', "_");
    format!("vst2:{}:{}", unique_id, sanitized)
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*



### Property 1: Subprocess Isolation

*For any* plugin that causes a crash during loading, the main process SHALL continue operating normally and receive an error result indicating the crash.

**Validates: Requirements 1.2, 8.3**

### Property 2: Timeout Enforcement

*For any* timeout value T and any plugin that takes longer than T to load, the subprocess SHALL be terminated and a timeout error SHALL be returned within T + epsilon seconds (where epsilon is a small grace period for process cleanup).

**Validates: Requirements 1.3, 1.4**

### Property 3: JSON Output Validity

*For any* invocation of the VST_Parser (success or failure), the stdout output SHALL be valid JSON that can be parsed by serde_json without error.

**Validates: Requirements 4.1, 4.4, 4.5, 6.3**

### Property 4: Format Auto-Detection

*For any* plugin path with a recognized extension (.dll, .so, .vst, .vst3), when no format hint is provided, the detected format SHALL match the expected format for that extension.

**Validates: Requirements 5.3**

### Property 5: Exit Code Correctness

*For any* successful plugin scan, the exit code SHALL be 0. *For any* failed plugin scan, the exit code SHALL be non-zero.

**Validates: Requirements 5.4**

### Property 6: Database Upsert Idempotence

*For any* plugin with a given dev_identifier, scanning and inserting it into the database multiple times SHALL result in exactly one record with that dev_identifier, with the most recent metadata.

**Validates: Requirements 7.2, 7.4**

### Property 7: Installed Flag Correctness

*For any* successfully scanned plugin that is inserted into the database, the installed field SHALL be set to true.

**Validates: Requirements 7.3**

### Property 8: Error Type Correctness

*For any* non-existent file path, the error type SHALL be FileNotFound. *For any* file that exists but is not a valid plugin, the error type SHALL be InvalidFormat or LoadFailed.

**Validates: Requirements 8.1, 8.2**

### Property 9: Partial Failure Handling

*For any* batch scan operation where some plugins succeed and some fail, all successful results SHALL be stored in the database regardless of the failures.

**Validates: Requirements 7.5**

### Property 10: JSON Round-Trip Compatibility

*For any* successful scan result, the JSON output SHALL be deserializable into a structure that can be converted to the existing Plugin model without data loss for required fields.

**Validates: Requirements 4.2, 4.3, 4.6**

## Error Handling

### Parser Error Types

| Error Type | Condition | Exit Code |
|------------|-----------|-----------|
| FileNotFound | Plugin path does not exist | 1 |
| InvalidFormat | File exists but is not a valid VST plugin | 2 |
| LoadFailed | Plugin loading failed (crash, missing deps) | 3 |
| Timeout | Plugin loading exceeded timeout | 4 |
| InvalidArguments | CLI arguments are invalid | 5 |

### Main Process Error Handling

```rust
pub enum SpawnError {
    /// Parser binary not found
    ParserNotFound(PathBuf),
    /// Failed to spawn subprocess
    SpawnFailed(std::io::Error),
    /// Subprocess timed out
    Timeout { path: PathBuf, timeout: Duration },
    /// Subprocess crashed
    Crashed { path: PathBuf, exit_code: Option<i32> },
    /// Failed to parse JSON output
    InvalidOutput { path: PathBuf, output: String },
}
```

### Error Recovery Strategy

1. **Timeout**: Kill subprocess, return timeout error, continue with next plugin
2. **Crash**: Detect non-zero exit without valid JSON, return crash error, continue
3. **Invalid Output**: Log malformed output, return parse error, continue
4. **Partial Failures**: Store successful results, aggregate errors for reporting

## Testing Strategy

### Unit Tests

Unit tests focus on specific components in isolation:

1. **CLI Argument Parsing**: Test that arguments are parsed correctly
2. **Format Detection**: Test extension-based format detection
3. **JSON Serialization**: Test that PluginMetadata serializes correctly
4. **Dev Identifier Generation**: Test identifier generation for VST2/VST3

### Property-Based Tests

Property-based tests verify universal properties across many inputs. Each test runs minimum 100 iterations.

1. **JSON Validity Property**: Generate random PluginMetadata, serialize, verify parseable
2. **Format Detection Property**: Generate paths with various extensions, verify correct detection
3. **Exit Code Property**: Generate success/failure scenarios, verify exit codes
4. **Upsert Idempotence Property**: Generate plugins, insert multiple times, verify single record

### Integration Tests

1. **End-to-End Scan**: Scan a known test plugin, verify output
2. **Timeout Handling**: Create slow-loading mock, verify timeout
3. **Crash Handling**: Create crashing mock, verify main process survives
4. **Directory Discovery**: Create directory with plugins, verify all found

### Test Plugin Fixtures

For integration testing, we'll use:
- A minimal valid VST3 plugin (can be built from VST3 SDK examples)
- A minimal valid VST2 plugin (if licensing permits)
- Invalid/corrupted plugin files for error testing
- Mock plugins that simulate slow loading or crashes

### Property-Based Testing Framework

Use `proptest` crate for property-based testing:

```toml
[dev-dependencies]
proptest = "1.4"
```

## Project Structure

```
seula/
├── Cargo.toml              # Workspace root
├── crates/
│   └── vst-parser/
│       ├── Cargo.toml      # Parser binary crate
│       └── src/
│           ├── main.rs     # CLI entry point
│           ├── lib.rs      # Library exports
│           ├── cli.rs      # Argument parsing
│           ├── loader/
│           │   ├── mod.rs
│           │   ├── vst2.rs # VST2 loading
│           │   └── vst3.rs # VST3 loading
│           ├── models.rs   # PluginMetadata, ScanResult
│           └── discovery.rs # Directory scanning
├── src/
│   ├── scan/
│   │   ├── mod.rs
│   │   ├── vst_scanner.rs  # NEW: Subprocess spawner
│   │   └── ...
│   └── ...
└── ...
```

## Dependencies

### vst-parser crate

```toml
[package]
name = "vst-parser"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"
env_logger = "0.11"

# VST2 support (deprecated but still widely used)
vst = "0.3"

# VST3 support
vst3 = "0.3"

# Platform-specific dynamic loading
libloading = "0.8"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.52", features = ["Win32_Foundation", "Win32_System_LibraryLoader"] }
```

### Main crate additions

```toml
[dependencies]
# ... existing deps ...

[dev-dependencies]
proptest = "1.4"
```

## Platform-Specific Considerations

### Windows

- VST2 plugins: `.dll` files
- VST3 plugins: `.vst3` bundles (folders with `Contents/x86_64-win/*.vst3`)
- Default paths:
  - `C:\Program Files\Common Files\VST3\`
  - `C:\Program Files\VSTPlugins\`
  - `C:\Program Files (x86)\VSTPlugins\`

### macOS

- VST2 plugins: `.vst` bundles
- VST3 plugins: `.vst3` bundles
- Default paths:
  - `/Library/Audio/Plug-Ins/VST3/`
  - `/Library/Audio/Plug-Ins/VST/`
  - `~/Library/Audio/Plug-Ins/VST3/`
  - `~/Library/Audio/Plug-Ins/VST/`

### Linux

- VST2 plugins: `.so` files
- VST3 plugins: `.vst3` bundles
- Default paths:
  - `/usr/lib/vst3/`
  - `/usr/local/lib/vst3/`
  - `~/.vst3/`

## Security Considerations

1. **Path Validation**: Validate plugin paths to prevent directory traversal
2. **Subprocess Isolation**: Parser runs with minimal privileges
3. **Timeout Enforcement**: Prevent DoS from malicious plugins
4. **Output Sanitization**: Validate JSON output before parsing
