use crate::cli::commands::CliContext;
use crate::cli::CliError;
use crate::cli::output::{OutputFormatter, TableDisplay};
use crate::database::search::{SearchQuery as DbSearchQuery, SearchResult as DbSearchResult};
use comfy_table::Table;
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

        let mut db = ctx.db.lock().await;
        let parsed = DbSearchQuery::parse(&self.query);
        let mut results: Vec<DbSearchResult> = db.search_fts(&parsed)?;

        if results.is_empty() {
            formatter.print_message("No results found", crate::cli::output::MessageType::Info);
            return Ok(());
        }

        // Pagination
        let start = self.offset.min(results.len());
        let end = (start + self.limit).min(results.len());
        let page = &results[start..end];

        let rows: Vec<SearchRow> = page
            .iter()
            .map(|r| SearchRow::from_result(r))
            .collect();

        let display = SearchResultsDisplay {
            total: results.len(),
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
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec![
            "ID",
            "Name",
            "Path",
            "Tempo",
            "Key",
            "Time Sig",
            "Rank",
            "Reason",
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
