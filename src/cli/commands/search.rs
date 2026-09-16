use crate::cli::commands::CliContext;
use crate::cli::CliError;
use crate::cli::output::{OutputFormatter, TableDisplay, SimpleTable};
use crate::database::search::SearchResult as DbSearchResult;
use serde::Serialize;

pub struct SearchCommand {
    pub query: String,
    pub limit: usize,
    pub offset: usize,
}

#[async_trait::async_trait]
impl crate::cli::commands::CliCommand for SearchCommand {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        let (page, total_count) = ctx
            .services
            .search
            .search(&self.query, Some(self.limit as i32), Some(self.offset as i32))
            .await?;

        if total_count == 0 {
            formatter.print_message("No results found", crate::cli::output::MessageType::Info);
            return Ok(());
        }

        let rows: Vec<SearchRow> = page.iter().map(SearchRow::from_result).collect();

        let display = SearchResultsDisplay {
            total: total_count as usize,
            displayed: rows,
        };

        formatter.print(&display)
    }
}

#[derive(Serialize)]
struct SearchRow {
    id: String,
    name: String,
    path: String,
    tempo: f64,
    key: String,
    time_signature: String,
    rank: f64,
    reasons: String,
}

impl SearchRow {
    fn from_result(r: &DbSearchResult) -> Self {
        let p = &r.project;
        let reasons = if r.match_reason.is_empty() {
            String::new()
        } else {
            r.match_reason
                .iter()
                .map(|mr| format!("{:?}", mr))
                .collect::<Vec<_>>()
                .join(", ")
        };

        Self {
            id: p.id.to_string(),
            name: p.name.clone(),
            path: p.file_path.display().to_string(),
            tempo: p.tempo,
            key: p
                .key_signature
                .as_ref()
                .map(|k| k.to_string())
                .unwrap_or_else(|| "".to_string()),
            time_signature: format!("{}/{}", p.time_signature.numerator, p.time_signature.denominator),
            rank: r.rank,
            reasons,
        }
    }
}

#[derive(Serialize)]
struct SearchResultsDisplay {
    total: usize,
    displayed: Vec<SearchRow>,
}

impl TableDisplay for SearchResultsDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Path".to_string(),
            "Tempo".to_string(),
            "Key".to_string(),
            "Time Sig".to_string(),
            "Rank".to_string(),
            "Reason".to_string(),
        ]);
        for row in &self.displayed {
            table.add_row(vec![
                row.id.clone(),
                row.name.clone(),
                row.path.clone(),
                format!("{:.1}", row.tempo),
                row.key.clone(),
                row.time_signature.clone(),
                format!("{:.4}", row.rank),
                row.reasons.clone(),
            ]);
        }
        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer
            .write_record(["id", "name", "path", "tempo", "key", "time_signature", "rank", "reasons"])
            .map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.name.as_str(),
                    row.path.as_str(),
                    &format!("{:.1}", row.tempo),
                    row.key.as_str(),
                    row.time_signature.as_str(),
                    &format!("{:.4}", row.rank),
                    row.reasons.as_str(),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}
