# Seula - Ableton Live Project Manager

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [CLI Commands](#cli-commands)
  - [Basic Usage](#basic-usage)
  - [Project Management](#project-management)
  - [Sample Management](#sample-management)
  - [Collection Management](#collection-management)
  - [Tag Management](#tag-management)
  - [Task Management](#task-management)
  - [Search and Discovery](#search-and-discovery)
  - [System Information](#system-information)
  - [Configuration Management](#configuration-management)
  - [Output Formats](#output-formats)
  - [Advanced Usage Examples](#advanced-usage-examples)
  - [Scripting and Automation](#scripting-and-automation)
- [CLI Summary](#cli-summary)
- [Performance](#performance)
- [Contributing](#contributing)

## Overview

A high-performance application for indexing, searching, and managing Ableton Live projects. This tool provides the fastest Ableton Live set parser available, offering comprehensive project analysis, search, and organization capabilities through both a powerful CLI and a clean gRPC API.

The application is now feature-complete with two modes of operation:
- **System Tray Mode** - Runs silently in the background with gRPC API access
- **CLI Mode** - Full-featured command-line interface for direct project management

## Features

### Core Features

- **Extremely fast scanning and parsing** of Ableton Live Set (.als) files (~160-270 MB/s)
- **Dual operation modes**:
  - **System tray application** - runs silently in the background with minimal system impact
  - **Command-line interface** - comprehensive CLI with 33+ commands for direct project management
- **Multiple interfaces**:
  - **gRPC API** for remote access and integration with any client application
  - **CLI commands** with table/JSON/CSV output for automation and scripting
- **Comprehensive project data extraction**:
    - Tempo
    - Ableton version
    - Time signature
    - Length (bars)
    - Plugins used
    - Samples used (paths)
    - Key + scale
    - Estimated duration
- **Plugin + Sample validation** - per project, check which samples/plugins are present on the system
- **5NF SQLite database** for storing project information
- **FTS5 based search engine** with operators:
    - `plugin:serum` - search by plugin name
    - `bpm:128` - search by tempo
    - `key:Cmaj` - search by key signature
    - `missing:true` - find projects with missing plugins
    - And more fuzzy search capabilities across all project data
- **Real-time file watching** with gRPC streaming integration
- **Notes** - descriptions for each project
- **Tags** - tag projects for categorization (e.g., artists, genres)
- **Collections** - for making tracklists; collects to-do lists of contained projects, support for cover art
- **Tasks/To-do lists** per project for mix notes, reminders, and project management
- **Batch operations** - perform bulk actions on multiple projects, tags, collections, and tasks for efficient project management
- **Media management** - upload/download cover art and audio files with storage statistics and cleanup
- **Advanced analytics** - collection-level statistics, task completion trends, and historical analytics
- **Data export** - CSV export of statistics and analytics data
- **Database statistics** with enhanced filtering (date ranges, collections, tags, Ableton versions)
- **Configurable settings** via `config.toml`

### Future Enhancements

- **Version control system** - track project changes over time
- **Audio file integration** - reference and play demo audio files for auditioning
- **Analytics dashboard frontend** - visual dashboard for the existing analytics backend ("Ableton Wrapped" style)
- **macOS support** - currently Windows-focused

## Installation

### Prerequisites

- **Rust 1.70 or higher**
- **SQLite 3.35.0 or higher**
- **Protocol Buffers compiler** (`protoc`)
  - Windows: `choco install protoc` (using Chocolatey)

### Building from source

1. Clone the repository:
```bash
git clone <repository-url>
cd seula
```

2. Build the project:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run --release
```

The application will start in system tray mode by default, with the gRPC server running on `localhost:50051`.

## Configuration

The application is configured via `config.toml`. All settings are optional with sensible defaults:

```toml
# Project directories to scan
paths = [
    '{USER_HOME}/Documents/Ableton Live Projects',
    '{USER_HOME}/Music/Ableton Projects'
]

# Database location (optional - defaults to executable directory)
database_path = ""

# Ableton Live database directory for plugin detection
live_database_dir = '{USER_HOME}/AppData/Local/Ableton/Live Database'

# gRPC server port (default: 50051)
grpc_port = 50051

# Log level: error, warn, info, debug, trace (default: info)
log_level = "info"
```

### Configuration Options

- **`paths`** - Array of project directories to scan
- **`database_path`** - SQLite database location (leave empty for executable directory)
- **`live_database_dir`** - Ableton Live's database directory for plugin detection
- **`grpc_port`** - Port for the gRPC server (default: 50051)
- **`log_level`** - Logging verbosity level (default: "info")

The `{USER_HOME}` placeholder will be automatically replaced with your user directory.

## Usage

Seula operates in two modes: **System Tray Mode** for background operation with gRPC API access, and **CLI Mode** for direct command-line project management.

### System Tray Mode (Default)

```bash
# Start as system tray application
./seula.exe

# Or with cargo
cargo run --release
```

The application will:
1. Load configuration from `config.toml`
2. Initialize the SQLite database
3. Start the gRPC server on port 50051
4. Run silently in the system tray

Right-click the tray icon to quit the application. The gRPC API will be available at `localhost:50051` for client applications.

### CLI Mode

```bash
# Start in CLI mode (shows logs in terminal)
./seula.exe --cli

# Or with cargo
cargo run --release -- --cli
```

Use CLI mode for debugging or when you want to see log output directly. The log level is configurable in `config.toml`.

## CLI Commands

Seula provides a comprehensive command-line interface for managing your Ableton Live projects. All commands support multiple output formats (table, JSON, CSV) and can be used for automation and scripting.

### Basic Usage

```bash
# Show help
seula --help

# Use specific output format
seula project list --format json
seula sample stats --format csv

# Disable colored output
seula project list --no-color
```

### Project Management

#### List Projects
```bash
# List all active projects
seula project list

# Show deleted projects
seula project list --deleted

# Limit results with pagination
seula project list --limit 20 --offset 40
```

#### Project Details
```bash
# Show detailed project information
seula project show <project-id>

# Update project information
seula project update <project-id> --name "New Name" --notes "Updated notes"

# Delete a project (marks as inactive)
seula project delete <project-id>

# Restore a deleted project
seula project restore <project-id>

# Rescan a specific project
seula project rescan <project-id>

# Show project statistics
seula project stats
```

### Sample Management

#### List and Search Samples
```bash
# List all samples with pagination
seula sample list --limit 50 --offset 0

# Search samples by name or path
seula sample search "kick drum" --limit 20

# Show sample statistics and analytics
seula sample stats

# Check sample file presence
seula sample check-presence
```

### Collection Management

Collections allow you to organize projects into groups for better project management.

#### Basic Operations
```bash
# List all collections
seula collection list

# Show collection details with projects
seula collection show <collection-id>

# Create a new collection
seula collection create "My Album" --description "Songs for my new album"

# Add project to collection
seula collection add <collection-id> <project-id>

# Remove project from collection
seula collection remove <collection-id> <project-id>
```

### Tag Management

Tags provide flexible categorization for your projects.

#### Tag Operations
```bash
# List all tags with usage statistics
seula tag list

# Create a new tag
seula tag create "Deep House" --color "#FF5733"

# Assign tag to project
seula tag assign <project-id> <tag-id>

# Remove tag from project
seula tag remove <project-id> <tag-id>

# Search projects by tag
seula tag search "Deep House"
```

### Task Management

Track to-do items and notes for your projects.

#### Task Operations
```bash
# List tasks for a specific project
seula task list --project-id <project-id>

# List all completed tasks
seula task list --project-id <project-id> --completed

# Create a new task
seula task create <project-id> "Fix the kick drum" --priority 5

# Mark task as complete
seula task complete <task-id>

# Delete a task
seula task delete <task-id>
```

### Search and Discovery

#### Full-Text Search
```bash
# Basic text search
seula search "house music"

# Search with operators
seula search "plugin:serum bpm:128"
seula search "key:Cmaj missing:true"
seula search "tag:techno samples:kick"

# Paginated search results
seula search "ambient" --limit 10 --offset 20
```

#### Project Scanning
```bash
# Scan default configured paths
seula scan

# Scan specific directories
seula scan /path/to/projects /another/path

# Force rescan (ignore timestamps)
seula scan --force /path/to/projects
```

### System Information

#### System Operations
```bash
# Show system information
seula system info

# Show comprehensive system statistics
seula system stats
```

### Configuration Management

#### Configuration Operations
```bash
# Show current configuration
seula config show

# Validate configuration
seula config validate

# Edit configuration file
seula config edit
```

### Output Formats

All commands support multiple output formats:

#### Table Format (Default)
```bash
seula project list
# Displays formatted table with colors
```

#### JSON Format
```bash
seula project list --format json
# Machine-readable JSON output for scripting
```

#### CSV Format
```bash
seula sample stats --format csv
# Spreadsheet-compatible CSV output
```

### Advanced Usage Examples

#### Project Discovery Workflow
```bash
# 1. Scan for new projects
seula scan --force

# 2. Check what was found
seula project stats

# 3. Find projects with missing samples
seula search "missing:true"

# 4. Create collection for problem projects
seula collection create "Needs Fixing"

# 5. Tag projects by genre
seula tag create "Techno"
seula tag assign <project-id> <tag-id>
```

#### Project Organization
```bash
# Create a collection for an album
COLLECTION_ID=$(seula collection create "Summer Album 2024" --format json | jq -r '.id')

# Find all summer-themed projects
seula search "summer" --format json | jq -r '.displayed[].id' | while read PROJECT_ID; do
    seula collection add $COLLECTION_ID $PROJECT_ID
done

# Add tasks for album completion
seula task create <project-id> "Master the track"
seula task create <project-id> "Create artwork"
```

#### Maintenance and Cleanup
```bash
# Check sample presence across all projects
seula sample check-presence

# Review projects with missing plugins
seula search "missing:true"

# Validate configuration
seula config validate

# Get system statistics
seula system stats
```

### Scripting and Automation

The CLI is designed for automation with JSON output:

```bash
#!/bin/bash
# Example: Find and list all techno projects

# Get techno tag ID
TAG_ID=$(seula tag search "techno" --format json | jq -r '.tag_name // empty')

if [ -n "$TAG_ID" ]; then
    # List all techno projects
    seula tag search "techno" --format json | jq -r '.projects[].name'
else
    echo "Techno tag not found"
fi
```

## CLI Summary

The Seula CLI provides **33+ commands** across **9 command groups**:

| Command Group | Commands | Description |
|---------------|----------|-------------|
| `project` | 7 commands | Project lifecycle management (list, show, update, delete, restore, rescan, stats) |
| `sample` | 4 commands | Sample analysis and management (list, search, stats, check-presence) |
| `collection` | 5 commands | Project organization into collections (list, show, create, add, remove) |
| `tag` | 5 commands | Flexible project categorization (list, create, assign, remove, search) |
| `task` | 4 commands | Project task management (list, create, complete, delete) |
| `system` | 2 commands | System information and statistics (info, stats) |
| `config` | 3 commands | Configuration management (show, validate, edit) |
| `search` | 1 command | Full-text search with operators |
| `scan` | 1 command | Project discovery and indexing |

**Key Features:**
- **Multiple Output Formats**: Table (default), JSON, CSV for all commands
- **Colored Output**: Visual indicators for status, success/failure, and data categories
- **Automation Ready**: JSON output perfect for scripting and integration
- **Comprehensive**: Complete CRUD operations for all entities
- **Fast**: Built on the same high-performance engine as the gRPC API
- **User-Friendly**: Intuitive command structure with helpful error messages

### Client Integration

The gRPC service can be integrated with any language that supports gRPC. The protobuf definitions are available in `proto/seula.proto`.

## Deployment

1. **Build the release binary:**
   ```bash
   cargo build --release
   ```

2. **Copy the executable and config:**
   ```
   seula.exe
   config.toml
   ```

3. **Run the application:**
   - Double-click the executable for tray mode
   - Or run from command line: `./seula.exe`

The application will automatically:
- Create the database if it doesn't exist
- Start the gRPC server
- Run in the system tray

You could then create or use any frontend you would like, or manually interact with the server using grpcurl.
I am currently working on a first party frontend.

## Performance

### Scanning Performance Benchmarks

**Cold Scan (First Run - 3,570 projects)**
- **Scanning speed**: 38.6 projects/sec (average), 43.7 projects/sec (peak)
- **Total time**: 92.52 seconds
- **Time per project**: 0.026 seconds
- **Throughput**: 2,315 projects/minute

**Warm Scan (Subsequent Runs - 3,570 projects)**
- **Scanning speed**: 860.7 projects/sec (average), 860.8 projects/sec (peak)
- **Total time**: 4.15 seconds
- **Time per project**: 0.001 seconds
- **Throughput**: 51,645 projects/minute

### System Resources

- **Memory usage**: Minimal - designed for long-running operation
- **Database**: SQLite with FTS5 for fast full-text search
- **Concurrency**: Multi-threaded scanning and processing

### Time Estimates

| Projects | Cold Scan | Warm Scan |
|----------|-----------|-----------|
| 100      | 2.6s      | 0.1s      |
| 500      | 13.0s     | 0.6s      |
| 1000     | 25.9s     | 1.2s      |

*Benchmarks conducted on modern hardware with 3,570 Ableton Live projects*

## Contributing

Contributions are welcome! Feel free to submit a PR, but please open an issue first for large contributions.
