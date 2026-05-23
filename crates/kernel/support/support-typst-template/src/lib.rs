use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypstTemplateError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Substitui marcadores `{{chave}}` no texto do template com os valores fornecidos.
pub fn substitute_vars(source: &str, vars: &[(&str, &str)]) -> String {
    let mut result = source.to_string();
    for (key, value) in vars {
        let marker = format!("{{{{{key}}}}}");
        result = result.replace(&marker, value);
    }
    result
}

/// Extrai texto simples de codigo-fonte Typst.
///
/// Remove:
/// - diretivas de linha (`#set`, `#let`, `#show`, `#import`, `#v(`, `#line(`)
/// - marcadores de cabecalho (`= `, `== `, `=== `)
/// - formatacao inline (`*...*`, `_..._`)
/// - converte listas ordenadas (`+ `) para `1. ` e nao ordenadas (`- `) para bullet.
pub fn extract_plain_text(typst_source: &str) -> String {
    let mut output_lines: Vec<String> = Vec::new();

    for line in typst_source.lines() {
        let trimmed = line.trim();

        if is_typst_directive(trimmed) {
            continue;
        }

        let line = strip_heading_marker(trimmed);
        let line = convert_list_marker(&line);
        let line = strip_inline_formatting(&line);

        output_lines.push(line);
    }

    collapse_blank_lines(&output_lines)
}

/// Renderiza um template Typst para texto simples: substitui variaveis e extrai texto.
pub fn render_text(typst_source: &str, vars: &[(&str, &str)]) -> String {
    let substituted = substitute_vars(typst_source, vars);
    extract_plain_text(&substituted)
}

/// Le um ficheiro `.typ` e devolve o conteudo como `String`.
pub fn load_typst_file(path: impl AsRef<Path>) -> Result<String, TypstTemplateError> {
    Ok(std::fs::read_to_string(path.as_ref())?)
}

fn is_typst_directive(trimmed: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "#set ",
        "#let ",
        "#show ",
        "#import ",
        "#include ",
        "#v(",
        "#line(",
        "#pagebreak()",
        "#align(",
        "#figure(",
        "#table(",
        "#grid(",
        "#colbreak()",
    ];
    PREFIXES.iter().any(|p| trimmed.starts_with(p))
}

fn strip_heading_marker(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("=== ") {
        rest.to_string()
    } else if let Some(rest) = line.strip_prefix("== ") {
        rest.to_string()
    } else if let Some(rest) = line.strip_prefix("= ") {
        rest.to_string()
    } else {
        line.to_string()
    }
}

fn convert_list_marker(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("+ ") {
        format!("1. {rest}")
    } else if let Some(rest) = line.strip_prefix("- ") {
        format!("• {rest}")
    } else {
        line.to_string()
    }
}

fn strip_inline_formatting(text: &str) -> String {
    let without_bold = strip_paired_markup(text, '*');
    strip_paired_markup(&without_bold, '_')
}

fn strip_paired_markup(text: &str, marker: char) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == marker {
            let mut inner = String::new();
            let mut found = false;
            for c in chars.by_ref() {
                if c == marker {
                    found = true;
                    break;
                }
                inner.push(c);
            }
            if found {
                result.push_str(&inner);
            } else {
                result.push(marker);
                result.push_str(&inner);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn collapse_blank_lines(lines: &[String]) -> String {
    let mut result = String::new();
    let mut prev_blank = false;
    for line in lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push_str(line);
        result.push('\n');
        prev_blank = is_blank;
    }
    result.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_vars() {
        let result = substitute_vars(
            "Eu, {{nome}}, NIF {{NIF}}.",
            &[("nome", "Ana Silva"), ("NIF", "501964843")],
        );
        assert_eq!(result, "Eu, Ana Silva, NIF 501964843.");
    }

    #[test]
    fn strips_headings() {
        let source = "= Título\n== Sub\n=== Sub-sub\nTexto.";
        let text = extract_plain_text(source);
        assert!(text.contains("Título"));
        assert!(text.contains("Sub"));
        assert!(!text.contains("= "));
        assert!(!text.contains("== "));
    }

    #[test]
    fn strips_bold_preserving_placeholders() {
        let text = strip_paired_markup("*{{nome}}*, NIF *{{NIF}}*.", '*');
        assert_eq!(text, "{{nome}}, NIF {{NIF}}.");
    }

    #[test]
    fn strips_bold_mixed() {
        let text = strip_paired_markup("*Classificação documental:* {{classificação}}", '*');
        assert_eq!(text, "Classificação documental: {{classificação}}");
    }

    #[test]
    fn skips_set_directives() {
        let source =
            "#set text(font: \"Linux Libertine\")\n#set page(paper: \"a4\")\nTexto normal.";
        let text = extract_plain_text(source);
        assert!(!text.contains("#set"));
        assert!(text.contains("Texto normal."));
    }

    #[test]
    fn skips_layout_commands() {
        let source = "Antes.\n#v(1.5cm)\n#line(length: 100%)\n#pagebreak()\nDepois.";
        let text = extract_plain_text(source);
        assert!(!text.contains("#v("));
        assert!(!text.contains("#line("));
        assert!(!text.contains("#pagebreak"));
        assert!(text.contains("Antes."));
        assert!(text.contains("Depois."));
    }

    #[test]
    fn converts_ordered_list() {
        let source = "+ Primeiro item\n+ Segundo item";
        let text = extract_plain_text(source);
        assert!(text.contains("1. Primeiro item"));
        assert!(text.contains("1. Segundo item"));
    }

    #[test]
    fn converts_bullet_list() {
        let source = "- Item A\n- Item B";
        let text = extract_plain_text(source);
        assert!(text.contains("• Item A"));
        assert!(text.contains("• Item B"));
    }

    #[test]
    fn collapses_multiple_blank_lines() {
        let source = "A\n\n\n\nB";
        let text = extract_plain_text(source);
        assert_eq!(text, "A\n\nB");
    }

    #[test]
    fn render_text_end_to_end() {
        let template = "= Declaração\n\nEu, *{{nome}}*, NIF {{NIF}}.\n\n- Cláusula 1\n\n+ Item 1";
        let result = render_text(template, &[("nome", "Ana Silva"), ("NIF", "501964843")]);
        assert_eq!(
            result,
            "Declaração\n\nEu, Ana Silva, NIF 501964843.\n\n• Cláusula 1\n\n1. Item 1"
        );
    }
}
