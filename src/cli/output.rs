use crate::cli::OutputFormat;
use crate::cli::CliError;
use colored::Colorize;
use serde::Serialize;
use std::cmp;
use terminal_size::{Width, terminal_size};

/// Output formatter for CLI results
pub struct OutputFormatter {
    format: OutputFormat,
    no_color: bool,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        Self { format, no_color }
    }

    /// Format and print data
    pub fn print<T: Serialize + TableDisplay>(&self, data: &T) -> Result<(), CliError> {
        match self.format {
            OutputFormat::Table => self.print_simple_table(data),
            OutputFormat::Json => self.print_json(data),
            OutputFormat::Csv => self.print_csv(data),
        }
    }

    /// Print a message with appropriate formatting
    pub fn print_message(&self, message: &str, message_type: MessageType) {
        let formatted_message = match message_type {
            MessageType::Info => message.blue(),
            MessageType::Success => message.green(),
            MessageType::Warning => message.yellow(),
            MessageType::Error => message.red(),
        };

        if self.no_color {
            println!("{}", message);
        } else {
            println!("{}", formatted_message);
        }
    }

    /// Get terminal width with fallback
    fn get_terminal_width(&self) -> usize {
        if let Some((Width(width), _)) = terminal_size() {
            width as usize
        } else {
            80 // fallback width
        }
    }

    fn print_simple_table<T: Serialize + TableDisplay>(&self, data: &T) -> Result<(), CliError> {
        let table_data = data.to_simple_table();
        let terminal_width = self.get_terminal_width();
        
        if table_data.rows.is_empty() {
            return Ok(());
        }

        // Calculate column widths
        let num_cols = table_data.headers.len();
        let mut col_widths = vec![0; num_cols];

        // Find max width for each column
        for (i, header) in table_data.headers.iter().enumerate() {
            col_widths[i] = cmp::max(col_widths[i], header.len());
        }

        for row in &table_data.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    // Strip color codes for width calculation
                    let plain_text = strip_ansi_codes(cell);
                    col_widths[i] = cmp::max(col_widths[i], plain_text.len());
                }
            }
        }

        // Adjust column widths to fit terminal
        let padding = 2; // spaces between columns
        let total_padding = padding * (num_cols - 1);
        let available_width = terminal_width.saturating_sub(total_padding);
        
        self.adjust_column_widths(&mut col_widths, available_width);

        // Print header
        self.print_table_row(&table_data.headers, &col_widths, true);
        
        // Print separator line
        self.print_separator(&col_widths);

        // Print data rows
        for row in &table_data.rows {
            self.print_table_row(row, &col_widths, false);
        }

        Ok(())
    }

    fn adjust_column_widths(&self, col_widths: &mut [usize], available_width: usize) {
        let total_preferred = col_widths.iter().sum::<usize>();
        
        if total_preferred <= available_width {
            return; // All columns fit
        }

        // Proportionally reduce column widths
        let reduction_factor = available_width as f64 / total_preferred as f64;
        
        for width in col_widths.iter_mut() {
            *width = cmp::max(8, (*width as f64 * reduction_factor) as usize); // min width 8
        }
    }

    fn print_table_row(&self, row: &[String], col_widths: &[usize], is_header: bool) {
        let mut output = String::new();
        
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                output.push_str("  "); // column padding
            }
            
            if i < col_widths.len() {
                let width = col_widths[i];
                let plain_text = strip_ansi_codes(cell);
                
                if plain_text.len() > width {
                    // Truncate with ellipsis
                    let truncated = if width > 3 {
                        format!("{}...", &plain_text[..width-3])
                    } else {
                        plain_text[..width].to_string()
                    };
                    
                    if self.no_color || !cell.contains('\x1b') {
                        output.push_str(&format!("{:<width$}", truncated, width = width));
                    } else {
                        // Preserve color formatting for truncated text
                        output.push_str(&format!("{:<width$}", truncated, width = width));
                    }
                } else {
                    // Pad to column width
                    if self.no_color || !cell.contains('\x1b') {
                        output.push_str(&format!("{:<width$}", plain_text, width = width));
                    } else {
                        // For colored text, we need to pad manually
                        let padding = width - plain_text.len();
                        output.push_str(cell);
                        output.push_str(&" ".repeat(padding));
                    }
                }
            } else {
                output.push_str(cell);
            }
        }

        if is_header && !self.no_color {
            println!("{}", output.bold());
        } else {
            println!("{}", output);
        }
    }

    fn print_separator(&self, col_widths: &[usize]) {
        let mut separator = String::new();
        
        for (i, &width) in col_widths.iter().enumerate() {
            if i > 0 {
                separator.push_str("  ");
            }
            separator.push_str(&"─".repeat(width));
        }

        if self.no_color {
            println!("{}", separator);
        } else {
            println!("{}", separator.dimmed());
        }
    }

    fn print_json<T: Serialize>(&self, data: &T) -> Result<(), CliError> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| -> CliError { e.into() })?;
        println!("{}", json);
        Ok(())
    }

    fn print_csv<T: Serialize + TableDisplay>(&self, data: &T) -> Result<(), CliError> {
        let mut writer = csv::Writer::from_writer(std::io::stdout());
        data.to_csv(&mut writer)?;
        writer.flush()?;
        Ok(())
    }
}

