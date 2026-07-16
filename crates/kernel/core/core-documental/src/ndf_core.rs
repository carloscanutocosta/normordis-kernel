//! Tipos e validador do NDF-core (NORMORDIS Document Format) conforme spec-ndf-v1.0.0.
//!
//! Valida estrutura (JSON Schema) e semântica (trait `ListaConsolidadaLookup`) do
//! conteúdo documental antes de entrar no pipeline de finalização (JCS + CAdES).

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Constante de versão ───────────────────────────────────────────────────────

/// Major máximo suportado por este leitor NDF conforme §7.2 da especificação.
pub const MAX_SUPPORTED_NDF_VERSION_MAJOR: u64 = 1;

// ── Schema embutido ───────────────────────────────────────────────────────────

const NDF_CORE_SCHEMA_STR: &str =
    include_str!("../../../../../normordis-spec/schemas/core/documental/ndf-core.schema.json");

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnidadePrazo {
    Dias,
    Meses,
    Anos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormaContagem {
    DataDocumento,
    EncerramentoProcesso,
    FimAnoCivil,
    FimVigencia,
    Outro,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestinoFinal {
    ConservacaoPermanente,
    Eliminacao,
    ConservacaoParcialPorAmostragem,
}

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrazoConservacao {
    pub valor: u32,
    pub unidade: UnidadePrazo,
    pub forma_contagem: FormaContagem,
    pub forma_contagem_detalhe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Avaliacao {
    pub tipo_classificacao_ref: String,
    pub prazo_conservacao_administrativa: PrazoConservacao,
    pub destino_final: DestinoFinal,
    pub lista_consolidada_versao_ref: String,
}

/// NDF-core desserializado — conteúdo lógico + metadados + avaliação arquivística.
///
/// `documento` e `metadados` são `Value` opaco porque a estrutura interna
/// depende do tipo documental, fora do âmbito deste schema base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NdfCore {
    pub ndf_version: String,
    pub documento: Value,
    pub metadados: Value,
    pub ndt_version_ref: String,
    pub avaliacao: Avaliacao,
}

// ── ListaConsolidadaLookup ────────────────────────────────────────────────────

/// Entrada resolvida da Lista Consolidada para um tipo de classificação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListaConsolidadaEntry {
    pub pca_valor: u32,
    pub pca_unidade: UnidadePrazo,
    pub pca_forma_contagem: FormaContagem,
    pub destino_final: DestinoFinal,
}

/// Porta de lookup na Lista Consolidada DGLAB — injectada no validador.
///
/// Resolve `tipo_classificacao_ref` + versão da lista para os valores normativos
/// de PCA e destino final. O validador não acopla à implementação concreta de
/// carregamento da Lista Consolidada.
pub trait ListaConsolidadaLookup {
    fn lookup(
        &self,
        tipo_classificacao_ref: &str,
        lista_versao_ref: &str,
    ) -> Option<ListaConsolidadaEntry>;
}

// ── NdfValidationError ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NdfValidationError {
    #[error("violação de schema: {0}")]
    SchemaViolation(String),

    #[error("versão NDF não suportada: '{found}' tem major > {max_supported_major}")]
    NdfVersionUnsupported {
        found: String,
        max_supported_major: u64,
    },

    #[error("tipo_classificacao_ref '{0}' não encontrado na Lista Consolidada")]
    TipoClassificacaoNaoEncontrado(String),

    #[error(
        "PCA/DF inconsistentes para tipo '{tipo_ref}': \
        Lista Consolidada determina PCA={pca_valor} {pca_unidade:?} ({pca_forma_contagem:?}) \
        e DF={destino_final:?}"
    )]
    PcaDfInconsistente {
        tipo_ref: String,
        pca_valor: u32,
        pca_unidade: UnidadePrazo,
        pca_forma_contagem: FormaContagem,
        destino_final: DestinoFinal,
    },

    #[error("forma_contagem_detalhe obrigatório quando forma_contagem = 'outro'")]
    FormaContagemDetalheObrigatorio,
}

