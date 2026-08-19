//! Visual tests for CLI output formatting
//! 
//! These tests are designed to be run manually to visually inspect the output formatting
//! capabilities of the CLI system. They generate various table structures and data types
//! to demonstrate how the output looks in different scenarios.

use seula::cli::output::{OutputFormatter, SimpleTable, TableDisplay, MessageType};
use seula::cli::{OutputFormat, CliError};
use serde::Serialize;
use colored::Colorize;

#[allow(unused)]
mod common;

/// Test data structure for basic table display
#[derive(Debug, Serialize)]
struct MockProjectData {
    projects: Vec<MockProject>,
}

#[derive(Debug, Serialize)]
struct MockProject {
    id: String,
    name: String,
    path: String,
    tempo: f64,
    time_signature: String,
    key: Option<String>,
    version: String,
    plugin_count: usize,
    sample_count: usize,
    created: String,
    size_mb: f64,
    status: String,
}

impl TableDisplay for MockProjectData {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Path".to_string(),
            "BPM".to_string(),
            "Time Sig".to_string(),
            "Key".to_string(),
            "Version".to_string(),
            "Plugins".to_string(),
            "Samples".to_string(),
            "Created".to_string(),
            "Size (MB)".to_string(),
            "Status".to_string(),
        ]);

        for project in &self.projects {
            table.add_row(vec![
                project.id.clone(),
                project.name.clone(),
                project.path.clone(),
                format!("{:.1}", project.tempo),
                project.time_signature.clone(),
                project.key.as_deref().unwrap_or("N/A").to_string(),
                project.version.clone(),
                project.plugin_count.to_string(),
                project.sample_count.to_string(),
                project.created.clone(),
                format!("{:.1}", project.size_mb),
                if project.status == "Active" {
                    project.status.green().to_string()
                } else {
                    project.status.yellow().to_string()
                },
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(&[
            "ID", "Name", "Path", "BPM", "Time Signature", "Key", "Version",
            "Plugins", "Samples", "Created", "Size (MB)", "Status"
        ])?;

        for project in &self.projects {
            writer.write_record(&[
                &project.id,
                &project.name,
                &project.path,
                &format!("{:.1}", project.tempo),
                &project.time_signature,
                project.key.as_deref().unwrap_or("N/A"),
                &project.version,
                &project.plugin_count.to_string(),
                &project.sample_count.to_string(),
                &project.created,
                &format!("{:.1}", project.size_mb),
                &project.status,
            ])?;
        }

        Ok(())
    }
}

/// Test data structure for plugin information
#[derive(Debug, Serialize)]
struct MockPluginData {
    plugins: Vec<MockPluginInfo>,
}

#[derive(Debug, Serialize)]
struct MockPluginInfo {
    name: String,
    vendor: String,
    format: String,
    version: String,
    installed: bool,
    usage_count: usize,
    last_used: Option<String>,
    category: String,
}

impl TableDisplay for MockPluginData {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Name".to_string(),
            "Vendor".to_string(),
            "Format".to_string(),
            "Version".to_string(),
            "OK".to_string(),  // Shorter header for installed status
            "Usage".to_string(),
            "Last Used".to_string(),
            "Category".to_string(),
        ]);

        for plugin in &self.plugins {
            // Truncate long names to fit better
            let name = if plugin.name.len() > 28 {
                format!("{}...", &plugin.name[..25])
            } else {
                plugin.name.clone()
            };

            let vendor = if plugin.vendor.len() > 28 {
                format!("{}...", &plugin.vendor[..25])
            } else {
                plugin.vendor.clone()
            };

            let version = if plugin.version.len() > 7 {
                format!("{}...", &plugin.version[..4])
            } else {
                plugin.version.clone()
            };

            table.add_row(vec![
                name,
                vendor,
                plugin.format.clone(),
                version,
                if plugin.installed {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                },
                plugin.usage_count.to_string(),
                plugin.last_used.as_deref().unwrap_or("Never").to_string(),
                plugin.category.clone(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(&[
            "Name", "Vendor", "Format", "Version", "Installed", "Usage", "Last Used", "Category"
        ])?;

        for plugin in &self.plugins {
            writer.write_record(&[
                &plugin.name,
                &plugin.vendor,
                &plugin.format,
                &plugin.version,
                &(if plugin.installed { "Yes" } else { "No" }).to_string(),
                &plugin.usage_count.to_string(),
                plugin.last_used.as_deref().unwrap_or("Never"),
                &plugin.category,
            ])?;
        }

        Ok(())
    }
}

/// Test data structure for system statistics
#[derive(Debug, Serialize)]
struct MockSystemStats {
    stats: Vec<MockStat>,
}

#[derive(Debug, Serialize)]
struct MockStat {
    metric: String,
    value: String,
    description: String,
}

impl TableDisplay for MockSystemStats {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Metric".to_string(),
            "Value".to_string(),
            "Description".to_string(),
        ]);

        for stat in &self.stats {
            table.add_row(vec![
                stat.metric.cyan().to_string(),
                stat.value.bold().to_string(),
                stat.description.clone(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(&["Metric", "Value", "Description"])?;

        for stat in &self.stats {
            writer.write_record(&[&stat.metric, &stat.value, &stat.description])?;
        }

        Ok(())
    }
}

/// Test data structure for narrow tables (few columns)
#[derive(Debug, Serialize)]
struct MockTagData {
    tags: Vec<MockTag>,
}

#[derive(Debug, Serialize)]
struct MockTag {
    name: String,
    color: String,
    projects: usize,
}

impl TableDisplay for MockTagData {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Tag".to_string(),
            "Color".to_string(),
            "Projects".to_string(),
        ]);

        for tag in &self.tags {
            table.add_row(vec![
                tag.name.clone(),
                format!("● {}", tag.color).color(tag.color.as_str()).to_string(),
                tag.projects.to_string(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(&["Tag", "Color", "Projects"])?;

        for tag in &self.tags {
            writer.write_record(&[&tag.name, &tag.color, &tag.projects.to_string()])?;
        }

        Ok(())
    }
}

/// Test data structure with very wide content
#[derive(Debug, Serialize)]
struct MockWideData {
    entries: Vec<MockWideEntry>,
}

#[derive(Debug, Serialize)]
struct MockWideEntry {
    id: String,
    very_long_path: String,
    extremely_long_description: String,
    short: String,
}

impl TableDisplay for MockWideData {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Very Long File Path That Exceeds Normal Width".to_string(),
            "Extremely Long Description That Should Be Truncated When Terminal Width Is Limited".to_string(),
            "Short".to_string(),
        ]);

        for entry in &self.entries {
            table.add_row(vec![
                entry.id.clone(),
                entry.very_long_path.clone(),
                entry.extremely_long_description.clone(),
                entry.short.clone(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(&[
            "ID", 
            "Very Long File Path", 
            "Extremely Long Description", 
            "Short"
        ])?;

        for entry in &self.entries {
            writer.write_record(&[
                &entry.id,
                &entry.very_long_path,
                &entry.extremely_long_description,
                &entry.short,
            ])?;
        }

        Ok(())
    }
}

/// Generate mock project data for testing
fn create_mock_project_data() -> MockProjectData {
    MockProjectData {
        projects: vec![
            MockProject {
                id: "proj_001".to_string(),
                name: "Deep House Vibes".to_string(),
                path: "/Users/producer/Music/Projects/Deep House Vibes.als".to_string(),
                tempo: 124.0,
                time_signature: "4/4".to_string(),
                key: Some("Am".to_string()),
                version: "11.2.0".to_string(),
                plugin_count: 8,
                sample_count: 23,
                created: "2023-10-15".to_string(),
                size_mb: 45.2,
                status: "Active".to_string(),
            },
            MockProject {
                id: "proj_002".to_string(),
                name: "Experimental Ambient Soundscape with Very Long Name".to_string(),
                path: "/Users/producer/Music/Projects/Experimental/Experimental Ambient Soundscape with Very Long Name.als".to_string(),
                tempo: 80.0,
                time_signature: "7/8".to_string(),
                key: Some("F#m".to_string()),
                version: "11.1.5".to_string(),
                plugin_count: 15,
                sample_count: 67,
                created: "2023-09-22".to_string(),
                size_mb: 128.7,
                status: "Draft".to_string(),
            },
            MockProject {
                id: "proj_003".to_string(),
                name: "Quick Beat".to_string(),
                path: "/Music/Quick Beat.als".to_string(),
                tempo: 140.0,
                time_signature: "4/4".to_string(),
                key: None,
                version: "10.1.43".to_string(),
                plugin_count: 3,
                sample_count: 12,
                created: "2023-11-01".to_string(),
                size_mb: 12.4,
                status: "Active".to_string(),
            },
            MockProject {
                id: "proj_004".to_string(),
                name: "🎵 Unicode Test 音楽".to_string(),
                path: "/Users/producer/🎵 Unicode Test 音楽.als".to_string(),
                tempo: 128.0,
                time_signature: "4/4".to_string(),
                key: Some("C".to_string()),
                version: "11.2.0".to_string(),
                plugin_count: 6,
                sample_count: 18,
                created: "2023-10-30".to_string(),
                size_mb: 34.1,
                status: "Draft".to_string(),
            },
        ],
    }
}

/// Generate mock plugin data for testing
fn create_mock_plugin_data() -> MockPluginData {
    MockPluginData {
        plugins: vec![
            MockPluginInfo {
                name: "Serum".to_string(),
                vendor: "Xfer Records".to_string(),
                format: "VST3".to_string(),
                version: "1.36b1".to_string(),
                installed: true,
                usage_count: 45,
                last_used: Some("2023-11-01".to_string()),
                category: "Synth".to_string(),
            },
            MockPluginInfo {
                name: "FabFilter Pro-Q 3".to_string(),
                vendor: "FabFilter".to_string(),
                format: "VST3".to_string(),
                version: "3.22".to_string(),
                installed: true,
                usage_count: 78,
                last_used: Some("2023-10-31".to_string()),
                category: "EQ".to_string(),
            },
            MockPluginInfo {
                name: "Valhalla VintageVerb".to_string(),
                vendor: "Valhalla DSP".to_string(),
                format: "VST3".to_string(),
                version: "3.0.1".to_string(),
                installed: false,
                usage_count: 12,
                last_used: Some("2023-08-15".to_string()),
                category: "Reverb".to_string(),
            },
            MockPluginInfo {
                name: "Massive X".to_string(),
                vendor: "Native Instruments".to_string(),
                format: "VST3".to_string(),
                version: "1.4.4".to_string(),
                installed: true,
                usage_count: 23,
                last_used: None,
                category: "Synth".to_string(),
            },
            MockPluginInfo {
                name: "Really Long Plugin Name Here".to_string(),
                vendor: "Super Long Vendor Name Inc".to_string(),
                format: "VST2".to_string(),
                version: "2.1.0.b12345".to_string(),
                installed: true,
                usage_count: 1,
                last_used: Some("2023-01-15".to_string()),
                category: "Effect".to_string(),
            },
            MockPluginInfo {
                name: "Operator".to_string(),
                vendor: "Ableton".to_string(),
                format: "VST3".to_string(),
                version: "11.2".to_string(),
                installed: true,
                usage_count: 156,
                last_used: Some("2023-11-02".to_string()),
                category: "Synth".to_string(),
            },
            MockPluginInfo {
                name: "Compressor".to_string(),
                vendor: "Ableton".to_string(),
                format: "VST3".to_string(),
                version: "11.2".to_string(),
                installed: true,
                usage_count: 89,
                last_used: Some("2023-11-01".to_string()),
                category: "Dynamics".to_string(),
            },
        ],
    }
}

/// Generate mock system stats for testing
fn create_mock_system_stats() -> MockSystemStats {
    MockSystemStats {
        stats: vec![
            MockStat {
                metric: "Total Projects".to_string(),
                value: "1,247".to_string(),
                description: "Number of Ableton Live projects in database".to_string(),
            },
            MockStat {
                metric: "Active Projects".to_string(),
                value: "892".to_string(),
                description: "Projects that exist on filesystem".to_string(),
            },
            MockStat {
                metric: "Total Plugins".to_string(),
                value: "156".to_string(),
                description: "Unique plugins found across all projects".to_string(),
            },
            MockStat {
                metric: "Installed Plugins".to_string(),
                value: "134".to_string(),
                description: "Plugins currently installed on system".to_string(),
            },
            MockStat {
                metric: "Total Samples".to_string(),
                value: "8,943".to_string(),
                description: "Audio samples referenced in projects".to_string(),
            },
            MockStat {
                metric: "Missing Samples".to_string(),
                value: "67".to_string(),
                description: "Samples that cannot be found on filesystem".to_string(),
            },
            MockStat {
                metric: "Database Size".to_string(),
                value: "45.2 MB".to_string(),
                description: "Size of the Seula database file".to_string(),
            },
            MockStat {
                metric: "Last Scan".to_string(),
                value: "2023-11-01 14:30:15".to_string(),
                description: "Timestamp of the most recent project scan".to_string(),
            },
        ],
    }
}

/// Generate mock tag data for testing
fn create_mock_tag_data() -> MockTagData {
    MockTagData {
        tags: vec![
            MockTag {
                name: "House".to_string(),
                color: "blue".to_string(),
                projects: 45,
            },
            MockTag {
                name: "Techno".to_string(),
                color: "red".to_string(),
                projects: 32,
            },
            MockTag {
                name: "Ambient".to_string(),
                color: "green".to_string(),
                projects: 18,
            },
            MockTag {
                name: "Experimental".to_string(),
                color: "purple".to_string(),
                projects: 12,
            },
            MockTag {
                name: "Work in Progress".to_string(),
                color: "yellow".to_string(),
                projects: 67,
            },
        ],
    }
}

/// Generate mock wide data for testing truncation
fn create_mock_wide_data() -> MockWideData {
    MockWideData {
        entries: vec![
            MockWideEntry {
                id: "001".to_string(),
                very_long_path: "/Users/producer/Music/Projects/2023/October/Deep House/Experimental/Subgenre/Deep House Vibes Final Master v3.als".to_string(),
                extremely_long_description: "This is an extremely long description that contains way too much information and should definitely be truncated when displayed in a terminal with limited width. It goes on and on with unnecessary details about the project, including technical specifications, creative notes, collaboration details, and various other metadata that makes the text exceed reasonable display limits.".to_string(),
                short: "OK".to_string(),
            },
            MockWideEntry {
                id: "002".to_string(),
                very_long_path: "/Volumes/External_Drive/Music_Production/Archive/2022/Ambient_Projects/Soundscapes/Nature_Sounds/Forest_Ambient_with_Birds_and_Water_Sounds_Extended_Version.als".to_string(),
                extremely_long_description: "Another incredibly verbose description that provides far more detail than necessary for a simple table display. This description includes information about the recording location, equipment used, post-processing techniques, intended audience, release plans, and artistic inspiration behind the work.".to_string(),
                short: "Good".to_string(),
            },
        ],
    }
}

/// Test basic table formatting with project data
#[test]
fn test_visual_project_table() {
    common::setup("error");
    
    println!("\n=== PROJECT TABLE TEST ===");
    let data = create_mock_project_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing project table display:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test plugin table with colored status indicators
#[test]
fn test_visual_plugin_table() {
    common::setup("error");
    
    println!("\n=== PLUGIN TABLE TEST ===");
    let data = create_mock_plugin_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing plugin table with status indicators:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test system stats table with colored metrics
#[test]
fn test_visual_stats_table() {
    common::setup("error");
    
    println!("\n=== SYSTEM STATS TABLE TEST ===");
    let data = create_mock_system_stats();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing system statistics table:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test narrow table with tags
#[test]
fn test_visual_narrow_table() {
    common::setup("error");
    
    println!("\n=== NARROW TABLE TEST ===");
    let data = create_mock_tag_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing narrow table with few columns:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test wide table with truncation
#[test]
fn test_visual_wide_table() {
    common::setup("error");
    
    println!("\n=== WIDE TABLE TEST (Truncation) ===");
    let data = create_mock_wide_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing wide table with truncation:", MessageType::Warning);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test table without colors
#[test]
fn test_visual_no_color_table() {
    common::setup("error");
    
    println!("\n=== NO COLOR TABLE TEST ===");
    let data = create_mock_project_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, true);
    
    formatter.print_message("Testing table without colors:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
}

/// Test JSON output format
#[test]
fn test_visual_json_output() {
    common::setup("error");
    
    println!("\n=== JSON OUTPUT TEST ===");
    let data = create_mock_project_data();
    let formatter = OutputFormatter::new(OutputFormat::Json, false);
    
    formatter.print_message("Testing JSON output format:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing JSON: {}", e);
    }
}

/// Test CSV output format
#[test]
fn test_visual_csv_output() {
    common::setup("error");
    
    println!("\n=== CSV OUTPUT TEST ===");
    let data = create_mock_project_data();
    let formatter = OutputFormatter::new(OutputFormat::Csv, false);
    
    formatter.print_message("Testing CSV output format:", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing CSV: {}", e);
    }
}

/// Test empty table
#[test]
fn test_visual_empty_table() {
    common::setup("error");
    
    println!("\n=== EMPTY TABLE TEST ===");
    let data = MockProjectData { projects: vec![] };
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing empty table:", MessageType::Warning);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing empty table: {}", e);
    }
    formatter.print_message("(Empty table should display nothing)", MessageType::Info);
}

/// Test all message types
#[test]
fn test_visual_message_types() {
    common::setup("error");
    
    println!("\n=== MESSAGE TYPES TEST ===");
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("This is an info message", MessageType::Info);
    formatter.print_message("This is a success message", MessageType::Success);
    formatter.print_message("This is a warning message", MessageType::Warning);
    formatter.print_message("This is an error message", MessageType::Error);
    
    println!("\n--- Same messages without color ---");
    let no_color_formatter = OutputFormatter::new(OutputFormat::Table, true);
    no_color_formatter.print_message("This is an info message (no color)", MessageType::Info);
    no_color_formatter.print_message("This is a success message (no color)", MessageType::Success);
    no_color_formatter.print_message("This is a warning message (no color)", MessageType::Warning);
    no_color_formatter.print_message("This is an error message (no color)", MessageType::Error);
}

/// Comprehensive visual test that runs all scenarios
#[test]
fn test_visual_comprehensive() {
    common::setup("error");
    
    println!("\n=== COMPREHENSIVE VISUAL TEST ===");
    println!("This test runs all visual scenarios in sequence.\n");
    
    // Run all individual tests
    test_visual_project_table();
    test_visual_plugin_table();
    test_visual_stats_table();
    test_visual_narrow_table();
    test_visual_wide_table();
    test_visual_no_color_table();
    test_visual_json_output();
    test_visual_csv_output();
    test_visual_empty_table();
    test_visual_message_types();
    
    println!("\n=== COMPREHENSIVE TEST COMPLETE ===");
}

/// Test with different terminal widths (simulated)
#[test]
fn test_visual_different_widths() {
    common::setup("error");
    
    println!("\n=== DIFFERENT TERMINAL WIDTHS TEST ===");
    println!("Note: Actual width depends on your terminal. These tests show the same data:");
    
    let data = create_mock_project_data();
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Project table (your current terminal width):", MessageType::Info);
    if let Err(e) = formatter.print(&data) {
        eprintln!("Error printing table: {}", e);
    }
    
    println!("\nTip: Resize your terminal and run this test again to see different truncation behavior!");
}

/// Test column alignment specifically
#[test]
fn test_visual_alignment() {
    common::setup("error");
    
    println!("\n=== COLUMN ALIGNMENT TEST ===");
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing improved plugin table alignment:", MessageType::Info);
    let plugin_data = create_mock_plugin_data();
    if let Err(e) = formatter.print(&plugin_data) {
        eprintln!("Error printing plugin table: {}", e);
    }
    
    println!("\n");
    formatter.print_message("Testing project table alignment:", MessageType::Info);
    let project_data = create_mock_project_data();
    if let Err(e) = formatter.print(&project_data) {
        eprintln!("Error printing project table: {}", e);
    }
}

/// Test   character alignment and width handling
#[test]
fn test_visual_unicode_alignment() {
    common::setup("error");
    
    println!("\n=== UNICODE ALIGNMENT TEST ===");
    let formatter = OutputFormatter::new(OutputFormat::Table, false);
    
    formatter.print_message("Testing Unicode character alignment with various character types:", MessageType::Info);
    
    // Create test data with various Unicode characters
    let unicode_data = MockProjectData {
        projects: vec![
            MockProject {
                id: "unicode_1".to_string(),
                name: "🎵 Music Project".to_string(),  // Emoji (2 columns wide)
                path: "/Users/测试/Music.als".to_string(),  // CJK characters (2 columns each)
                tempo: 120.0,
                time_signature: "4/4".to_string(),
                key: Some("C♯m".to_string()),  // Musical symbol
                version: "11.2.0".to_string(),
                plugin_count: 5,
                sample_count: 10,
                created: "2023-11-01".to_string(),
                size_mb: 25.5,
                status: "Active".to_string(),
            },
            MockProject {
                id: "unicode_2".to_string(),
                name: "Café Ambient 🌙".to_string(),  // Accented chars + emoji
                path: "/Volumes/Ñoño/Música/Ambient.als".to_string(),  // Various accents
                tempo: 85.0,
                time_signature: "3/4".to_string(),
                key: Some("F♯".to_string()),
                version: "11.1.5".to_string(),
                plugin_count: 8,
                sample_count: 23,
                created: "2023-10-15".to_string(),
                size_mb: 42.1,
                status: "Draft".to_string(),
            },
            MockProject {
                id: "unicode_3".to_string(),
                name: "Ｈｅｌｌｏ Ｗｏｒｌｄ".to_string(),  // Full-width characters (2 columns each)
                path: "/home/user/こんにちは.als".to_string(),  // Japanese hiragana
                tempo: 140.0,
                time_signature: "7/8".to_string(),
                key: Some("A♭".to_string()),
                version: "10.1.43".to_string(),
                plugin_count: 12,
                sample_count: 45,
                created: "2023-09-20".to_string(),
                size_mb: 67.8,
                status: "Active".to_string(),
            },
            MockProject {
                id: "normal".to_string(),
                name: "Normal ASCII Project".to_string(),  // Regular ASCII for comparison
                path: "/Users/producer/Normal.als".to_string(),
                tempo: 128.0,
                time_signature: "4/4".to_string(),
                key: Some("Dm".to_string()),
                version: "11.2.0".to_string(),
                plugin_count: 6,
                sample_count: 18,
                created: "2023-11-01".to_string(),
                size_mb: 34.2,
                status: "Active".to_string(),
            },
        ],
    };
    
    if let Err(e) = formatter.print(&unicode_data) {
        eprintln!("Error printing Unicode table: {}", e);
    }
    
    formatter.print_message("Note: Columns should be properly aligned despite different character widths", MessageType::Info);
}

/// Test Unicode width calculation directly (demonstrating the fix)
#[test]
fn test_visual_unicode_width_demo() {
    common::setup("error");
    
    println!("\n=== UNICODE WIDTH CALCULATION DEMO ===");
    
    // Test various Unicode strings and their display widths
    let test_strings = vec![
        ("Hello, world!", "Regular ASCII text"),
        ("Ｈｅｌｌｏ, ｗｏｒｌｄ!", "Full-width characters (each char = 2 columns)"),
        ("🎵🎶🎸🎹", "Emojis (each emoji = 2 columns)"),
        ("测试文本", "CJK characters (each char = 2 columns)"),
        ("Café naïve résumé", "Accented characters (1 column each)"),
        ("♫ ♪ ♬ ♩", "Musical symbols (1 column each)"),
        ("C♯m F♯ A♭", "Musical notation with sharps/flats"),
        ("こんにちは世界", "Japanese hiragana + kanji (2 columns each)"),
    ];
    
    use unicode_width::UnicodeWidthStr;
    
    println!("String width analysis:");
    println!("{:<35} | {:<8} | {:<8} | Description", "Text", "Bytes", "Columns");
    println!("{:-<35}-+-{:-<8}-+-{:-<8}-+{:-<50}", "", "", "", "");
    
    for (text, description) in test_strings {
        let byte_length = text.len();
        let display_width = text.width();
        println!("{:<35} | {:<8} | {:<8} | {}", 
                text, 
                byte_length, 
                display_width, 
                description);
    }
    
    println!("\nThis demonstrates why using .len() for table alignment fails with Unicode!");
    println!("Our table formatter now uses .width() for proper alignment.");
}