/// Message type for colored output
pub enum MessageType {
    Info,
    Success,
    Warning,
    Error,
}

/// Simple table data structure
#[derive(Clone, Serialize)]
pub struct SimpleTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl SimpleTable {
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }
}

/// Trait for types that can be displayed as tables
pub trait TableDisplay {
    fn to_simple_table(&self) -> SimpleTable;
    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError>;
}

/// Helper macro to create table rows for SimpleTable
#[macro_export]
macro_rules! simple_table_row {
    ($table:expr, $($cell:expr),* $(,)?) => {
        $table.add_row(vec![$($cell.to_string()),*]);
    };
}

/// Helper macro to create colored table cells
#[macro_export]
macro_rules! colored_cell {
    ($value:expr, $color:ident) => {
        if $crate::cli::output::should_use_color() {
            {
                use colored::Colorize;
                $value.to_string().$color().to_string()
            }
        } else {
            $value.to_string()
        }
    };
}

/// Check if colors should be used
pub fn should_use_color() -> bool {
    !std::env::var("NO_COLOR").is_ok() && colored::control::SHOULD_COLORIZE.should_colorize()
}

/// Strip ANSI escape codes from a string to get plain text length
fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip escape sequence
            if chars.next() == Some('[') {
                // Skip until we find a letter (end of escape sequence)
                while let Some(c) = chars.next() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Serialize)]
    struct TestTable {
        table: SimpleTable,
    }
    
    impl TableDisplay for TestTable {
        fn to_simple_table(&self) -> SimpleTable {
            self.table.clone()
        }
        
        fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
            writer.write_record(["col1", "col2"]).map_err(|e| -> CliError { e.into() })?;
            for row in &self.table.rows {
                if row.len() >= 2 {
                    writer.write_record([&row[0], &row[1]]).map_err(|e| -> CliError { e.into() })?;
                }
            }
            Ok(())
        }
    }
    
    #[test]
    fn test_simple_table_creation() {
        let mut table = SimpleTable::new(vec!["Name".to_string(), "Value".to_string()]);
        table.add_row(vec!["Test".to_string(), "123".to_string()]);
        table.add_row(vec!["Another".to_string(), "456".to_string()]);
        
        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0][0], "Test");
        assert_eq!(table.rows[1][1], "456");
    }
    
    #[test]
    fn test_table_formatting() {
        let mut table = SimpleTable::new(vec!["Short".to_string(), "Very Long Header Name".to_string()]);
        table.add_row(vec!["A".to_string(), "Short".to_string()]);
        table.add_row(vec!["Very Long Value".to_string(), "B".to_string()]);
        
        let test_data = TestTable { table };
        let formatter = OutputFormatter::new(OutputFormat::Table, false);
        
        // This should not panic and should handle width calculations
        let result = formatter.print(&test_data);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_ansi_stripping() {
        let colored_text = "\x1b[32mGreen Text\x1b[0m";
        let stripped = strip_ansi_codes(colored_text);
        assert_eq!(stripped, "Green Text");
        
        let plain_text = "Normal Text";
        let stripped_plain = strip_ansi_codes(plain_text);
        assert_eq!(stripped_plain, "Normal Text");
    }
}
