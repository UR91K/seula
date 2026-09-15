# TUI Architecture Analysis: Tray + Separate TUI Process Model

## Current Architecture Analysis

### Current Flow (from `src/main.rs`)
```
seula.exe [no args] → Tray Mode (default)
  ├── Spawn gRPC server in background task
  ├── Run tray app in blocking task
  └── When tray quits → abort gRPC server

seula.exe --cli → CLI Mode
  └── Run gRPC server directly (blocks main thread)
```

### Current Issues & Assumptions to Address

#### 1. **Single Binary Architecture**
**Current State**: One binary that can run as either tray or CLI mode
**Issue**: TUI would need to be embedded in the same binary, making it heavy
**Solution Needed**: Separate TUI binary or new launch mode

#### 2. **gRPC Server Lifecycle**
**Current State**: gRPC server dies when tray application exits
**Issue**: TUI clients would lose connection when tray is closed
**Solution Needed**: Independent server lifecycle management

#### 3. **Configuration Access**
**Current State**: Both tray and CLI modes load CONFIG at startup
**Issue**: TUI needs config access but shouldn't duplicate server startup
**Solution Needed**: Config service via gRPC or shared config file access

#### 4. **Process Communication**
**Current State**: No inter-process communication beyond gRPC
**Issue**: Tray can't launch TUI, no coordination between processes
**Solution Needed**: Process launching and status coordination

## Proposed Architecture: Multi-Process Model

### Process Structure
```
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│   Tray Process      │    │   Server Process    │    │   TUI Process       │
│                     │    │                     │    │                     │
│ studio_mgr.exe      │    │ studio_mgr.exe      │    │ studio_mgr_tui.exe  │
│ (default, no args)  │    │ --server            │    │ (separate binary)   │
│                     │    │                     │    │                     │
│ ┌─────────────────┐ │    │ ┌─────────────────┐ │    │ ┌─────────────────┐ │
│ │ System Tray     │ │    │ │ gRPC Server     │ │    │ │ ratatui TUI     │ │
│ │ - Menu          │ │    │ │ - All Services  │ │    │ │ - All Views     │ │
│ │ - Launch TUI    │ │    │ │ - File Watcher  │ │    │ │ - gRPC Client   │ │
│ │ - Settings      │ │    │ │ - Background    │ │    │ │ - Event Loop    │ │
│ │ - Quit          │ │    │ │   Scanning      │ │    │ │                 │ │
│ └─────────────────┘ │    │ └─────────────────┘ │    │ └─────────────────┘ │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
          │                           │                           │
          │                           │                           │
          ├──── Launch TUI ────────────┼──────────────────────────►│
          │                           │                           │
          └──── gRPC Calls ──────────►│◄──── gRPC Calls ─────────┘
                                      │
                                      │
                              ┌─────────────────────┐
                              │   Config File       │
                              │   config.toml       │
                              │                     │
                              │ - Shared by all     │
                              │ - File watching     │
                              │ - Hot reload        │
                              └─────────────────────┘
```

### Command Line Interface Design

#### Primary Binary: `seula.exe`
```bash
# Default behavior - Start tray application
seula.exe

# Server-only mode - Run gRPC server without tray
seula.exe --server

# Legacy CLI mode - Server only, blocking (for backwards compatibility)
seula.exe --cli

# Help and version
seula.exe --help
seula.exe --version
```

#### TUI Binary: `seula_tui.exe` (or `spm_tui.exe`)
```bash
# Launch TUI (connects to running server)
seula_tui.exe

# Launch TUI with specific server address
seula_tui.exe --server localhost:50051

# Launch TUI and start server if not running
seula_tui.exe --auto-server

# Help and version
seula_tui.exe --help
seula_tui.exe --version
```

## Implementation Plan

### Phase 1: Refactor Main Binary for Multi-Process Support

