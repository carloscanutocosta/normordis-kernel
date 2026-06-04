use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é uma string de versão no formato Semantic Versioning 2.0.
///
/// Formato aceite: `MAJOR.MINOR.PATCH[-prerelease][+build]`
/// - MAJOR, MINOR e PATCH: inteiros não-negativos sem zeros à esquerda (excepto `"0"`).
/// - Prerelease: identificadores dot-separated `[0-9A-Za-z-]`, não-vazios.
///   Identificadores **puramente numéricos** não podem ter zeros à esquerda
///   (`"01"` inválido; `"0"` e `"1"` válidos). Spec: SemVer 2.0 §9.
/// - Build metadata: identificadores dot-separated `[0-9A-Za-z-]`, não-vazios.
///   Zeros à esquerda são permitidos em build metadata.
///
/// Exemplos válidos: `"1.0.0"`, `"0.3.0"`, `"1.2.3-rc.1"`, `"1.0.0+build.42"`,
/// `"2.0.0-alpha.1+build.001"` (build metadata pode ter zeros à esquerda).
///
/// Exemplos inválidos: `"1.0.0-01"` (prerelease numérico com zero à esquerda),
/// `"01.0.0"` (MAJOR com zero à esquerda), `"1.0.0-rc!1"` (char inválido).
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
    let valid_numeric = |p: &str| -> bool {
        !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && (p == "0" || !p.starts_with('0'))
    };
    let mut parts = core.split('.');
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
        (None, rest.strip_prefix('+'))
    };

    // Prerelease: [0-9A-Za-z-]+, sem zeros à esquerda em identificadores numéricos (SemVer §9)
    let valid_prerelease_id = |seg: &str| -> bool {
        !seg.is_empty()
            && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && if seg.chars().all(|c| c.is_ascii_digit()) {
                seg == "0" || !seg.starts_with('0')
            } else {
                true
            }
    };

    // Build metadata: [0-9A-Za-z-]+, zeros à esquerda permitidos
    let valid_build_id =
        |seg: &str| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');

    let valid_pre = |sfx: Option<&str>| match sfx {
        None => true,
        Some(s) => !s.is_empty() && s.split('.').all(valid_prerelease_id),
    };
    let valid_build = |sfx: Option<&str>| match sfx {
        None => true,
        Some(s) => !s.is_empty() && s.split('.').all(valid_build_id),
    };

    valid_pre(prerelease) && valid_build(build)
}
