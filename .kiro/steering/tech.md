# Technology Stack

## Core Technologies

- **Language**: Rust 2021 edition
- **Database**: SQLite 3.35.0+ with FTS5 full-text search
- **API**: gRPC with Protocol Buffers (tonic/prost)
- **Async Runtime**: Tokio for async operations
- **CLI Framework**: Clap v4 with derive features
- **Configuration**: TOML format with serde

## Key Dependencies

### Core Functionality
- `rusqlite` - SQLite database interface with bundled SQLite
- `elementtree` + `quick-xml` - XML parsing for .als files
- `flate2` + `zune-inflate` - Gzip decompression for compressed projects
- `walkdir` - Directory traversal for project scanning
- `notify` - File system watching
- `uuid` - Unique identifier generation
- `chrono` - Date/time handling

### gRPC & Networking
- `tonic` - gRPC server implementation
- `prost` - Protocol Buffer code generation
- `tokio-stream` - Async streaming support

### CLI & Output
- `clap` - Command-line argument parsing
- `comfy-table` - Table formatting for CLI output
- `colored` - Terminal color support
- `csv` - CSV output format
- `serde_json` - JSON output format
- `rustyline` - Interactive CLI with readline support

### System Integration
- `tray-icon` - System tray functionality
- `dirs` - Standard directory paths
- `sys-info` - System information gathering
- `windows-sys` + `winreg` - Windows-specific functionality

## Build System

### Protocol Buffers
- Uses `tonic-build` in `build.rs` to compile `.proto` files
- Proto files located in `proto/` directory with service separation
- Generates both server and client code (server-only in production)

### Common Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run with CLI mode
cargo run --release -- --cli

# Run tests
cargo test

# Run specific test module
cargo test config_tests

# Run tests with output
cargo test -- --nocapture

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Testing Commands

```bash
# Run all tests
cargo test

# Run tests sequentially (for shared resource tests)
cargo test -- --test-threads=1

# Run specific test file
cargo test --test config_tests

# Run integration tests only
cargo test --test integration_tests

# Run with debug output
RUST_LOG=debug cargo test
```

## Project Structure Conventions

- `src/` - Main source code
- `proto/` - Protocol Buffer definitions
- `tests/` - Integration and unit tests
- `docs/` - Documentation and specifications
- `target/` - Build artifacts (gitignored)

## Configuration

Uses `config.toml` with environment variable overrides:
- `STUDIO_PROJECT_MANAGER_GRPC_PORT` - Override gRPC port
- `STUDIO_PROJECT_MANAGER_LOG_LEVEL` - Override log level
- `STUDIO_PROJECT_MANAGER_DATABASE_PATH` - Override database path
- `STUDIO_PROJECT_MANAGER_CONFIG` - Override config file location

## Performance Characteristics

- **Scanning Speed**: 160-270 MB/s for .als file parsing
- **Concurrency**: Multi-threaded scanning with configurable thread pools
- **Memory**: Designed for long-running operation with minimal memory usage
- **Database**: SQLite with WAL mode for concurrent read/write access