#### 1.1 Update Command Line Arguments
```rust
// src/main.rs - Updated argument parsing
#[derive(Debug)]
enum RunMode {
    Tray,       // Default: Run tray app + server
    Server,     // New: Run server only
    Cli,        // Legacy: Run server only (blocking)
}

fn parse_args() -> RunMode {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"--server".to_string()) || args.contains(&"-s".to_string()) {
        RunMode::Server
    } else if args.contains(&"--cli".to_string()) || args.contains(&"-c".to_string()) {
        RunMode::Cli
    } else {
        RunMode::Tray
    }
}
```

#### 1.2 Add Server-Only Mode
```rust
// src/main.rs - New server-only mode
async fn run_server_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Seula gRPC Server (server-only mode)");
    
    // Set up signal handling for graceful shutdown
    setup_signal_handlers().await?;
    
    start_grpc_server().await
}
```

#### 1.3 Update Tray to Launch TUI
```rust
// src/tray.rs - Add TUI launching capability
impl TrayApp {
    fn create_menu() -> Menu {
        let menu = Menu::new();
        
        let open_tui = MenuItem::new("Open Interface", true, None);
        let settings = MenuItem::new("Settings", true, None);
        let separator = MenuItem::new("", false, None); // Separator
        let quit = MenuItem::new("Quit", true, None);
        
        menu.append(&open_tui)?;
        menu.append(&settings)?;
        menu.append(&separator)?;
        menu.append(&quit)?;
        
        menu
    }
    
    fn handle_menu_event(&self, event_id: &MenuId) {
        match event_id {
            id if id == &self.open_tui_id => {
                self.launch_tui();
            }
            id if id == &self.settings_id => {
                self.launch_settings();
            }
            id if id == &self.quit_id => {
                self.shutdown();
            }
        }
    }
    
    fn launch_tui(&self) {
        // Launch TUI process
        match std::process::Command::new("seula_tui.exe")
            .spawn() 
        {
            Ok(_) => info!("TUI launched successfully"),
            Err(e) => error!("Failed to launch TUI: {}", e),
        }
    }
}
```

### Phase 2: Create Separate TUI Binary

#### 2.1 New Binary Configuration
```toml
# Cargo.toml - Add TUI binary
[[bin]]
name = "seula"
path = "src/main.rs"

[[bin]]
name = "seula_tui"
path = "src/bin/tui_main.rs"
```

#### 2.2 TUI Main Entry Point
```rust
// src/bin/tui_main.rs
use seula::tui::TuiApp;
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Seula TUI")
        .version("1.0.0")
        .about("Terminal interface for Seula")
        .arg(
            Arg::new("server")
                .long("server")
                .value_name("ADDRESS")
                .help("gRPC server address")
                .default_value("127.0.0.1:50051")
        )
        .arg(
            Arg::new("auto-server")
                .long("auto-server")
                .help("Start server automatically if not running")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    let server_addr = matches.get_one::<String>("server").unwrap();
    let auto_server = matches.get_flag("auto-server");

    // Initialize TUI application
    let mut tui_app = TuiApp::new(server_addr, auto_server).await?;
    
    // Run the TUI
    tui_app.run().await?;
    
    Ok(())
}
```

#### 2.3 TUI Application Structure
```rust
// src/tui/app.rs
use crate::tui::client::GrpcClient;
use ratatui::Terminal;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

pub struct TuiApp {
    client: GrpcClient,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    should_quit: bool,
    current_view: ViewType,
}

impl TuiApp {
    pub async fn new(server_addr: &str, auto_server: bool) -> Result<Self, TuiError> {
        // Try to connect to server
        let client = match GrpcClient::new(server_addr).await {
            Ok(client) => client,
            Err(_) if auto_server => {
                // Launch server and retry
                Self::start_server_if_needed().await?;
                GrpcClient::new(server_addr).await?
            }
            Err(e) => return Err(e.into()),
        };

        // Initialize terminal
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            client,
            terminal,
            should_quit: false,
            current_view: ViewType::Projects,
        })
    }

    async fn start_server_if_needed() -> Result<(), TuiError> {
        // Check if server is already running
        if Self::check_server_running("127.0.0.1:50051").await {
            return Ok(());
        }

        // Start server process
        std::process::Command::new("seula.exe")
            .arg("--server")
            .spawn()
            .map_err(|e| TuiError::ServerLaunch(e))?;

        // Wait for server to be ready
        for _ in 0..30 { // Wait up to 3 seconds
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if Self::check_server_running("127.0.0.1:50051").await {
                return Ok(());
            }
        }

        Err(TuiError::ServerTimeout)
    }
}
```

