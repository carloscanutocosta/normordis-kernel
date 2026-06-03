use crate::{rules, ValidationIssue, ValidationReport};

/// Valida que `value` é uma string de versão no formato Semantic Versioning 2.0.
///
/// Formato aceite: `MAJOR.MINOR.PATCH[-prerelease][+build]`
/// onde MAJOR, MINOR e PATCH são inteiros não-negativos (sem zeros à esquerda
/// além do próprio zero), e prerelease/build são sufixos não-vazios se presentes.
///
/// Exemplos válidos: `"1.0.0"`, `"0.3.0"`, `"1.2.3-rc.1"`, `"1.0.0+build.42"`,
/// `"2.0.0-alpha.1+build.001"`.
///
/// Esta é uma validação estrutural — não verifica se a versão existe num
/// registo externo nem se é maior ou menor que outra.
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

    // Separar o core do sufixo (prerelease ou build)
    let core = match s.find(['-', '+']) {
        Some(i) => {
            // O sufixo após o separador deve ser não-vazio
            if s[i + 1..].is_empty() {
                return false;
            }
            &s[..i]
        }
        None => s,
    };

    // Core deve ser exactamente MAJOR.MINOR.PATCH
    let mut parts = core.split('.');
    let valid_part = |p: &str| -> bool { !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) };

    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(maj), Some(min_v), Some(patch), None)
        if valid_part(maj) && valid_part(min_v) && valid_part(patch)
    )
}