// ── validate_ndf_core ─────────────────────────────────────────────────────────

/// Valida um NDF-core antes de entrar no pipeline de finalização (JCS + CAdES).
///
/// Aplica validação em três camadas sequenciais:
/// 1. JSON Schema — estrutura e tipos (rejeita cedo se inválido)
/// 2. Desserialização para `NdfCore`
/// 3. Semântica — versão, Lista Consolidada, PCA/DF, forma_contagem_detalhe
///
/// Retorna *todos* os erros semânticos encontrados (não para no primeiro), para
/// que o chamador possa reportar feedback completo ao utilizador/sistema.
pub fn validate_ndf_core(
    raw: &Value,
    lista_consolidada: &impl ListaConsolidadaLookup,
) -> Result<NdfCore, Vec<NdfValidationError>> {
    let mut errors: Vec<NdfValidationError> = Vec::new();

    // 1. Compilar e validar contra JSON Schema
    let schema_value: Value = serde_json::from_str(NDF_CORE_SCHEMA_STR)
        .expect("NDF_CORE_SCHEMA_STR inválido — erro de build");

    let validator = jsonschema::options()
        .build(&schema_value)
        .expect("schema NDF-core inválido — erro de build");

    let schema_errors: Vec<String> = validator.iter_errors(raw).map(|e| e.to_string()).collect();

    if !schema_errors.is_empty() {
        return Err(schema_errors
            .into_iter()
            .map(NdfValidationError::SchemaViolation)
            .collect());
    }

    // 2. Desserializar para NdfCore
    let ndf: NdfCore = match serde_json::from_value(raw.clone()) {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![NdfValidationError::SchemaViolation(format!(
                "falha de desserialização após schema válido: {e}"
            ))]);
        }
    };

    // 3a. ndf_version: major suportado (§7.2)
    match parse_semver_major(&ndf.ndf_version) {
        Some(major) if major > MAX_SUPPORTED_NDF_VERSION_MAJOR => {
            errors.push(NdfValidationError::NdfVersionUnsupported {
                found: ndf.ndf_version.clone(),
                max_supported_major: MAX_SUPPORTED_NDF_VERSION_MAJOR,
            });
        }
        None => {
            errors.push(NdfValidationError::SchemaViolation(format!(
                "ndf_version '{}' não é semver válido",
                ndf.ndf_version
            )));
        }
        _ => {}
    }

    // 3b. forma_contagem_detalhe obrigatório quando forma_contagem == Outro (§3.3)
    let pca = &ndf.avaliacao.prazo_conservacao_administrativa;
    if pca.forma_contagem == FormaContagem::Outro && pca.forma_contagem_detalhe.is_none() {
        errors.push(NdfValidationError::FormaContagemDetalheObrigatorio);
    }

    // 3c. tipo_classificacao_ref existe na Lista Consolidada + PCA/DF consistentes (§3.5)
    let tipo_ref = &ndf.avaliacao.tipo_classificacao_ref;
    let lista_versao = &ndf.avaliacao.lista_consolidada_versao_ref;

    match lista_consolidada.lookup(tipo_ref, lista_versao) {
        None => {
            errors.push(NdfValidationError::TipoClassificacaoNaoEncontrado(
                tipo_ref.clone(),
            ));
        }
        Some(entry) => {
            let df = &ndf.avaliacao.destino_final;
            if pca.valor != entry.pca_valor
                || pca.unidade != entry.pca_unidade
                || pca.forma_contagem != entry.pca_forma_contagem
                || *df != entry.destino_final
            {
                errors.push(NdfValidationError::PcaDfInconsistente {
                    tipo_ref: tipo_ref.clone(),
                    pca_valor: entry.pca_valor,
                    pca_unidade: entry.pca_unidade,
                    pca_forma_contagem: entry.pca_forma_contagem,
                    destino_final: entry.destino_final,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(ndf)
    } else {
        Err(errors)
    }
}

fn parse_semver_major(version: &str) -> Option<u64> {
    version.split('.').next()?.parse().ok()
}

// ── Testes ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Mock da Lista Consolidada ─────────────────────────────────────────────

    struct MockLista;

    impl ListaConsolidadaLookup for MockLista {
        fn lookup(&self, tipo: &str, _versao: &str) -> Option<ListaConsolidadaEntry> {
            match tipo {
                "PROC.01" => Some(ListaConsolidadaEntry {
                    pca_valor: 5,
                    pca_unidade: UnidadePrazo::Anos,
                    pca_forma_contagem: FormaContagem::EncerramentoProcesso,
                    destino_final: DestinoFinal::Eliminacao,
                }),
                "850.10.303" => Some(ListaConsolidadaEntry {
                    pca_valor: 10,
                    pca_unidade: UnidadePrazo::Anos,
                    pca_forma_contagem: FormaContagem::FimVigencia,
                    destino_final: DestinoFinal::ConservacaoPermanente,
                }),
                _ => None,
            }
        }
    }

    // ── Fixture válido base ───────────────────────────────────────────────────

    fn valid_raw() -> Value {
        json!({
            "ndf_version": "1.0.0",
            "documento": { "assunto": "Ofício de teste" },
            "metadados": { "entidade": "IGAC" },
            "ndt_version_ref": "oficio@v1.0.0",
            "avaliacao": {
                "tipo_classificacao_ref": "PROC.01",
                "prazo_conservacao_administrativa": {
                    "valor": 5,
                    "unidade": "anos",
                    "forma_contagem": "encerramento_processo"
                },
                "destino_final": "eliminacao",
                "lista_consolidada_versao_ref": "lista-consolidada-v2024"
            }
        })
    }

    // ── Testes do JSON Schema ─────────────────────────────────────────────────

    fn schema_validator() -> jsonschema::Validator {
        let schema: Value = serde_json::from_str(NDF_CORE_SCHEMA_STR).expect("schema inválido");
        jsonschema::options()
            .build(&schema)
            .expect("falha ao compilar schema")
    }

    #[test]
    fn schema_aceita_ndf_core_minimo_valido() {
        let v = schema_validator();
        assert!(v.is_valid(&valid_raw()));
    }

    #[test]
    fn schema_rejeita_falta_de_ndf_version() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("ndf_version");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_documento() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("documento");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_metadados() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("metadados");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_ndt_version_ref() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("ndt_version_ref");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_avaliacao() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("avaliacao");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_tipo_classificacao_ref() {
        let mut raw = valid_raw();
        raw["avaliacao"]
            .as_object_mut()
            .unwrap()
            .remove("tipo_classificacao_ref");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_prazo_conservacao_administrativa() {
        let mut raw = valid_raw();
        raw["avaliacao"]
            .as_object_mut()
            .unwrap()
            .remove("prazo_conservacao_administrativa");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_destino_final() {
        let mut raw = valid_raw();
        raw["avaliacao"]
            .as_object_mut()
            .unwrap()
            .remove("destino_final");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_falta_de_lista_consolidada_versao_ref() {
        let mut raw = valid_raw();
        raw["avaliacao"]
            .as_object_mut()
            .unwrap()
            .remove("lista_consolidada_versao_ref");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_unidade_fora_do_enum() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["unidade"] = json!("semanas");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_forma_contagem_fora_do_enum() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem"] =
            json!("desde_sempre");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_destino_final_fora_do_enum() {
        let mut raw = valid_raw();
        raw["avaliacao"]["destino_final"] = json!("destruicao");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_campo_extra_no_root() {
        let mut raw = valid_raw();
        raw["campo_intruso"] = json!("valor");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_campo_extra_em_avaliacao() {
        let mut raw = valid_raw();
        raw["avaliacao"]["campo_intruso"] = json!("valor");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_campo_extra_em_prazo() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["campo_intruso"] = json!("valor");
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_rejeita_valor_negativo() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["valor"] = json!(-1);
        assert!(!schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_aceita_forma_contagem_outro_com_detalhe() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem"] = json!("outro");
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem_detalhe"] =
            json!("A partir da publicação em DR");
        assert!(schema_validator().is_valid(&raw));
    }

    #[test]
    fn schema_aceita_forma_contagem_outro_sem_detalhe() {
        // O schema não impõe a regra — é da responsabilidade do validador Rust.
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem"] = json!("outro");
        assert!(schema_validator().is_valid(&raw));
    }

    // ── Testes do validador Rust ──────────────────────────────────────────────

    #[test]
    fn validador_aceita_ndf_core_valido_completo() {
        let result = validate_ndf_core(&valid_raw(), &MockLista);
        assert!(result.is_ok());
        let ndf = result.unwrap();
        assert_eq!(ndf.ndf_version, "1.0.0");
        assert_eq!(ndf.avaliacao.tipo_classificacao_ref, "PROC.01");
        assert_eq!(ndf.avaliacao.destino_final, DestinoFinal::Eliminacao);
    }

    #[test]
    fn validador_aceita_ndf_v1_minor_futuro() {
        let mut raw = valid_raw();
        raw["ndf_version"] = json!("1.5.0");
        assert!(validate_ndf_core(&raw, &MockLista).is_ok());
    }

    #[test]
    fn validador_aceita_forma_contagem_outro_com_detalhe() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem"] = json!("outro");
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem_detalhe"] =
            json!("A partir da publicação em DR");
        // Mock não tem "PROC.01" com forma_contagem=Outro — ajustar pca para evitar PcaDfInconsistente
        // Aqui testamos apenas que forma_contagem_detalhe presente não gera FormaContagemDetalheObrigatorio
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(!errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::FormaContagemDetalheObrigatorio)));
    }

    #[test]
    fn validador_rejeita_schema_violacao_campo_obrigatorio() {
        let mut raw = valid_raw();
        raw.as_object_mut().unwrap().remove("ndf_version");
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::SchemaViolation(_))));
    }

    #[test]
    fn validador_rejeita_ndf_version_major_superior() {
        let mut raw = valid_raw();
        raw["ndf_version"] = json!("2.0.0");
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            NdfValidationError::NdfVersionUnsupported {
                max_supported_major: 1,
                ..
            }
        )));
    }

    #[test]
    fn validador_rejeita_tipo_classificacao_nao_encontrado() {
        let mut raw = valid_raw();
        raw["avaliacao"]["tipo_classificacao_ref"] = json!("TIPO-INEXISTENTE");
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs.iter().any(|e| matches!(
            e,
            NdfValidationError::TipoClassificacaoNaoEncontrado(t) if t == "TIPO-INEXISTENTE"
        )));
    }

    #[test]
    fn validador_rejeita_pca_inconsistente_valor() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["valor"] = json!(99);
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::PcaDfInconsistente { .. })));
    }

    #[test]
    fn validador_rejeita_pca_inconsistente_destino_final() {
        let mut raw = valid_raw();
        raw["avaliacao"]["destino_final"] = json!("conservacao_permanente");
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::PcaDfInconsistente { .. })));
    }

    #[test]
    fn validador_rejeita_forma_contagem_detalhe_ausente_quando_outro() {
        let mut raw = valid_raw();
        raw["avaliacao"]["prazo_conservacao_administrativa"]["forma_contagem"] = json!("outro");
        // sem forma_contagem_detalhe
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::FormaContagemDetalheObrigatorio)));
    }

    #[test]
    fn validador_retorna_multiplos_erros_simultaneamente() {
        // ndf_version 2.0.0 → NdfVersionUnsupported
        // tipo_classificacao_ref inexistente → TipoClassificacaoNaoEncontrado
        let raw = json!({
            "ndf_version": "2.0.0",
            "documento": {},
            "metadados": {},
            "ndt_version_ref": "oficio@v1.0.0",
            "avaliacao": {
                "tipo_classificacao_ref": "TIPO-INEXISTENTE",
                "prazo_conservacao_administrativa": {
                    "valor": 5,
                    "unidade": "anos",
                    "forma_contagem": "encerramento_processo"
                },
                "destino_final": "eliminacao",
                "lista_consolidada_versao_ref": "lista-consolidada-v2024"
            }
        });
        let errs = validate_ndf_core(&raw, &MockLista).unwrap_err();
        assert!(errs.len() >= 2, "esperados >= 2 erros, obtidos: {errs:?}");
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::NdfVersionUnsupported { .. })));
        assert!(errs
            .iter()
            .any(|e| matches!(e, NdfValidationError::TipoClassificacaoNaoEncontrado(_))));
    }

    #[test]
    fn validador_aceita_fixture_full_com_850_10_303() {
        let raw = json!({
            "ndf_version": "1.0.0",
            "documento": {
                "assunto": "Licença de utilização — obra ref. 2026/LU/042"
            },
            "metadados": {
                "entidade": "Câmara Municipal de Lisboa"
            },
            "ndt_version_ref": "licenca-utilizacao@v2.3.1",
            "avaliacao": {
                "tipo_classificacao_ref": "850.10.303",
                "prazo_conservacao_administrativa": {
                    "valor": 10,
                    "unidade": "anos",
                    "forma_contagem": "fim_vigencia"
                },
                "destino_final": "conservacao_permanente",
                "lista_consolidada_versao_ref": "lista-consolidada-dglab-v2024"
            }
        });
        assert!(validate_ndf_core(&raw, &MockLista).is_ok());
    }

    // ── parse_semver_major ────────────────────────────────────────────────────

    #[test]
    fn parse_semver_major_extrai_corretamente() {
        assert_eq!(parse_semver_major("1.0.0"), Some(1));
        assert_eq!(parse_semver_major("2.5.3"), Some(2));
        assert_eq!(parse_semver_major("10.0.0"), Some(10));
        assert_eq!(parse_semver_major("abc"), None);
        assert_eq!(parse_semver_major(""), None);
    }

    // ── Serde round-trip ──────────────────────────────────────────────────────

    #[test]
    fn serde_round_trip_ndf_core() {
        let raw = valid_raw();
        let ndf: NdfCore = serde_json::from_value(raw.clone()).unwrap();
        let serialized = serde_json::to_value(&ndf).unwrap();
        // Campos de topo preservados
        assert_eq!(serialized["ndf_version"], raw["ndf_version"]);
        assert_eq!(
            serialized["avaliacao"]["destino_final"],
            raw["avaliacao"]["destino_final"]
        );
    }

    #[test]
    fn serde_enum_unidade_round_trip() {
        assert_eq!(
            serde_json::to_value(UnidadePrazo::Dias).unwrap(),
            json!("dias")
        );
        assert_eq!(
            serde_json::to_value(UnidadePrazo::Meses).unwrap(),
            json!("meses")
        );
        assert_eq!(
            serde_json::to_value(UnidadePrazo::Anos).unwrap(),
            json!("anos")
        );
    }

    #[test]
    fn serde_enum_destino_final_round_trip() {
        assert_eq!(
            serde_json::to_value(DestinoFinal::ConservacaoPermanente).unwrap(),
            json!("conservacao_permanente")
        );
        assert_eq!(
            serde_json::to_value(DestinoFinal::Eliminacao).unwrap(),
            json!("eliminacao")
        );
        assert_eq!(
            serde_json::to_value(DestinoFinal::ConservacaoParcialPorAmostragem).unwrap(),
            json!("conservacao_parcial_por_amostragem")
        );
    }

    #[test]
    fn serde_enum_forma_contagem_round_trip() {
        for (variant, expected) in [
            (FormaContagem::DataDocumento, "data_documento"),
            (FormaContagem::EncerramentoProcesso, "encerramento_processo"),
            (FormaContagem::FimAnoCivil, "fim_ano_civil"),
            (FormaContagem::FimVigencia, "fim_vigencia"),
            (FormaContagem::Outro, "outro"),
        ] {
            assert_eq!(
                serde_json::to_value(&variant).unwrap(),
                json!(expected),
                "falhou para {variant:?}"
            );
        }
    }
}
