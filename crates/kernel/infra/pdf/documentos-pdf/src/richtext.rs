/// Converte NxDoc 1.0.0 ou Lexical JSON para markup Typst.
/// Se o input não for JSON válido, devolve o texto escapado.
pub fn to_typst(json_str: &str) -> String {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return typst_escape(json_str);
    };
    if val.get("nxdoc").is_some() {
        nxdoc_to_typst(&val)
    } else {
        lexical_to_typst_value(&val)
    }
}

pub fn typst_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '#' | '@' | '*' | '_' | '`' | '$' | '\\' | '<' | '>' | '[' | ']' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

// --- NxDoc ---

fn nxdoc_to_typst(val: &serde_json::Value) -> String {
    let empty = vec![];
    let blocks = val
        .get("blocks")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty);
    let mut out = String::new();
    for block in blocks {
        out.push_str(&nxdoc_block_to_typst(block));
    }
    out.trim_end_matches('\n').to_string()
}

fn nxdoc_block_to_typst(block: &serde_json::Value) -> String {
    let t = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match t {
        "paragraph" => {
            let inner = nxdoc_inlines_to_typst(block.get("children"));
            if inner.trim().is_empty() {
                "\n".to_string()
            } else {
                format!("{inner}\n\n")
            }
        }
        "heading" => {
            let level = block
                .get("level")
                .and_then(|l| l.as_u64())
                .unwrap_or(1)
                .min(6) as usize;
            let inner = nxdoc_inlines_to_typst(block.get("children"));
            format!("{} {inner}\n\n", "=".repeat(level))
        }
        "quote" => {
            let inner = nxdoc_inlines_to_typst(block.get("children"))
                .trim()
                .to_string();
            format!("#quote(block: false)[{inner}]\n\n")
        }
        "ul" => {
            let mut s = String::new();
            if let Some(arr) = block.get("items").and_then(|i| i.as_array()) {
                for item in arr {
                    s.push_str(&format!("- {}\n", nxdoc_inlines_to_typst(Some(item))));
                }
            }
            format!("{s}\n")
        }
        "ol" => {
            let mut s = String::new();
            if let Some(arr) = block.get("items").and_then(|i| i.as_array()) {
                for item in arr {
                    s.push_str(&format!("+ {}\n", nxdoc_inlines_to_typst(Some(item))));
                }
            }
            format!("{s}\n")
        }
        _ => String::new(),
    }
}

fn nxdoc_inlines_to_typst(node: Option<&serde_json::Value>) -> String {
    let empty = vec![];
    let arr = node.and_then(|v| v.as_array()).unwrap_or(&empty);
    arr.iter().map(nxdoc_inline_to_typst).collect()
}

fn nxdoc_inline_to_typst(inline: &serde_json::Value) -> String {
    let t = inline.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match t {
        "text" => {
            let raw = inline.get("text").and_then(|s| s.as_str()).unwrap_or("");
            if raw.is_empty() {
                return String::new();
            }
            let escaped = typst_escape(raw);
            let leading: String = escaped.chars().take_while(|c| c.is_whitespace()).collect();
            let trailing: String = {
                let rev: String = escaped
                    .chars()
                    .rev()
                    .take_while(|c| c.is_whitespace())
                    .collect();
                rev.chars().rev().collect()
            };
            let core = escaped.trim();
            if core.is_empty() {
                return escaped;
            }
            let flag = |key: &str| inline.get(key).and_then(|b| b.as_bool()).unwrap_or(false);
            let mut s = core.to_string();
            if flag("code") {
                s = format!("`{s}`");
            }
            if flag("bold") {
                s = format!("*{s}*");
            }
            if flag("italic") {
                s = format!("_{s}_");
            }
            if flag("underline") {
                s = format!("#underline[{s}]");
            }
            if flag("strike") {
                s = format!("#strike[{s}]");
            }
            format!("{leading}{s}{trailing}")
        }
        "placeholder" => {
            let v = inline
                .get("variavel")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            typst_escape(&format!("{{{{{v}}}}}"))
        }
        "br" => "\\\n".to_string(),
        _ => String::new(),
    }
}

// --- Lexical ---

fn lexical_to_typst_value(val: &serde_json::Value) -> String {
    let Some(root) = val.get("root") else {
        return String::new();
    };
    node_to_typst(root, false)
        .trim_end_matches('\n')
        .to_string()
}

#[derive(Clone, Copy)]
enum ListKind {
    Bullet,
    Number,
}

