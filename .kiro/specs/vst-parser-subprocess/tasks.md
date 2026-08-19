# Implementation Plan: VST Parser Subprocess

## Overview

This plan implements a standalone VST plugin parser that runs as a subprocess, scanning VST2 and VST3 plugins and outputting JSON metadata compatible with Seula's existing plugin database.

## Tasks

- [x] 1. Set up workspace structure and vst-parser crate
  - Create `crates/vst-parser/` directory structure
  - Configure Cargo workspace in root `Cargo.toml`
  - Create `crates/vst-parser/Cargo.toml` with dependencies
  - Create basic `src/main.rs` and `src/lib.rs` files
  - _Requirements: 1.1, 5.1_

- [ ] 2. Implement core data models
  - [x] 2.1 Create PluginMetadata and ScanResult types
    - Define `PluginMetadata` struct with serde serialization
    - Define `ScanResult` enum with success/error variants
    - Define `ErrorType` enum for error classification
    - Ensure format field uses existing PluginFormat values
    - _Requirements: 4.1, 4.2, 4.3, 4.6_

  - [x] 2.2 Write property test for JSON round-trip

    - **Property 10: JSON Round-Trip Compatibility**
    - **Validates: Requirements 4.2, 4.3, 4.6**

- [ ] 3. Implement CLI argument parsing
  - [ ] 3.1 Create CLI module with clap derive
    - Define `Cli` struct with subcommands
    - Implement `scan` subcommand with path and format options
    - Implement `discover` subcommand with directory and recursive options
    - Add `--help` and `--version` support
    - _Requirements: 5.1, 5.2, 5.5, 5.6_

  - [ ]* 3.2 Write unit tests for CLI parsing
    - Test argument parsing for various inputs
    - Test help and version flags
    - _Requirements: 5.5, 5.6_

- [ ] 4. Implement format detection
  - [ ] 4.1 Create format detection module
    - Implement extension-based format detection
    - Handle `.dll`, `.so`, `.vst`, `.vst3` extensions
    - Support format hint override from CLI
    - _Requirements: 5.3_

  - [ ]* 4.2 Write property test for format auto-detection
    - **Property 4: Format Auto-Detection**
    - **Validates: Requirements 5.3**

- [ ] 5. Implement VST2 plugin loader
  - [ ] 5.1 Create VST2 loader using vst crate
    - Implement minimal Host trait for plugin loading
    - Load plugin via PluginLoader::load()
    - Extract name, vendor, version from plugin info
    - Determine instrument vs effect from category
    - Generate dev_identifier from unique_id
    - Close plugin immediately after extraction
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.8_

  - [ ] 5.2 Implement VST2 error handling
    - Handle file not found errors
    - Handle invalid plugin format errors
    - Handle plugin load failures
    - Return appropriate ErrorType values
    - _Requirements: 3.7, 8.1, 8.2_

- [ ] 6. Implement VST3 plugin loader
  - [ ] 6.1 Create VST3 loader using vst3 crate
    - Load VST3 module from bundle path
    - Get plugin factory and enumerate classes
    - Extract name, vendor, version, SDK version
    - Determine instrument vs effect from category
    - Generate dev_identifier from class ID (GUID)
    - Close module immediately after extraction
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.8_

  - [ ] 6.2 Implement VST3 error handling
    - Handle file not found errors
    - Handle invalid bundle structure errors
    - Handle plugin load failures
    - Return appropriate ErrorType values
    - _Requirements: 2.7, 8.1, 8.2_

- [ ] 7. Checkpoint - Verify plugin loaders work
  - Ensure VST2 and VST3 loaders compile
  - Test with sample plugins if available
  - Ensure all tests pass, ask the user if questions arise

- [ ] 8. Implement directory discovery
  - [ ] 8.1 Create discovery module
    - Implement recursive directory scanning
    - Filter for VST2 and VST3 file extensions
    - Skip non-plugin files
    - Output progress to stderr when enabled
    - _Requirements: 6.1, 6.2, 6.4, 6.5_

  - [ ]* 8.2 Write property test for directory scanning
    - **Property (combined)**: For any directory with plugins, all plugins should be discovered
    - **Validates: Requirements 6.2, 6.4**

