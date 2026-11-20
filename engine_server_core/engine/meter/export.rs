pub fn escape_csv_cell(raw: &str) -> String {
    let sanitized = if raw.starts_with(['=', '+', '-', '@']) {
        format!("'{raw}")
    } else {
        raw.to_string()
    };
    let escaped = sanitized.replace('"', "\"\"");
    format!("\"{escaped}\"")
}