fn node_to_typst(node: &serde_json::Value, in_list: bool) -> String {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match node_type {
        "placeholder" => {
            let v = node.get("variavel").and_then(|v| v.as_str()).unwrap_or("");
            typst_escape(&format!("{{{{{v}}}}}"))
        }
        "text" => {
            let raw = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
            if raw.is_empty() {
                return String::new();
            }
            let fmt = node.get("format").and_then(|f| f.as_i64()).unwrap_or(0);
            apply_lexical_format(&typst_escape(raw), fmt)
        }
        "linebreak" => "\\\n".to_string(),
        "paragraph" => {
            let inner = children_to_typst(node, false);
            if inner.trim().is_empty() {
                "\n".to_string()
            } else {
                format!("{inner}\n\n")
            }
        }
        "heading" => {
            let tag = node.get("tag").and_then(|t| t.as_str()).unwrap_or("h1");
            let level = tag
                .trim_start_matches('h')
                .parse::<usize>()
                .unwrap_or(1)
                .min(6);
            let inner = children_to_typst(node, false);
            format!("{} {inner}\n\n", "=".repeat(level))
        }
        "quote" => {
            let inner = children_to_typst(node, false).trim().to_string();
            format!("#quote(block: false)[{inner}]\n\n")
        }
        "list" => {
            let kind = if node.get("listType").and_then(|t| t.as_str()) == Some("number") {
                ListKind::Number
            } else {
                ListKind::Bullet
            };
            let children = node
                .get("children")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|item| listitem_to_typst(item, kind))
                        .collect::<String>()
                })
                .unwrap_or_default();
            format!("{children}\n")
        }
        "root" => children_to_typst(node, in_list),
        _ => children_to_typst(node, in_list),
    }
}

fn listitem_to_typst(node: &serde_json::Value, kind: ListKind) -> String {
    let prefix = match kind {
        ListKind::Number => "+ ",
        ListKind::Bullet => "- ",
    };
    let inner = children_to_typst(node, true).trim().to_string();
    format!("{prefix}{inner}\n")
}

fn children_to_typst(node: &serde_json::Value, in_list: bool) -> String {
    node.get("children")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .map(|child| node_to_typst(child, in_list))
                .collect::<String>()
        })
        .unwrap_or_default()
}

/// Bitmask do Lexical: 1=bold, 2=italic, 4=strikethrough, 8=underline, 16=code.
fn apply_lexical_format(escaped: &str, fmt: i64) -> String {
    if fmt == 0 {
        return escaped.to_string();
    }
    let leading: String = escaped.chars().take_while(|c| c.is_whitespace()).collect();
    let trailing: String = {
        let rev: String = escaped
            .chars()
            .rev()
            .take_while(|c| c.is_whitespace())
            .collect();
        rev.chars().rev().collect()
    };
    let core = escaped.trim();
    if core.is_empty() {
        return escaped.to_string();
    }
    let mut s = core.to_string();
    if fmt & 16 != 0 {
        s = format!("`{s}`");
    }
    if fmt & 1 != 0 {
        s = format!("*{s}*");
    }
    if fmt & 2 != 0 {
        s = format!("_{s}_");
    }
    if fmt & 8 != 0 {
        s = format!("#underline[{s}]");
    }
    if fmt & 4 != 0 {
        s = format!("#strike[{s}]");
    }
    format!("{leading}{s}{trailing}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nxdoc_paragraph() {
        let json = r#"{"nxdoc":"1.0.0","blocks":[{"type":"paragraph","children":[{"type":"text","text":"Olá mundo."}]}]}"#;
        assert_eq!(to_typst(json), "Olá mundo.");
    }

    #[test]
    fn nxdoc_bold() {
        let json = r#"{"nxdoc":"1.0.0","blocks":[{"type":"paragraph","children":[{"type":"text","text":"forte","bold":true}]}]}"#;
        assert_eq!(to_typst(json), "*forte*");
    }

    #[test]
    fn typst_escape_chars() {
        assert_eq!(typst_escape("a#b@c"), r"a\#b\@c");
    }

    #[test]
    fn lexical_paragraph() {
        let json = r#"{"root":{"type":"root","children":[{"type":"paragraph","children":[{"type":"text","text":"Texto simples.","format":0}]}]}}"#;
        assert_eq!(to_typst(json), "Texto simples.");
    }

    #[test]
    fn fallback_plain_text() {
        assert_eq!(to_typst("texto puro"), "texto puro");
    }
}