- [ ] 9. Implement main entry point and JSON output
  - [ ] 9.1 Wire CLI to loaders and output JSON
    - Parse CLI arguments
    - Route to appropriate loader based on format
    - Serialize result to JSON and print to stdout
    - Set appropriate exit codes
    - _Requirements: 4.1, 4.4, 4.5, 5.4, 6.3_

  - [ ]* 9.2 Write property test for JSON output validity
    - **Property 3: JSON Output Validity**
    - **Validates: Requirements 4.1, 4.4, 4.5, 6.3**

  - [ ]* 9.3 Write property test for exit code correctness
    - **Property 5: Exit Code Correctness**
    - **Validates: Requirements 5.4**

- [ ] 10. Checkpoint - Verify standalone parser works
  - Build vst-parser binary
  - Test CLI with --help and --version
  - Test scanning a plugin path (if available)
  - Ensure all tests pass, ask the user if questions arise

- [ ] 11. Implement subprocess spawner in main crate
  - [ ] 11.1 Create VstParserSpawner struct
    - Implement subprocess spawning with Command
    - Implement configurable timeout handling
    - Parse JSON output from subprocess stdout
    - Handle subprocess crashes and timeouts
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 8.3_

  - [ ]* 11.2 Write property test for subprocess isolation
    - **Property 1: Subprocess Isolation**
    - **Validates: Requirements 1.2, 8.3**

  - [ ]* 11.3 Write property test for timeout enforcement
    - **Property 2: Timeout Enforcement**
    - **Validates: Requirements 1.3, 1.4**

- [ ] 12. Implement database integration
  - [ ] 12.1 Add upsert_plugin_from_scan method
    - Parse ScanResult JSON into Plugin model
    - Insert or update plugin using dev_identifier as key
    - Set installed field to true for successful scans
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

  - [ ] 12.2 Add batch_upsert_plugins method
    - Process multiple scan results
    - Handle partial failures gracefully
    - Return summary of successes and failures
    - _Requirements: 7.5_

  - [ ]* 12.3 Write property test for database upsert idempotence
    - **Property 6: Database Upsert Idempotence**
    - **Validates: Requirements 7.2, 7.4**

  - [ ]* 12.4 Write property test for installed flag correctness
    - **Property 7: Installed Flag Correctness**
    - **Validates: Requirements 7.3**

  - [ ]* 12.5 Write property test for partial failure handling
    - **Property 9: Partial Failure Handling**
    - **Validates: Requirements 7.5**

- [ ] 13. Implement error type handling
  - [ ] 13.1 Add error type mapping in spawner
    - Map subprocess exit codes to error types
    - Map JSON error responses to SpawnError variants
    - Log detailed errors for debugging
    - _Requirements: 8.1, 8.2, 8.4, 8.5_

  - [ ]* 13.2 Write property test for error type correctness
    - **Property 8: Error Type Correctness**
    - **Validates: Requirements 8.1, 8.2**

- [ ] 14. Add gRPC service integration
  - [ ] 14.1 Add scan plugin RPC to PluginService
    - Add ScanPluginRequest/Response to plugins.proto
    - Implement handler that uses VstParserSpawner
    - Return scan results via gRPC
    - _Requirements: 7.1_

  - [ ] 14.2 Add scan directory RPC to PluginService
    - Add ScanPluginDirectoryRequest/Response to plugins.proto
    - Implement handler for directory scanning
    - Stream results for large directories
    - _Requirements: 6.1_

- [ ] 15. Add CLI commands for plugin scanning
  - [ ] 15.1 Add scan-vst command to CLI
    - Add subcommand to scan a single VST plugin
    - Display results in table/JSON/CSV format
    - _Requirements: 5.1_

  - [ ] 15.2 Add scan-vst-directory command to CLI
    - Add subcommand to scan a directory of plugins
    - Display progress and results
    - _Requirements: 6.1, 6.5_

- [ ] 16. Final checkpoint - Full integration testing
  - Ensure all tests pass
  - Test end-to-end workflow: scan plugin → update database → query plugin
  - Verify subprocess isolation with crash test
  - Ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- The vst-parser binary must be built and available in PATH or a known location for the main process to spawn it
