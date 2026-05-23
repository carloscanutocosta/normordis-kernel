#[cfg(not(feature = "native-pkcs11"))]
fn main() {
    eprintln!("execute com --features native-pkcs11");
}

#[cfg(feature = "native-pkcs11")]
fn main() -> infra_signing::Result<()> {
    let module_path = std::env::args()
        .nth(1)
        .or_else(|| {
            infra_signing::citizen_card_pkcs11_module_candidates()
                .into_iter()
                .find(|path| std::path::Path::new(path).exists())
                .map(str::to_string)
        })
        .ok_or_else(|| infra_signing::SigningError::InvalidValue {
            field: "module_path",
            reason: "módulo PKCS#11 não encontrado nos caminhos conhecidos",
        })?;

    let probe = infra_signing::probe_pkcs11_module(module_path)?;
    println!("{probe:#?}");
    Ok(())
}
