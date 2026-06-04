use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é uma string de versão no formato Semantic Versioning 2.0.
///
/// Formato aceite: `MAJOR.MINOR.PATCH[-prerelease][+build]`
/// - MAJOR, MINOR e PATCH são inteiros não-negativos sem zeros à esquerda
///   (excepto o próprio `"0"`).
/// - Identificadores de prerelease e build são dot-separated, não-vazios,
///   com caracteres `[0-9A-Za-z-]` em cada segmento.
///
/// Exemplos válidos: `"1.0.0"`, `"0.3.0"`, `"1.2.3-rc.1"`, `"1.0.0+build.42"`,
/// `"2.0.0-alpha.1+build.001"`.
///
/// Esta é uma validação estrutural — não verifica existência em registos externos
/// nem ordena versões.
pub fn validate_semver(field: impl Into<String>, value: &str) -> ValidationReport {
    let field = field.into();
    if is_semver(value) {
        ValidationReport::ok()
    } else {
        ValidationReport::with_issue(ValidationIssue::error(
            rules::SEMVER_FORMAT,
            field,
            "version must be in semver format (MAJOR.MINOR.PATCH[-prerelease][+build])",
        ))
    }
}

fn is_semver(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Separar core do resto (prerelease e/ou build)
    let core_end = s.find(['-', '+']).unwrap_or(s.len());
    let core = &s[..core_end];
    let rest = &s[core_end..];

    // Core: MAJOR.MINOR.PATCH — sem zeros à esquerda
    let mut parts = core.split('.');
    let valid_numeric = |p: &str| -> bool {
        !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && (p == "0" || !p.starts_with('0'))
    };
    if !matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(maj), Some(min_v), Some(patch), None)
        if valid_numeric(maj) && valid_numeric(min_v) && valid_numeric(patch)
    ) {
        return false;
    }

    if rest.is_empty() {
        return true;
    }

    // Separar prerelease (após '-') de build (após '+')
    let (prerelease, build) = if let Some(pre_rest) = rest.strip_prefix('-') {
        match pre_rest.find('+') {
            Some(i) => (Some(&pre_rest[..i]), Some(&pre_rest[i + 1..])),
            None => (Some(pre_rest), None),
        }
    } else {
        // rest starts with '+'
        (None, rest.strip_prefix('+'))
    };

    // Cada segmento dot-separated: não-vazio, só [0-9A-Za-z-]
    let valid_identifier =
        |seg: &str| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');

    let valid_suffix = |sfx: Option<&str>| match sfx {
        None => true,
        Some(s) => !s.is_empty() && s.split('.').all(valid_identifier),
    };

    valid_suffix(prerelease) && valid_suffix(build)
}
