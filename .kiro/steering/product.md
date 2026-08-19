# Product Overview

Seula is a high-performance Ableton Live project manager that provides comprehensive indexing, searching, and organization capabilities for music producers.

## Core Purpose

Seula scans Ableton Live project directories, extracts detailed metadata from `.als` files, and stores everything in a searchable SQLite database. It operates as both a system tray application with gRPC API and a full-featured CLI tool.

## Key Features

- **Ultra-fast scanning**: 160-270 MB/s parsing speed with parallel processing
- **Comprehensive metadata extraction**: Tempo, plugins, samples, key signatures, time signatures, project length, Ableton version
- **Full-text search**: FTS5-powered search with operators (`plugin:serum`, `bpm:128`, `key:Cmaj`, `missing:true`)
- **Project organization**: Tags, collections, tasks/notes, and batch operations
- **Plugin/sample validation**: Checks which plugins and samples are present on the system
- **Media management**: Cover art and audio file storage with size limits
- **Real-time file watching**: Monitors project directories for changes
- **Dual interfaces**: System tray mode with gRPC API + comprehensive CLI with 33+ commands

## Target Users

Music producers and audio engineers who work with large collections of Ableton Live projects and need efficient ways to organize, search, and manage their work.

## Architecture

- **Backend**: Rust-based high-performance engine with SQLite database
- **API**: gRPC services for all functionality
- **CLI**: Full-featured command-line interface with table/JSON/CSV output
- **Frontend**: Planned web-based dashboard (see FRONTEND_SPEC.md)