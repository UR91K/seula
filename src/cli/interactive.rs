use crate::cli::commands::CliContext;
use crate::cli::OutputFormat;
use crate::cli::commands::CliCommand;
use crate::cli::CliError;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::{Editor, Config};
use rustyline::config::EditMode;
use std::collections::HashMap;

/// Interactive CLI mode
pub struct InteractiveCli {
    editor: Editor<()>,
    context: CliContext,
    commands: HashMap<&'static str, &'static str>,
}

impl InteractiveCli {
    pub async fn new(output_format: OutputFormat, no_color: bool) -> Result<Self, CliError> {
        let context = CliContext::new(output_format, no_color).await?;

        // Create editor with configuration to handle colored prompts properly
        let config = Config::builder()
            .edit_mode(EditMode::Emacs)
            .auto_add_history(true)
            .max_history_size(1000)
            .build();
            
        let mut editor = Editor::<()>::with_config(config).map_err(|e| -> CliError {
            std::io::Error::new(std::io::ErrorKind::Other, format!("{e}"))
                .into()
        })?;

        // Load command history if it exists
        let _ = editor.load_history(&Self::history_file());

        let mut commands = HashMap::new();
        commands.insert("help", "Show this help message");
        commands.insert("exit", "Exit the interactive CLI");
        commands.insert("quit", "Exit the interactive CLI");
        commands.insert("clear", "Clear the screen");
        commands.insert("status", "Show system status");
        commands.insert("scan", "Scan directories for projects");
        commands.insert("search", "Search projects");
        commands.insert("project", "Project management commands");
        commands.insert("sample", "Sample management commands");
        commands.insert("collection", "Collection management commands");
        commands.insert("tag", "Tag management commands");
        commands.insert("task", "Task management commands");
        commands.insert("system", "System operations");
        commands.insert("config", "Configuration management");

        Ok(Self {
            editor,
            context,
            commands,
        })
    }

    pub async fn run(&mut self) -> Result<(), CliError> {
        self.show_welcome();

        loop {
            let line = match self.editor.readline(&self.prompt()) {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "Interrupted (Ctrl+C)".yellow());
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye! 👋".green());
                    break;
                }
                Err(err) => {
                    println!("{}", format!("Error: {:?}", err).red());
                    break;
                }
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Add to history
            self.editor.add_history_entry(line);

            // Parse and execute command
            if let Err(e) = self.execute_command(line).await {
                println!("{}", format!("Error: {}", e).red());
            }
            
            // Ensure proper line separation after command execution
            // This helps prevent prompt duplication issues
            println!();
        }

        // Save history
        let _ = self.editor.save_history(&Self::history_file());

