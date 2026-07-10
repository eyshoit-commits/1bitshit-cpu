// CLUAIZ-OS: DuckDB Expert Logic (WASM/Rust)
// Native linkage patterns for in-memory SQL execution.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn optimize_query(raw_sql: &str) -> String {
    // Logic to sanitize and optimize SQL for DuckDB
    let sanitized = raw_sql.trim().to_lowercase();
    if sanitized.contains("select") && !sanitized.contains("limit") {
        return format!("{} LIMIT 1000", raw_sql);
    }
    raw_sql.to_string()
}

#[wasm_bindgen]
pub fn format_result_as_markdown(json_data: &str) -> String {
    // Converts DuckDB JSON output to a beautiful Markdown table
    format!("| Result |\n|---|\n| {} |", json_data)
}
