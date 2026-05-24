use chrono::Datelike as _;
use chrono::Timelike as _;
use regex::Regex;
use serde_json::Value;

use super::data::NdtData;

// Matches {{key}} and {{obj.nested.key}} placeholders.
fn placeholder_re() -> Regex {
    Regex::new(r"\{\{([a-zA-Z0-9_\.]+)\}\}").expect("static regex")
}

/// Replace every `{{key}}` in `text` with the corresponding value from `data`.
/// Unknown keys are left as-is.
pub fn resolve_string(text: &str, data: &NdtData) -> String {
    let re = placeholder_re();
    re.replace_all(text, |caps: &regex::Captures| {
        let key = &caps[1];
        data.get_string(key).unwrap_or_else(|| caps[0].to_string())
    })
    .into_owned()
}

/// If `text` is exactly a single `{{key}}` placeholder, return the raw JSON
/// value it resolves to (useful for image srcs and NCRTF content).
pub fn resolve_value(text: &str, data: &NdtData) -> Option<Value> {
    let re = placeholder_re();
    let trimmed = text.trim();
    let caps = re.captures(trimmed)?;

    // Ensure the entire string is that one placeholder (no surrounding text).
    if caps.get(0)?.as_str() != trimmed {
        return None;
    }

    data.get(&caps[1]).cloned()
}

// ── RuntimeContext ────────────────────────────────────────────────────────────

/// Runtime context available during PDF rendering for field resolution.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::template::resolver::{RuntimeContext, resolve_runtime_fields};
///
/// let ctx = RuntimeContext::new(2, 5);
/// let result = resolve_runtime_fields("Página {{page}} de {{total_pages}}", &ctx);
/// assert_eq!(result, "Página 2 de 5");
/// ```
pub struct RuntimeContext {
    pub page_number: u32,
    pub total_pages: u32,
    /// Long-form Portuguese date, e.g. "25 de Abril de 2026".
    pub today: String,
    /// Short date+time, e.g. "25/04/2026 14:32".
    pub now: String,
}

impl RuntimeContext {
    pub fn new(page_number: u32, total_pages: u32) -> Self {
        let dt = chrono::Local::now();
        const MONTHS: [&str; 12] = [
            "Janeiro",
            "Fevereiro",
            "Março",
            "Abril",
            "Maio",
            "Junho",
            "Julho",
            "Agosto",
            "Setembro",
            "Outubro",
            "Novembro",
            "Dezembro",
        ];
        let month_name = MONTHS[(dt.month0()) as usize];
        let today = format!("{} de {} de {}", dt.day(), month_name, dt.year());
        let now = format!(
            "{:02}/{:02}/{} {:02}:{:02}",
            dt.day(),
            dt.month(),
            dt.year(),
            dt.hour(),
            dt.minute()
        );
        Self {
            page_number,
            total_pages,
            today,
            now,
        }
    }
}

/// Resolves runtime fields in a string.
///
/// Replaces: `{{page}}`, `{{total_pages}}`, `{{today}}`, `{{now}}`.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::template::resolver::{RuntimeContext, resolve_runtime_fields};
///
/// let ctx = RuntimeContext::new(1, 3);
/// assert_eq!(resolve_runtime_fields("{{page}} / {{total_pages}}", &ctx), "1 / 3");
/// ```
pub fn resolve_runtime_fields(text: &str, ctx: &RuntimeContext) -> String {
    text.replace("{{page}}", &ctx.page_number.to_string())
        .replace("{{total_pages}}", &ctx.total_pages.to_string())
        .replace("{{today}}", &ctx.today)
        .replace("{{now}}", &ctx.now)
}
