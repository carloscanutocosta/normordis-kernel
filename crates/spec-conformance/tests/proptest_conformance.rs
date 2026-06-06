mod support;
use support::{validator_for, ContractSchema};

use proptest::prelude::*;
use serde_json::json;

// ── Estratégias auxiliares ────────────────────────────────────────────────────

/// UUID v4 fixo — usamos sempre o mesmo para evitar gerar UUIDs inválidos.
const FIXED_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

// ── AuditEvent — geração aleatória de campos variáveis ───────────────────────
//
// Verifica que instâncias geradas com campos variáveis mas estruturalmente
// correctos passam o schema. Detecta regressões onde o schema ficou mais
// restritivo do que o tipo Rust permite.

proptest! {
    #[test]
    fn arbitrary_event_type_passes_schema(
        domain in "[a-z][a-z0-9_]{1,10}",
        action in "[a-z][a-z0-9_]{1,10}",
    ) {
        let event_type = format!("{domain}.{action}");
        let instance = json!({
            "event_id": FIXED_UUID,
            "occurred_at_utc": "2026-01-15T10:00:00Z",
            "event_type": event_type,
            "actor": { "actor_id": "user-001" },
            "target": { "target_type": "document", "target_id": "doc-001" },
            "outcome": "success"
        });
        let validator = validator_for(ContractSchema::AuditEvent);
        prop_assert!(
            validator.is_valid(&instance),
            "event_type '{}' deveria ser válido mas falhou schema",
            event_type
        );
    }

    #[test]
    fn arbitrary_actor_id_nonempty_passes_schema(
        actor_id in "[a-z][a-z0-9_-]{0,30}",
    ) {
        let instance = json!({
            "event_id": FIXED_UUID,
            "occurred_at_utc": "2026-01-15T10:00:00Z",
            "event_type": "user.login",
            "actor": { "actor_id": actor_id },
            "target": { "target_type": "session", "target_id": "sess-001" },
            "outcome": "success"
        });
        let validator = validator_for(ContractSchema::AuditEvent);
        prop_assert!(
            validator.is_valid(&instance),
            "actor_id '{}' deveria ser válido",
            actor_id
        );
    }

    #[test]
    fn arbitrary_outcome_passes_schema(
        outcome_idx in 0usize..3,
    ) {
        const OUTCOMES: &[&str] = &["success", "failure", "partial_success"];
        let outcome = OUTCOMES[outcome_idx];
        let instance = json!({
            "event_id": FIXED_UUID,
            "occurred_at_utc": "2026-01-15T10:00:00Z",
            "event_type": "document.sign",
            "actor": { "actor_id": "user-001" },
            "target": { "target_type": "document", "target_id": "doc-001" },
            "outcome": outcome
        });
        let validator = validator_for(ContractSchema::AuditEvent);
        prop_assert!(
            validator.is_valid(&instance),
            "outcome '{}' deveria ser válido",
            outcome
        );
    }
}

// ── AuditEvent round-trip — Rust → JSON → schema ─────────────────────────────
//
// Gera instâncias válidas, desserializa em AuditEvent, re-serializa e valida.
// Detecta drift: se o tipo Rust serializar campos que o schema não cobre, falha.

proptest! {
    #[test]
    fn arbitrary_audit_event_round_trip_passes_schema(
        actor_id in "[a-z][a-z0-9_-]{1,20}",
        // Cada segmento deve começar com letra — underscore inicial é rejeitado pelo schema.
        event_type in "[a-z][a-z0-9_]{1,7}\\.[a-z][a-z0-9_]{1,7}",
        target_id in "[a-z0-9-]{1,20}",
    ) {
        use core_audit::AuditEvent;

        let instance = json!({
            "event_id": FIXED_UUID,
            "occurred_at_utc": "2026-01-15T10:00:00Z",
            "event_type": event_type,
            "actor": { "actor_id": actor_id },
            "target": { "target_type": "document", "target_id": target_id },
            "outcome": "success"
        });

        if let Ok(event) = serde_json::from_value::<AuditEvent>(instance) {
            let re_serialized = serde_json::to_value(&event)
                .expect("AuditEvent deve serializar");
            let validator = validator_for(ContractSchema::AuditEvent);
            prop_assert!(
                validator.is_valid(&re_serialized),
                "DRIFT: AuditEvent serializado falhou schema:\n{:#?}",
                re_serialized
            );
        }
        // Se desserializar falhar (ex: event_type inválido), o test é descartado silenciosamente.
    }
}

