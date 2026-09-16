use tracing::info;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use seula::config::CONFIG;
use seula::database::ProjectDatabase;
use seula::grpc::common::ScanStatus;
use seula::http;
use seula::media::{MediaConfig, MediaStorageManager};
use seula::services::{Services, SystemService};
use seula::{grpc, tray};
use seula::grpc::projects::project_service_server;
use seula::grpc::search::search_service_server;
use seula::grpc::collections::collection_service_server;
use seula::grpc::tags::tag_service_server;
use seula::grpc::tasks::task_service_server;
use seula::grpc::media::media_service_server;
use seula::grpc::system::system_service_server;
use seula::grpc::plugins::plugin_service_server;
use seula::grpc::samples::sample_service_server;
use seula::grpc::scanning::scanning_service_server;
use seula::grpc::watcher::watcher_service_server;
use seula::cli::{Cli, Commands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CONFIG.as_ref().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    init_logging(&config.log_level);

    let cli = Cli::parse();

    match &cli.command {
        Some(command) => {
            run_direct_command(command, cli.format, cli.no_color).await
        }
        None => {
            if cli.cli {
                run_interactive_cli(cli.format, cli.no_color).await
            } else {
                let args: Vec<String> = env::args().collect();
                let run_as_server = args.contains(&"--server".to_string()) || args.contains(&"-s".to_string());

                if run_as_server {
                    run_server_mode().await
                } else {
                    run_tray_mode().await
                }
            }
        }
    }
}

/// The state every adapter (gRPC, HTTP) runs on top of, built once so all of them share
/// the same database connection and the same in-memory `SystemService` state (scan
/// status, watcher handle) rather than each opening their own -- see ADR-0024.
struct SharedState {
    db: Arc<Mutex<ProjectDatabase>>,
    media_storage: Arc<MediaStorageManager>,
    services: Services,
    system_service: SystemService,
}

fn build_shared_state() -> Result<SharedState, Box<dyn std::error::Error>> {
    let config = CONFIG
        .as_ref()
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let database_path = config
        .database_path
        .as_ref()
        .expect("Database path should be set by config initialization");
    let db_path = PathBuf::from(database_path);
    let db = ProjectDatabase::new(db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    let db = Arc::new(Mutex::new(db));

    let media_config = MediaConfig::from(config);
    let media_storage = Arc::new(MediaStorageManager::new(
        PathBuf::from(&config.media_storage_dir),
        media_config,
    )?);

    let scan_status = Arc::new(Mutex::new(ScanStatus::ScanUnknown));
    let scan_progress = Arc::new(Mutex::new(None));
    let watcher = Arc::new(Mutex::new(None));
    let watcher_events = Arc::new(Mutex::new(None));
    let start_time = Instant::now();
    let services = Services::new(Arc::clone(&db), Arc::clone(&media_storage));
    let system_service = SystemService::new(
        Arc::clone(&db),
        scan_status,
        scan_progress,
        watcher,
        watcher_events,
        start_time,
    );

    Ok(SharedState {
        db,
        media_storage,
        services,
        system_service,
    })
}

#[allow(unused)] //TODO: Remove this once it is used.
async fn run_cli_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Seula gRPC Server (CLI mode)");
    let state = build_shared_state()?;
    start_grpc_server(state).await
}

async fn run_server_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Seula gRPC Server (server-only mode)");

    let state = build_shared_state()?;
    let http_services = state.services.clone();
    let http_system_service = state.system_service.clone();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = start_http_server(http_services, http_system_service).await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    let grpc_result = start_grpc_server(state).await;
    http_handle.abort();
    grpc_result
}

async fn run_tray_mode() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Seula gRPC Server (tray mode)");

    let state = build_shared_state()?;

    // Start the HTTP server in a background task first, since starting the gRPC
    // server below consumes `state`.
    let http_services = state.services.clone();
    let http_system_service = state.system_service.clone();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = start_http_server(http_services, http_system_service).await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    // Start the gRPC server in a background task
    let server_handle = tokio::spawn(async {
        if let Err(e) = start_grpc_server(state).await {
            eprintln!("gRPC server error: {}", e);
        }
    });

    // Create and run the tray app in a blocking task
    let tray_result = tokio::task::spawn_blocking(move || {
        let tray_app = tray::TrayApp::new()?;
        tray_app.run()
    })
    .await;

    match tray_result {
        Ok(Ok(_)) => {
            info!("Tray app exited normally");
        }
        Ok(Err(e)) => {
            eprintln!("Tray app error: {}", e);
        }
        Err(e) => {
            eprintln!("Failed to run tray app: {}", e);
        }
    }

    // If we get here, the user quit the tray, so abort the servers
    server_handle.abort();
    http_handle.abort();

    Ok(())
}

