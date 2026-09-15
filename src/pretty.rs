use crate::parser::LogEntry;

/// Renders parsed entries back into aligned text: timestamp, level (padded
/// to 5 columns, the width of "ERROR"), module (padded to the widest module
/// name in this batch), message, then fields in their original order.
pub fn pretty_print(entries: &[LogEntry]) -> String {
    let module_width = entries
        .iter()
        .map(|e| e.module.chars().count())
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    for entry in entries {
        match &entry.timestamp {
            Some(ts) => out.push_str(&ts.to_string()),
            None => out.push_str(&"-".repeat(24)),
        }
        out.push(' ');
        out.push_str(&format!("{:<5}", entry.level.as_str()));
        out.push(' ');
        out.push_str(&format!("{:<width$}", entry.module, width = module_width));
        out.push_str(": ");
        out.push_str(&entry.message);

        if !entry.fields.is_empty() {
            out.push_str(" | ");
            let rendered: Vec<String> = entry
                .fields
                .iter()
                .map(|(k, v)| render_field(k, v))
                .collect();
            out.push_str(&rendered.join(" "));
        }
        out.push('\n');
    }
    out
}

fn render_field(key: &str, value: &str) -> String {
    let needs_quotes = value.is_empty() || value.chars().any(|c| c.is_whitespace() || c == '"');
    if needs_quotes {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("{}=\"{}\"", key, escaped)
    } else {
        format!("{}={}", key, value)
    }
}
