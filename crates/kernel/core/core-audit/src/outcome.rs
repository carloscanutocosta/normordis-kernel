use serde::{Deserialize, Serialize};

/// Resultado observável da operação auditada.
///
/// # Enquadramento COSO
///
/// O COSO (Committee of Sponsoring Organizations — Internal Control Integrated Framework)
/// exige que os auditores possam responder à pergunta:
///
/// > **"O controlo foi executado com sucesso?"**
///
/// O `AuditOutcome` é a resposta estruturada a essa pergunta. É distinto do facto de o
/// evento ter sido gravado — o evento é **sempre** gravado com fidelidade, independentemente
/// do resultado da operação que o originou. Um evento com `Failure` é tão valioso para a
/// auditoria quanto um evento com `Success`: ambos constituem evidência verificável.
///
/// # Uso
///
/// ```rust
/// use core_audit::AuditOutcome;
///
/// // Operação concluída normalmente
/// let _ = AuditOutcome::Success;
///
/// // Tentativa de acesso negada — evento igualmente gravado
/// let _ = AuditOutcome::Failure;
///
/// // Importação parcial: 80 de 100 registos processados
/// let _ = AuditOutcome::PartialSuccess;
///
/// // Evento informativo sem operação associada
/// let _ = AuditOutcome::NotApplicable;
/// ```
///
/// # Serialização
///
/// Serializa em `snake_case` para JSON. Quando o valor é [`NotApplicable`] (o valor por
/// omissão), o campo é **omitido da serialização** para preservar compatibilidade com
/// registos criados antes da introdução deste campo.
///
/// [`NotApplicable`]: AuditOutcome::NotApplicable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// A operação completou-se com todos os efeitos pretendidos.
    ///
    /// Corresponde ao cenário nominal de execução de um controlo COSO: o controlo
    /// foi executado, o actor tinha autoridade, e o efeito foi produzido.
    Success,

    /// A operação falhou; nenhum efeito permanente foi produzido.
    ///
    /// Inclui falhas de validação, rejeições de autorização, erros de negócio e
    /// qualquer outro resultado em que a operação não tenha surtido efeito. O registo
    /// de falhas é crítico para a monitorização COSO: uma série de falhas pode indicar
    /// tentativas de contornar controlos.
    Failure,

    /// A operação completou-se parcialmente; alguns efeitos ocorreram, outros não.
    ///
    /// Adequado para operações em lote ou transacções com múltiplas partes independentes
    /// onde parte do trabalho foi concluída antes de um erro. O campo `details_json` deve
    /// descrever quais partes foram concluídas e quais falharam.
    PartialSuccess,

    /// O resultado não é aplicável para este tipo de evento.
    ///
    /// Usado para eventos puramente informativos, de rastreabilidade, ou de ciclo de vida
    /// do sistema que não correspondem a uma operação com resultado binário. É o valor
    /// por omissão (`Default`) e é omitido da serialização JSON quando presente, garantindo
    /// compatibilidade retroactiva com registos anteriores a este campo.
    #[default]
    NotApplicable,
}

impl AuditOutcome {
    /// Devolve `true` se o outcome é [`NotApplicable`].
    ///
    /// Usado internamente como predicado `skip_serializing_if` para preservar a
    /// forma canónica de serialização de eventos que não especificam um resultado.
    ///
    /// [`NotApplicable`]: AuditOutcome::NotApplicable
    pub fn is_not_applicable(&self) -> bool {
        *self == Self::NotApplicable
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn default_is_not_applicable() {
        assert_eq!(AuditOutcome::default(), AuditOutcome::NotApplicable);
    }

    #[test]
    fn serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_value(AuditOutcome::Success).unwrap(),
            json!("success")
        );
        assert_eq!(
            serde_json::to_value(AuditOutcome::Failure).unwrap(),
            json!("failure")
        );
        assert_eq!(
            serde_json::to_value(AuditOutcome::PartialSuccess).unwrap(),
            json!("partial_success")
        );
        assert_eq!(
            serde_json::to_value(AuditOutcome::NotApplicable).unwrap(),
            json!("not_applicable")
        );
    }

    #[test]
    fn deserializes_from_snake_case() {
        assert_eq!(
            serde_json::from_value::<AuditOutcome>(json!("success")).unwrap(),
            AuditOutcome::Success
        );
        assert_eq!(
            serde_json::from_value::<AuditOutcome>(json!("failure")).unwrap(),
            AuditOutcome::Failure
        );
        assert_eq!(
            serde_json::from_value::<AuditOutcome>(json!("partial_success")).unwrap(),
            AuditOutcome::PartialSuccess
        );
        assert_eq!(
            serde_json::from_value::<AuditOutcome>(json!("not_applicable")).unwrap(),
            AuditOutcome::NotApplicable
        );
    }

    #[test]
    fn is_not_applicable_only_for_not_applicable() {
        assert!(AuditOutcome::NotApplicable.is_not_applicable());
        assert!(!AuditOutcome::Success.is_not_applicable());
        assert!(!AuditOutcome::Failure.is_not_applicable());
        assert!(!AuditOutcome::PartialSuccess.is_not_applicable());
    }
}