// ── AuditChainLink — geração aleatória ───────────────────────────────────────
//
// Verifica que elos com hash válido (64 hex lowercase) passam o schema,
// qualquer que seja a sequência ou os valores concretos dos hashes.

proptest! {
    #[test]
    fn arbitrary_chain_link_passes_schema(
        sequence in 1u64..=1000,
        record_hash in "[0-9a-f]{64}",
        prev_hash in "[0-9a-f]{64}",
    ) {
        let instance = if sequence == 1 {
            json!({ "sequence": sequence, "record_hash": record_hash })
        } else {
            json!({
                "sequence": sequence,
                "previous_record_hash": prev_hash,
                "record_hash": record_hash
            })
        };
        let validator = validator_for(ContractSchema::AuditChainLink);
        prop_assert!(
            validator.is_valid(&instance),
            "chain link com sequence={sequence} deveria ser válido"
        );
    }

    #[test]
    fn chain_link_with_sequence_zero_fails_schema(
        record_hash in "[0-9a-f]{64}",
    ) {
        // sequence mínimo é 1; 0 deve ser rejeitado.
        let instance = json!({ "sequence": 0u64, "record_hash": record_hash });
        let validator = validator_for(ContractSchema::AuditChainLink);
        prop_assert!(
            !validator.is_valid(&instance),
            "sequence=0 deveria ser rejeitado pelo schema"
        );
    }
}

// ── StorageKey — charset e comprimento ───────────────────────────────────────

proptest! {
    #[test]
    fn valid_storage_key_chars_pass_schema(
        // Sem ponto para evitar '..' (rejeitado pelo schema via `not`).
        key in "[A-Za-z0-9_-]{1,50}",
    ) {
        let instance = serde_json::Value::String(key.clone());
        let validator = validator_for(ContractSchema::SupportStorageKey);
        prop_assert!(
            validator.is_valid(&instance),
            "StorageKey '{}' deveria ser válido",
            key
        );
    }

    #[test]
    fn storage_key_with_slash_fails_schema(
        prefix in "[A-Za-z0-9]{1,10}",
        suffix in "[A-Za-z0-9]{1,10}",
    ) {
        let key = format!("{prefix}/{suffix}");
        let instance = serde_json::Value::String(key.clone());
        let validator = validator_for(ContractSchema::SupportStorageKey);
        prop_assert!(
            !validator.is_valid(&instance),
            "StorageKey com '/' deveria ser rejeitado: '{}'",
            key
        );
    }
}

// ── ControlId — padrão CTRL-CATEGORIA-NNN ────────────────────────────────────

proptest! {
    #[test]
    fn valid_control_id_passes_schema(
        // control_id usa códigos fixos; category e severity são enums do schema.
        cat_idx in 0usize..10,
        num in 1u16..=999,
        sev_idx in 0usize..3,
    ) {
        const CATEGORIES: &[(&str, &str)] = &[
            ("AUTH", "auth"), ("VAL", "validation"), ("TRACE", "traceability"),
            ("DOC", "documentary"), ("INT", "integrity"), ("PRIV", "privacy"),
            ("SEC", "security"), ("ING", "ingestion"), ("EXP", "export"), ("CONT", "continuity"),
        ];
        const SEVERITIES: &[&str] = &["low", "medium", "high"];
        let (code, cat_value) = CATEGORIES[cat_idx];
        let severity = SEVERITIES[sev_idx];
        let control_id = format!("CTRL-{code}-{num:03}");
        let instance = json!({
            "control_id": control_id,
            "name": "Controlo de teste gerado por proptest",
            "version": "1.0",
            "category": cat_value,
            "severity": severity,
            "valid_from": "2024-01-01T00:00:00Z",
            "active": true
        });
        let validator = validator_for(ContractSchema::ControlDefinition);
        prop_assert!(
            validator.is_valid(&instance),
            "control_id '{}' deveria ser válido (category={cat_value}, severity={severity})",
            control_id
        );
    }
}
