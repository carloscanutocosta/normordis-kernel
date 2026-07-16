use std::path::PathBuf;

/// Caminho absoluto para a raiz de normordis-spec.
///
/// Resolução por ordem de precedência:
///
/// 1. Variável de ambiente `NORMORDIS_SPEC_PATH` — usada quando normordis-spec vive
///    num repo próprio (clone, submodule, artifact de CI).
///    Exemplo: `NORMORDIS_SPEC_PATH=/workspace/normordis-spec cargo test`
///
/// 2. Caminho relativo ao manifest deste crate — fallback para desenvolvimento
///    local enquanto normordis-spec coexiste em normordis-kernel.
pub fn spec_root() -> PathBuf {
    if let Ok(path) = std::env::var("NORMORDIS_SPEC_PATH") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../normordis-spec")
}