async fn start_grpc_server(state: SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let config = CONFIG.as_ref().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    let server = grpc::server::SeulaServer::from_shared(
        state.db,
        state.media_storage,
        state.services,
        state.system_service,
    );

    // Set up the gRPC service
    let addr = format!("127.0.0.1:{}", config.grpc_port).parse()?;
    info!("gRPC server listening on {}", addr);

    // Start the server with all services
    tonic::transport::Server::builder()
        .add_service(project_service_server::ProjectServiceServer::new(server.clone()))
        .add_service(search_service_server::SearchServiceServer::new(server.clone()))
        .add_service(collection_service_server::CollectionServiceServer::new(server.clone()))
        .add_service(tag_service_server::TagServiceServer::new(server.clone()))
        .add_service(task_service_server::TaskServiceServer::new(server.clone()))
        .add_service(media_service_server::MediaServiceServer::new(server.clone()))
        .add_service(system_service_server::SystemServiceServer::new(server.clone()))
        .add_service(plugin_service_server::PluginServiceServer::new(server.clone()))
        .add_service(sample_service_server::SampleServiceServer::new(server.clone()))
        .add_service(scanning_service_server::ScanningServiceServer::new(server.clone()))
        .add_service(watcher_service_server::WatcherServiceServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}

async fn start_http_server(services: Services, system_service: SystemService) -> Result<(), Box<dyn std::error::Error>> {
    let config = CONFIG.as_ref().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    let state = http::state::AppState::new(services, system_service);
    let router = http::server::build_router(state);

    let addr = format!("127.0.0.1:{}", config.http_port()).parse::<std::net::SocketAddr>()?;
    info!("HTTP server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn run_interactive_cli(format: seula::cli::OutputFormat, no_color: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Seula Interactive CLI");

    let mut interactive = seula::cli::InteractiveCli::new(format, no_color)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;
    interactive
        .run()
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    Ok(())
}

async fn run_direct_command(command: &Commands, format: seula::cli::OutputFormat, no_color: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing direct CLI command");

    use seula::cli::commands::{execute_command, ScanCommand, SearchCommand};

    let result: Result<(), Box<dyn std::error::Error>> = match command {
        Commands::Scan { paths, force } => {
            let scan_cmd = ScanCommand {
                paths: paths.clone(),
                force: *force,
            };
            execute_command(&scan_cmd, format, no_color).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Search { query, limit, offset } => {
            let search_cmd = SearchCommand {
                query: query.clone(),
                limit: *limit,
                offset: *offset,
            };
            execute_command(&search_cmd, format, no_color).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Project { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Sample { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Collection { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Tag { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Task { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Plugin { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::System { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
        Commands::Config { subcommand } => {
            use seula::cli::commands::CliCommand;
            let ctx = seula::cli::commands::CliContext::new(format, no_color)
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            subcommand.execute(&ctx).await.map_err(|e| e as Box<dyn std::error::Error>)
        }
    };

    result
}

fn init_logging(log_level: &str) {
    use tracing_subscriber::filter::{EnvFilter, LevelFilter};

    let level = match log_level.to_lowercase().as_str() {
        "error" => LevelFilter::ERROR,
        "warn" => LevelFilter::WARN,
        "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        _ => {
            eprintln!("Invalid log level '{}', defaulting to 'info'", log_level);
            LevelFilter::INFO
        }
    };

    // Reproduces `env_logger::Builder::from_default_env().filter_level(level)`.
    // There, RUST_LOG is parsed first and `filter_level` then *replaces* the
    // bare (no-target) directive while leaving per-target ones alone -- so
    // `RUST_LOG=seula::scan=trace` wins for that module, but a bare
    // `RUST_LOG=debug` loses to the config value. `add_directive` with a bare
    // level has the same replace-on-same-key behaviour, so the precedence is
    // unchanged.
    let filter = EnvFilter::builder()
        .parse_lossy(std::env::var("RUST_LOG").unwrap_or_default())
        .add_directive(level.into());

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
