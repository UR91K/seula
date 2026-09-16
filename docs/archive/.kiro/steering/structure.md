# Project Structure

## Source Code Organization

### Core Modules (`src/`)

- **`main.rs`** - Application entry point with CLI/tray mode selection
- **`lib.rs`** - Library interface and main processing functions
- **`models.rs`** - Core data structures and enums
- **`error.rs`** - Centralized error handling types

### Feature Modules

- **`config/`** - Configuration loading, validation, and management
  - `mod.rs` - Main config struct and global CONFIG instance
  - `loader.rs` - File loading and path resolution
  - `validator.rs` - Configuration validation logic
  - `defaults.rs` - Default values and constants
  - `paths.rs` - Path handling utilities
  - `windows_paths.rs` - Windows-specific path logic

- **`database/`** - SQLite database operations
  - `core.rs` - Database initialization and connection management
  - `models.rs` - Database-specific data structures
  - `batch.rs` - Batch insert operations for performance
  - Individual modules for each entity: `projects.rs`, `samples.rs`, `plugins.rs`, `collections.rs`, `tags.rs`, `tasks.rs`, `media.rs`, `search.rs`, `stats.rs`

- **`scan/`** - Project discovery and parsing
  - `project_scanner.rs` - Directory scanning for .als files
  - `parser.rs` - Individual .als file parsing logic
  - `parallel.rs` - Multi-threaded parsing coordination

- **`grpc/`** - gRPC server and API handlers
  - `server.rs` - Main gRPC server setup and service registration
  - `handlers/` - Individual service implementations
    - One handler per service: `projects.rs`, `collections.rs`, `tags.rs`, etc.
    - `utils.rs` - Common handler utilities

- **`cli/`** - Command-line interface
  - `mod.rs` - CLI argument parsing and main CLI struct
  - `interactive.rs` - Interactive CLI mode
  - `output.rs` - Output formatting (table/JSON/CSV)
  - `commands/` - Individual command implementations
    - One module per command group: `project.rs`, `collection.rs`, `tag.rs`, etc.

- **`media/`** - Media file storage and management
  - `storage.rs` - File storage operations
  - `validation.rs` - File validation and size limits
  - `error.rs` - Media-specific error types

- **`watcher/`** - File system monitoring
  - `file_watcher.rs` - File system event handling

- **`utils/`** - Utility functions
  - `metadata.rs` - File metadata extraction
  - `plugins.rs` - Plugin-related utilities
  - `samples.rs` - Sample-related utilities
  - `time_signature.rs` - Time signature parsing
  - `macos_formats.rs` - macOS-specific format handling

### Supporting Files

- **`ableton_db.rs`** - Ableton Live database integration
- **`live_set.rs`** - Core LiveSet struct and parsing logic
- **`tray.rs`** - System tray application implementation

## Protocol Buffers (`proto/`)

- **`common.proto`** - Shared message types and enums
- **`services/`** - Individual service definitions
  - One .proto file per service: `projects.proto`, `collections.proto`, etc.
  - Services follow consistent CRUD patterns where applicable

## Testing Structure (`tests/`)

### Test Organization
- **Integration tests** - One file per major feature area
- **Module-specific tests** - `tests/{module}/` directories for complex modules
- **Common utilities** - `tests/common/` for shared test helpers

### Key Test Files
- **`config_tests.rs`** - Configuration loading and validation
- **`database_tests.rs`** - Database operations
- **`scan_tests.rs`** - Project scanning and parsing
- **`grpc_tests.rs`** - gRPC service endpoints
- **`integration_tests.rs`** - End-to-end workflows

### Test Patterns
- Use `tests/common/builders.rs` for test data creation
- Use `tests/grpc/server_setup.rs` for gRPC test setup
- Sequential execution for tests with shared resources (environment variables, files)
- In-memory databases for isolated testing

## Documentation (`docs/`)

- **`codebase_patterns.md`** - Development patterns and conventions
- **`generalisation_plan.md`** - Architecture evolution plans
- **`plugin_scanner_plan.md`** - Plugin scanning implementation details

## Configuration Files

- **`Cargo.toml`** - Rust package configuration and dependencies
- **`build.rs`** - Build script for Protocol Buffer compilation
- **`config.toml`** - Runtime configuration (created by user)

## Naming Conventions

### Files and Modules
- Snake_case for file names and module names
- One primary struct per file when possible
- Handler modules named after their service (e.g., `projects.rs` for ProjectService)

### Structs and Types
- PascalCase for struct names
- Descriptive names that indicate purpose (e.g., `LiveSetDatabase`, `ProjectPathScanner`)
- `*Handler` suffix for gRPC service handlers
- `*Error` suffix for error types

### Functions and Variables
- Snake_case for function and variable names
- Descriptive names that indicate action (e.g., `process_projects`, `scan_directory`)
- `get_*` prefix for retrieval functions
- `create_*` prefix for creation functions