        Ok(())
    }

    fn show_welcome(&self) {
        println!("{}", "=".repeat(60).bold().blue());
        println!("{}", "  Seula Interactive CLI".bold().blue());
        println!("{}", "  Ableton Live Project Manager".bold().cyan());
        println!("{}", "=".repeat(60).bold().blue());
        println!();
        println!("{}", "Type 'help' for available commands or 'exit' to quit.".yellow());
        println!();
    }

    fn prompt(&self) -> String {
        // Use a plain prompt to avoid cursor alignment issues with colored output in terminals
        // Colors in prompts can cause rustyline to miscalculate prompt width, especially on Windows
        if self.context.no_color || Self::is_problematic_terminal() {
            "seula> ".to_string()
        } else {
            // For terminals that handle colors properly, we still use plain text
            // to ensure consistent behavior across all environments
            "seula> ".to_string()
        }
    }

    /// Detect if we're running in a terminal that has issues with colored prompts
    fn is_problematic_terminal() -> bool {
        // Check for Git Bash on Windows, which commonly has cursor alignment issues
        std::env::var("TERM").unwrap_or_default().contains("cygwin") ||
        std::env::var("MSYSTEM").is_ok() || // Git Bash/MSYS2
        cfg!(target_os = "windows")
    }

    fn history_file() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".seula_history")
    }

    async fn execute_command(&mut self, line: &str) -> Result<(), CliError> {
        let args: Vec<&str> = line.split_whitespace().collect();
        let command = args[0].to_lowercase();

        match command.as_str() {
            "help" | "?" => self.show_help(),
            "exit" | "quit" => {
                println!("{}", "Goodbye! 👋".green());
                std::process::exit(0);
            }
            "clear" => self.clear_screen(),
            "status" => self.show_status().await?,
            _ => self.execute_external_command(args).await?,
        }

        Ok(())
    }

    fn show_help(&self) {
        println!("{}", "Available Commands:".bold().underline());
        println!();

        let mut sorted_commands: Vec<_> = self.commands.iter().collect();
        sorted_commands.sort_by_key(|(name, _)| *name);

        for (name, description) in sorted_commands {
            println!("  {:<15} {}", name.bold().cyan(), description);
        }

        println!();
        println!("{}", "Command Examples:".bold().underline());
        println!("  {}", "scan [--force] [PATH ...]".italic());
        println!("  {}", "search <query> [--limit N] [--offset N]".italic());
        println!("  {}", "project list".italic());
        println!("  {}", "system info".italic());
        println!("  {}", "status".italic());
        println!();
    }

    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
        use std::io::{self, Write};
        let _ = io::stdout().flush();
    }

    async fn show_status(&self) -> Result<(), CliError> {
        println!("{}", "System Status".bold().underline());

        // Show database status
        let db = self.context.db.lock().await;
        let (projects, _plugins, samples, _collections, _tags, _tasks) =
            db.get_basic_counts().unwrap_or((0, 0, 0, 0, 0, 0));
        println!("  📁 Projects: {}", projects.to_string().bold().green());
        println!("  🎵 Samples: {}", samples.to_string().bold().green());

        // Show configuration status
        if self.context.config.needs_setup() {
            println!("  ⚠️  Configuration: {}", "Needs setup".bold().yellow());
        } else {
            println!("  ✅ Configuration: {}", "OK".bold().green());
        }

        // Show scan status
        println!("  ⏸️  Scanner: {}", "Idle".bold().cyan());

        println!();
        Ok(())
    }

    async fn execute_external_command(&mut self, args: Vec<&str>) -> Result<(), CliError> {
        if args.is_empty() {
            return Ok(());
        }

        // Convert the command line arguments back to a string for clap parsing
        #[allow(unused_variables)] // TODO: Remove this once we have implemented the commands that use this
        let full_command = format!("seula {}", args.join(" "));

        // This is a simplified approach - in a real implementation, you'd want to
        // properly parse the arguments and route to the appropriate command handlers
        match args[0] {
            "scan" => {
                use crate::cli::commands::{CliCommand, ScanCommand};

                // parse flags and paths: scan [--force] [PATH ...]
                let mut force = false;
                let mut paths: Vec<std::path::PathBuf> = Vec::new();
                for &arg in &args[1..] {
                    if arg == "--force" || arg == "-f" {
                        force = true;
                    } else {
                        paths.push(std::path::PathBuf::from(arg));
                    }
                }

                let cmd = ScanCommand { paths, force };
                cmd.execute(&self.context).await?;
            }
            "search" => {
                use crate::cli::commands::{CliCommand, SearchCommand};

                if args.len() < 2 {
                    println!("{}", "Usage: search <query> [--limit N] [--offset N]".red());
                    return Ok(());
                }

                // parse: search <query parts and/or flags>
                let mut limit: usize = 50;
                let mut offset: usize = 0;
                let mut query_parts: Vec<String> = Vec::new();

                let mut i = 1;
                while i < args.len() {
                    match args[i] {
                        "--limit" => {
                            if i + 1 < args.len() {
                                if let Ok(v) = args[i + 1].parse::<usize>() {
                                    limit = v;
                                    i += 2;
                                    continue;
                                }
                            }
                            println!("{}", "Invalid or missing value for --limit".red());
                            return Ok(());
                        }
                        "--offset" => {
                            if i + 1 < args.len() {
                                if let Ok(v) = args[i + 1].parse::<usize>() {
                                    offset = v;
                                    i += 2;
                                    continue;
                                }
                            }
                            println!("{}", "Invalid or missing value for --offset".red());
                            return Ok(());
                        }
                        _ => {
                            query_parts.push(args[i].to_string());
                            i += 1;
                        }
                    }
                }

                if query_parts.is_empty() {
                    println!("{}", "Usage: search <query> [--limit N] [--offset N]".red());
                    return Ok(());
                }

                let query = query_parts.join(" ");
                let cmd = SearchCommand { query, limit, offset };
                cmd.execute(&self.context).await?;
            }
            "project" => {
                println!("{}", "Project management commands not yet implemented".yellow());
            }
            "system" => {
                if args.len() > 1 && args[1] == "info" {
                    let cmd = crate::cli::SystemCommands::Info;
                    CliCommand::execute(&cmd, &self.context).await?;
                } else if args.len() > 1 && args[1] == "stats" {
                    let cmd = crate::cli::SystemCommands::Stats;
                    CliCommand::execute(&cmd, &self.context).await?;
                } else {
                    println!("{}", "Usage: system <info|stats>".red());
                }
            }
            _ => {
                println!("{}", format!("Unknown command: {}. Type 'help' for available commands.", args[0]).red());
            }
        }

        Ok(())
    }
}
