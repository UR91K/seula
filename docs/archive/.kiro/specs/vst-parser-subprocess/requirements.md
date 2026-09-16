# Requirements Document

## Introduction

This document specifies the requirements for a standalone VST plugin parser that runs as a separate subprocess. The parser scans VST2 and VST3 plugins on the system, extracts their metadata, and outputs JSON that can be used by the main Seula process to populate the plugins database. Running as a subprocess isolates the main process from crashes or instability caused by problematic plugins.

## Glossary

- **VST_Parser**: The standalone executable that scans VST plugins and extracts metadata
- **Main_Process**: The primary Seula application that invokes the VST_Parser subprocess
- **Plugin_Metadata**: The structured data extracted from a VST plugin including name, vendor, version, and format
- **VST2**: Virtual Studio Technology version 2 plugin format (.dll on Windows, .vst on macOS)
- **VST3**: Virtual Studio Technology version 3 plugin format (.vst3 bundle)
- **Plugin_Path**: The file system path to a VST plugin file or bundle
- **Scan_Result**: The JSON output from the VST_Parser containing plugin metadata or error information
- **Plugin_Format**: The type classification of a plugin (VST2Instrument, VST2AudioFx, VST3Instrument, VST3AudioFx)

## Requirements

### Requirement 1: Subprocess Isolation

**User Story:** As a system administrator, I want the VST scanner to run in a separate process, so that unstable or crashing plugins do not affect the main application.

#### Acceptance Criteria

1. THE VST_Parser SHALL execute as a standalone binary separate from the Main_Process
2. WHEN the VST_Parser crashes or hangs, THE Main_Process SHALL continue operating normally
3. THE Main_Process SHALL spawn the VST_Parser as a child process with configurable timeout
4. IF the VST_Parser exceeds the timeout, THEN THE Main_Process SHALL terminate the subprocess and report a timeout error

### Requirement 2: VST3 Plugin Scanning

**User Story:** As a music producer, I want to scan VST3 plugins on my system, so that I can see which VST3 instruments and effects are available.

#### Acceptance Criteria

1. WHEN given a path to a VST3 plugin bundle, THE VST_Parser SHALL open the plugin and extract its metadata
2. THE VST_Parser SHALL extract the plugin name from the VST3 plugin info
3. THE VST_Parser SHALL extract the vendor name from the VST3 plugin info
4. THE VST_Parser SHALL extract the version string from the VST3 plugin info
5. THE VST_Parser SHALL determine whether the plugin is an instrument or audio effect
6. THE VST_Parser SHALL extract the plugin's unique identifier (class ID)
7. WHEN the VST3 plugin cannot be loaded, THE VST_Parser SHALL return an error with a descriptive message
8. THE VST_Parser SHALL immediately close the plugin after extracting metadata to minimize resource usage

### Requirement 3: VST2 Plugin Scanning

**User Story:** As a music producer, I want to scan VST2 plugins on my system, so that I can see which legacy VST2 instruments and effects are available.

#### Acceptance Criteria

1. WHEN given a path to a VST2 plugin file, THE VST_Parser SHALL open the plugin and extract its metadata
2. THE VST_Parser SHALL extract the plugin name from the VST2 plugin
3. THE VST_Parser SHALL extract the vendor name from the VST2 plugin if available
4. THE VST_Parser SHALL extract the version number from the VST2 plugin
5. THE VST_Parser SHALL determine whether the plugin is an instrument (VSTi) or audio effect
6. THE VST_Parser SHALL extract the plugin's unique ID
7. WHEN the VST2 plugin cannot be loaded, THE VST_Parser SHALL return an error with a descriptive message
8. THE VST_Parser SHALL immediately close the plugin after extracting metadata to minimize resource usage

### Requirement 4: JSON Output Format

**User Story:** As a developer, I want the parser to output structured JSON, so that the main process can easily parse and use the plugin metadata.

#### Acceptance Criteria

1. THE VST_Parser SHALL output Plugin_Metadata as a JSON string to stdout
2. THE JSON output SHALL include the following fields: name, vendor, version, format, dev_identifier, sdk_version
3. THE JSON output format SHALL be compatible with the existing Plugin model in the Seula database
4. WHEN an error occurs, THE VST_Parser SHALL output a JSON error object with an error field and message
5. THE VST_Parser SHALL output valid JSON that can be parsed by serde_json
6. THE VST_Parser SHALL use the same Plugin_Format values as the existing database (VST2Instrument, VST2AudioFx, VST3Instrument, VST3AudioFx)

### Requirement 5: Command Line Interface

**User Story:** As a developer, I want to invoke the parser with command line arguments, so that I can specify which plugin to scan.

#### Acceptance Criteria

1. THE VST_Parser SHALL accept a plugin path as a command line argument
2. THE VST_Parser SHALL accept an optional format hint argument (vst2 or vst3)
3. WHEN no format hint is provided, THE VST_Parser SHALL auto-detect the format from the file extension
4. THE VST_Parser SHALL return exit code 0 on success and non-zero on failure
5. THE VST_Parser SHALL support a --help flag that displays usage information
6. THE VST_Parser SHALL support a --version flag that displays the version number

### Requirement 6: Plugin Discovery

**User Story:** As a music producer, I want to scan all plugins in standard directories, so that I can build a complete plugin database.

#### Acceptance Criteria

1. THE VST_Parser SHALL support a scan-directory mode that discovers all plugins in a given directory
2. WHEN scanning a directory, THE VST_Parser SHALL recursively search for VST2 and VST3 plugins
3. THE VST_Parser SHALL output a JSON array of Scan_Results when scanning a directory
4. THE VST_Parser SHALL skip files that are not valid VST plugins
5. THE VST_Parser SHALL report progress to stderr when scanning directories

### Requirement 7: Database Integration

**User Story:** As a developer, I want the main process to update the database with scanned plugin data, so that plugins are available for project analysis.

#### Acceptance Criteria

1. THE Main_Process SHALL parse the JSON output from VST_Parser and create Plugin records
2. THE Main_Process SHALL insert or update plugins in the database using the dev_identifier as the unique key
3. THE Main_Process SHALL set the installed field to true for successfully scanned plugins
4. WHEN a plugin already exists in the database, THE Main_Process SHALL update its metadata with the new scan data
5. THE Main_Process SHALL handle partial scan failures gracefully, storing successful results even if some plugins fail

### Requirement 8: Error Handling

**User Story:** As a developer, I want comprehensive error handling, so that I can diagnose issues with plugin scanning.

#### Acceptance Criteria

1. IF a plugin file does not exist, THEN THE VST_Parser SHALL return an error indicating the file was not found
2. IF a plugin file is corrupted or invalid, THEN THE VST_Parser SHALL return an error indicating the plugin could not be loaded
3. IF the plugin crashes during scanning, THEN THE Main_Process SHALL detect the crash and report it
4. THE VST_Parser SHALL log detailed error information to stderr for debugging
5. THE Main_Process SHALL store scan errors in a log for user review