### Phase 3: Enhanced Process Coordination

#### 3.1 Server Discovery and Health Checking
```rust
// src/tui/client/health.rs
impl GrpcClient {
    pub async fn check_server_health(&mut self) -> Result<bool, GrpcError> {
        // Use gRPC health checking service or system service ping
        match self.system_client.get_server_info(Request::new(())).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    pub async fn wait_for_server(&mut self, timeout_secs: u64) -> Result<(), GrpcError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        
        while start.elapsed() < timeout {
            if self.check_server_health().await? {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        Err(GrpcError::ServerTimeout)
    }
}
```

#### 3.2 Configuration Access Strategy
```rust
// Option 1: Direct file access (simpler)
// src/tui/config.rs
pub struct TuiConfig {
    pub server_addr: String,
    pub auto_reconnect: bool,
    pub theme: String,
}

impl TuiConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // Load from same config.toml file as server
        // Extract only TUI-relevant settings
    }
}

// Option 2: gRPC config service (more robust)
// Use the existing ConfigService to get settings
impl TuiApp {
    async fn load_config(&mut self) -> Result<AppConfig, TuiError> {
        let request = Request::new(GetConfigRequest {});
        let response = self.client.config_client.get_config(request).await?;
        Ok(AppConfig::from(response.into_inner().config))
    }
}
```

## Benefits of This Architecture

### 1. **Separation of Concerns**
- **Tray**: System integration, process management, notifications
- **Server**: Data processing, file watching, gRPC services  
- **TUI**: User interface, interaction, visualization

### 2. **Independent Lifecycles**
- **Server** can run without tray (for servers/headless)
- **TUI** can be opened/closed without affecting server
- **Tray** manages overall system presence

### 3. **Flexibility**
- Multiple TUI instances can connect to same server
- Server can run on different machine (future enhancement)
- Easy to add other frontends (web, mobile) later

### 4. **Resource Efficiency**
- TUI only loads when needed
- Server runs continuously with minimal resources
- Tray has minimal footprint

## Migration Strategy

### Phase 1: Minimal Changes (1-2 weeks)
1. Add `--server` flag to main binary
2. Update tray to launch separate TUI binary
3. Create basic TUI binary that connects via gRPC

### Phase 2: Full Implementation (2-3 weeks)  
1. Implement complete TUI application
2. Add health checking and auto-reconnection
3. Enhanced error handling and user feedback

### Phase 3: Polish (1 week)
1. Installer updates for multiple binaries
2. Desktop shortcuts and file associations
3. Documentation and user guides

## Potential Issues & Solutions

### 1. **Binary Distribution**
**Issue**: Two binaries to distribute
**Solution**: Package both in installer, create wrapper scripts

### 2. **Server Discovery**
**Issue**: TUI needs to find running server
**Solution**: Standard port + health checking + auto-start option

### 3. **Configuration Sync**
**Issue**: Config changes need to reach all processes
**Solution**: File watching + gRPC config service + reload notifications

### 4. **Process Management**
**Issue**: Orphaned processes if something crashes
**Solution**: PID files, health monitoring, cleanup on startup

## Conclusion

This multi-process architecture provides:
- ✅ **Clean separation** between tray, server, and TUI
- ✅ **Independent lifecycles** for each component  
- ✅ **Scalable design** for future enhancements
- ✅ **User flexibility** in how they interact with the system
- ✅ **Resource efficiency** with on-demand TUI loading

The implementation can be done incrementally, starting with basic functionality and adding sophistication over time. The gRPC architecture you've already built makes this separation natural and robust.
