//! Testes de integração realistas — NNS com persistência SQLite.
//!
//! Cada teste usa um ficheiro SQLite temporário real para verificar
//! comportamento correcto em cenários próximos da produção.
//!
//! Executar:
//!   cargo test -p domain-numerador-sqlite --test sqlite_realistic -- --nocapture

use chrono::{NaiveDate, TimeZone, Utc};
use domain_numerador::{
    ActorRef, AssignNumberRequest, AssignedStatus, AssignmentFilter, ChangeStatusRequest,
    FormatPart, NumberFormat, NumberingKind, NumberingSequence, NumberingSequenceRepository,
    NumberingStore, NumeradorDomainError, ResetPolicy, TargetRef,
};
use numerador_sqlite::NumeradorDb;
use rusqlite::Connection;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open(path: &std::path::Path) -> NumeradorDb {
    let conn = Connection::open(path).expect("abrir ligação SQLite");
    NumeradorDb::from_connection(conn).expect("inicializar NumeradorDb")
}

fn dt(year: i32, month: u32, day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 9, 0, 0).unwrap()
}

fn seq(
    sequence_id: &str,
    entity_id: &str,
    document_type: &str,
    padding: usize,
    reset: ResetPolicy,
    format_parts: Vec<FormatPart>,
    valid_from: NaiveDate,
    valid_to: Option<NaiveDate>,
) -> NumberingSequence {
    NumberingSequence {
        sequence_id: sequence_id.into(),
        kind: NumberingKind::Document,
        document_type: Some(document_type.into()),
        procedure_type: None,
        entity_id: entity_id.into(),
        org_unit_id: None,
        padding,
        reset_policy: reset,
        format: NumberFormat {
            separator: String::new(),
            parts: format_parts,
        },
        valid_from,
        valid_to,
    }
}

fn req(
    target_id: &str,
    entity_id: &str,
    document_type: &str,
    actor_id: &str,
    subject: Option<&str>,
    recipient: Option<&str>,
) -> AssignNumberRequest {
    AssignNumberRequest {
        kind: NumberingKind::Document,
        target: TargetRef {
            id: target_id.into(),
            target_type: "document".into(),
        },
        document_type: Some(document_type.into()),
        procedure_type: None,
        entity_id: entity_id.into(),
        org_unit_id: None,
        actor: ActorRef {
            id: actor_id.into(),
            name: Some(actor_id.into()),
        },
        requested_at: None,
        correlation_id: None,
        metadata: domain_numerador::AssignmentMetadata {
            subject: subject.map(str::to_string),
            recipient: recipient.map(str::to_string),
            ..Default::default()
        },
    }
}

fn oficio_seq(
    entity_id: &str,
    valid_from: NaiveDate,
    valid_to: Option<NaiveDate>,
) -> NumberingSequence {
    seq(
        &format!("{entity_id}-oficio"),
        entity_id,
        "oficio_at",
        5,
        ResetPolicy::Yearly,
        vec![
            FormatPart::Literal("OF/".into()),
            FormatPart::Period,
            FormatPart::Literal("/".into()),
            FormatPart::Sequence,
        ],
        valid_from,
        valid_to,
    )
}

fn despacho_seq(entity_id: &str, valid_from: NaiveDate) -> NumberingSequence {
    seq(
        &format!("{entity_id}-despacho"),
        entity_id,
        "despacho",
        4,
        ResetPolicy::Yearly,
        vec![
            FormatPart::Literal("DESP/".into()),
            FormatPart::Period,
            FormatPart::Literal("/".into()),
            FormatPart::Sequence,
        ],
        valid_from,
        None,
    )
}

// ─── Cenário 1: Persistência entre sessões ───────────────────────────────────

#[test]
fn persistencia_entre_sessoes() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("persist.db");
    let s = oficio_seq(
        "sf-setubal",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    );

    // Sessão 1: criar sequência e emitir 3 ofícios
    {
        let db = open(&db_path);
        db.upsert(&s).unwrap();
        let mut db = open(&db_path); // reabrir para ter &mut
        db.assign(
            &req(
                "doc-01",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Pedido de certidão"),
                Some("João Silva"),
            ),
            dt(2026, 1, 10),
            "ref-01",
        )
        .unwrap();
        db.assign(
            &req(
                "doc-02",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Resposta ACSS"),
                Some("ACSS"),
            ),
            dt(2026, 1, 15),
            "ref-02",
        )
        .unwrap();
        db.assign(
            &req(
                "doc-03",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Pedido de informação"),
                Some("ATA"),
            ),
            dt(2026, 1, 20),
            "ref-03",
        )
        .unwrap();
    } // conexão fecha aqui

    // Sessão 2: reabrir e verificar que os dados persistiram
    {
        let db = open(&db_path);
        let all = db
            .list_assignments(&AssignmentFilter::default(), 100)
            .unwrap();
        assert_eq!(all.len(), 3, "3 ofícios devem persistir entre sessões");
        assert_eq!(all[0].sequence_value, 3); // mais recente primeiro
        assert_eq!(all[2].sequence_value, 1);
        assert_eq!(all[2].number_value, "OF/2026/00001");
        assert_eq!(all[1].number_value, "OF/2026/00002");
        assert_eq!(all[0].number_value, "OF/2026/00003");

        // Emitir mais 2
        let mut db = open(&db_path);
        db.assign(
            &req("doc-04", "sf-setubal", "oficio_at", "mmatos", None, None),
            dt(2026, 2, 1),
            "ref-04",
        )
        .unwrap();
        db.assign(
            &req("doc-05", "sf-setubal", "oficio_at", "mmatos", None, None),
            dt(2026, 2, 5),
            "ref-05",
        )
        .unwrap();
    }

    // Sessão 3: verificar contador continuou do ponto correcto
    {
        let db = open(&db_path);
        let all = db
            .list_assignments(&AssignmentFilter::default(), 100)
            .unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].sequence_value, 5);
        assert_eq!(all[0].number_value, "OF/2026/00005");

        // Verificar metadados do primeiro ofício persistiram
        let first = db
            .get_by_target(&NumberingKind::Document, "doc-01")
            .unwrap()
            .unwrap();
        assert_eq!(
            first.metadata.subject.as_deref(),
            Some("Pedido de certidão")
        );
        assert_eq!(first.metadata.recipient.as_deref(), Some("João Silva"));
    }
}

// ─── Cenário 2: Isolamento entre entidades ───────────────────────────────────

#[test]
fn isolamento_entre_entidades() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("entities.db");

    let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let db = open(&db_path);
    db.upsert(&oficio_seq("sf-setubal", from, None)).unwrap();
    db.upsert(&oficio_seq("sf-sintra", from, None)).unwrap();
    db.upsert(&oficio_seq("sf-cascais", from, None)).unwrap();

    let mut db = open(&db_path);

    // Cada entidade emite ofícios independentes (timestamps distintos para ORDER BY determinístico)
    db.assign(
        &req("s-doc-1", "sf-setubal", "oficio_at", "u1", None, None),
        dt(2026, 3, 10),
        "ref-s1",
    )
    .unwrap();
    db.assign(
        &req("s-doc-2", "sf-setubal", "oficio_at", "u1", None, None),
        dt(2026, 3, 11),
        "ref-s2",
    )
    .unwrap();
    db.assign(
        &req("s-doc-3", "sf-setubal", "oficio_at", "u1", None, None),
        dt(2026, 3, 12),
        "ref-s3",
    )
    .unwrap();

    db.assign(
        &req("n-doc-1", "sf-sintra", "oficio_at", "u2", None, None),
        dt(2026, 3, 13),
        "ref-n1",
    )
    .unwrap();
    db.assign(
        &req("n-doc-2", "sf-sintra", "oficio_at", "u2", None, None),
        dt(2026, 3, 14),
        "ref-n2",
    )
    .unwrap();

    db.assign(
        &req("c-doc-1", "sf-cascais", "oficio_at", "u3", None, None),
        dt(2026, 3, 15),
        "ref-c1",
    )
    .unwrap();

    // Contadores são independentes por entidade
    let by_setubal = db
        .list_assignments(
            &AssignmentFilter {
                sequence_id: Some("sf-setubal-oficio".into()),
                ..Default::default()
            },
            50,
        )
        .unwrap();
    assert_eq!(by_setubal.len(), 3);
    assert_eq!(by_setubal[0].sequence_value, 3); // mais recente primeiro

    let by_sintra = db
        .list_assignments(
            &AssignmentFilter {
                sequence_id: Some("sf-sintra-oficio".into()),
                ..Default::default()
            },
            50,
        )
        .unwrap();
    assert_eq!(by_sintra.len(), 2);
    assert_eq!(by_sintra[0].sequence_value, 2);

    let by_cascais = db
        .list_assignments(
            &AssignmentFilter {
                sequence_id: Some("sf-cascais-oficio".into()),
                ..Default::default()
            },
            50,
        )
        .unwrap();
    assert_eq!(by_cascais.len(), 1);
    assert_eq!(by_cascais[0].sequence_value, 1);

    // Número formatado inclui o período correcto e sem mistura
    assert_eq!(by_setubal[2].number_value, "OF/2026/00001");
    assert_eq!(by_sintra[1].number_value, "OF/2026/00001");
    assert_eq!(by_cascais[0].number_value, "OF/2026/00001");
}

// ─── Cenário 3: Reset anual do contador ──────────────────────────────────────

#[test]
fn reset_anual_do_contador() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("reset_anual.db");

    let db = open(&db_path);
    db.upsert(&oficio_seq(
        "sf-setubal",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    ))
    .unwrap();

    let mut db = open(&db_path);

    // 2025: emitir 5 ofícios
    for i in 1..=5u64 {
        let r = db
            .assign(
                &req(
                    &format!("doc-2025-{i}"),
                    "sf-setubal",
                    "oficio_at",
                    "u1",
                    None,
                    None,
                ),
                dt(2025, 6, i as u32),
                &format!("ref-2025-{i}"),
            )
            .unwrap();
        assert_eq!(r.sequence_value, i, "2025: valor sequencial esperado {i}");
        assert_eq!(r.period_key, "2025");
        assert!(r.number_value.starts_with("OF/2025/"), "formato 2025");
    }

    // 2026: contador deve reiniciar em 1
    for i in 1..=3u64 {
        let r = db
            .assign(
                &req(
                    &format!("doc-2026-{i}"),
                    "sf-setubal",
                    "oficio_at",
                    "u1",
                    None,
                    None,
                ),
                dt(2026, 2, i as u32),
                &format!("ref-2026-{i}"),
            )
            .unwrap();
        assert_eq!(r.sequence_value, i, "2026: reinicia em {i}");
        assert_eq!(r.period_key, "2026");
        assert!(r.number_value.starts_with("OF/2026/"), "formato 2026");
    }

    // Contagens por period_key correctas
    let count_2025 = db
        .count_assignments(&AssignmentFilter {
            period_key: Some("2025".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(count_2025, 5);

    let count_2026 = db
        .count_assignments(&AssignmentFilter {
            period_key: Some("2026".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(count_2026, 3);
}

// ─── Cenário 4: Reset mensal do contador ─────────────────────────────────────

#[test]
fn reset_mensal_do_contador() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("reset_mensal.db");

    let certidao_seq = seq(
        "sf-setubal-certidao",
        "sf-setubal",
        "certidao",
        4,
        ResetPolicy::Monthly,
        vec![
            FormatPart::Literal("CERT/".into()),
            FormatPart::Period,
            FormatPart::Literal("/".into()),
            FormatPart::Sequence,
        ],
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    );

    let db = open(&db_path);
    db.upsert(&certidao_seq).unwrap();

    let mut db = open(&db_path);

    // Janeiro: 3 certidões
    let r = db
        .assign(
            &req("c-jan-1", "sf-setubal", "certidao", "u1", None, None),
            dt(2026, 1, 5),
            "rj1",
        )
        .unwrap();
    assert_eq!(r.period_key, "2026-01");
    assert_eq!(r.number_value, "CERT/2026-01/0001");

    db.assign(
        &req("c-jan-2", "sf-setubal", "certidao", "u1", None, None),
        dt(2026, 1, 15),
        "rj2",
    )
    .unwrap();
    let r3 = db
        .assign(
            &req("c-jan-3", "sf-setubal", "certidao", "u1", None, None),
            dt(2026, 1, 28),
            "rj3",
        )
        .unwrap();
    assert_eq!(r3.sequence_value, 3);
    assert_eq!(r3.number_value, "CERT/2026-01/0003");

    // Fevereiro: contador reinicia
    let r_feb = db
        .assign(
            &req("c-fev-1", "sf-setubal", "certidao", "u1", None, None),
            dt(2026, 2, 3),
            "rf1",
        )
        .unwrap();
    assert_eq!(r_feb.period_key, "2026-02");
    assert_eq!(r_feb.sequence_value, 1);
    assert_eq!(r_feb.number_value, "CERT/2026-02/0001");

    // Março: reinicia de novo
    let r_mar = db
        .assign(
            &req("c-mar-1", "sf-setubal", "certidao", "u1", None, None),
            dt(2026, 3, 10),
            "rm1",
        )
        .unwrap();
    assert_eq!(r_mar.period_key, "2026-03");
    assert_eq!(r_mar.sequence_value, 1);

    // Contagem por mês
    let jan = db
        .count_assignments(&AssignmentFilter {
            period_key: Some("2026-01".into()),
            ..Default::default()
        })
        .unwrap();
    let fev = db
        .count_assignments(&AssignmentFilter {
            period_key: Some("2026-02".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(jan, 3);
    assert_eq!(fev, 1);
}

// ─── Cenário 5: Validade temporal de sequências ───────────────────────────────

#[test]
fn validade_temporal_da_sequencia() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("validade.db");

    // Sequência antiga (2024-2025)
    let seq_antiga = oficio_seq(
        "sf-setubal",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
    );
    // Sequência nova (2026+) com formato diferente para distinção
    let seq_nova = seq(
        "sf-setubal-oficio-2026",
        "sf-setubal",
        "oficio_at",
        6,
        ResetPolicy::Yearly,
        vec![
            FormatPart::Literal("OF/".into()),
            FormatPart::Period,
            FormatPart::Literal("/".into()),
            FormatPart::Sequence,
        ],
        NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        None,
    );

    let db = open(&db_path);
    db.upsert(&seq_antiga).unwrap();
    db.upsert(&seq_nova).unwrap();

    // Verificar que a sequência certa é seleccionada por data
    let active_2025 = db
        .find_active_for(
            &NumberingKind::Document,
            "sf-setubal",
            Some("oficio_at"),
            None,
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        )
        .unwrap()
        .expect("deve encontrar sequência para 2025");
    assert_eq!(active_2025.sequence_id, "sf-setubal-oficio");
    assert_eq!(active_2025.padding, 5);

    let active_2026 = db
        .find_active_for(
            &NumberingKind::Document,
            "sf-setubal",
            Some("oficio_at"),
            None,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        )
        .unwrap()
        .expect("deve encontrar sequência para 2026");
    assert_eq!(active_2026.sequence_id, "sf-setubal-oficio-2026");
    assert_eq!(active_2026.padding, 6);

    // Antes de 2024: nenhuma sequência activa
    let antes = db
        .find_active_for(
            &NumberingKind::Document,
            "sf-setubal",
            Some("oficio_at"),
            None,
            NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
        )
        .unwrap();
    assert!(antes.is_none());

    // Atribuição usa automaticamente a sequência correcta
    let mut db = open(&db_path);
    let r_2025 = db
        .assign(
            &req("old-doc-1", "sf-setubal", "oficio_at", "u1", None, None),
            dt(2025, 11, 10),
            "rold",
        )
        .unwrap();
    let r_2026 = db
        .assign(
            &req("new-doc-1", "sf-setubal", "oficio_at", "u1", None, None),
            dt(2026, 2, 5),
            "rnew",
        )
        .unwrap();

    assert_eq!(r_2025.sequence_id, "sf-setubal-oficio");
    assert_eq!(r_2026.sequence_id, "sf-setubal-oficio-2026");
    // Padding diferente reflecte-se no número formatado
    assert_eq!(r_2025.number_value, "OF/2025/00001"); // padding 5
    assert_eq!(r_2026.number_value, "OF/2026/000001"); // padding 6
}

// ─── Cenário 6: Anulação de número ───────────────────────────────────────────

#[test]
fn anulacao_de_numero() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("anulacao.db");

    let db = open(&db_path);
    db.upsert(&oficio_seq(
        "sf-setubal",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    ))
    .unwrap();

    let mut db = open(&db_path);
    let now = dt(2026, 4, 10);

    let r1 = db
        .assign(
            &req(
                "doc-anular",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Erro de digitação"),
                None,
            ),
            now,
            "ref-anular",
        )
        .unwrap();
    assert_eq!(r1.number_value, "OF/2026/00001");
    assert!(matches!(
        r1.status,
        domain_numerador::AssignedStatus::Assigned
    ));

    // Anular o ofício
    let void_req = ChangeStatusRequest {
        kind: NumberingKind::Document,
        target: TargetRef {
            id: "doc-anular".into(),
            target_type: "document".into(),
        },
        actor: ActorRef {
            id: "chefe".into(),
            name: Some("Chefe de Serviço".into()),
        },
        reason: "Ofício emitido por engano".into(),
        correlation_id: None,
    };
    let voided = db
        .change_status(&void_req, AssignedStatus::Void, dt(2026, 4, 11))
        .unwrap();
    assert!(matches!(voided.status, AssignedStatus::Void));

    // Verificar que a anulação persiste após reabrir a DB
    let db = open(&db_path);
    let loaded = db
        .get_by_target(&NumberingKind::Document, "doc-anular")
        .unwrap()
        .unwrap();
    assert!(matches!(loaded.status, AssignedStatus::Void));
    assert_eq!(loaded.number_value, "OF/2026/00001");

    // Tentar anular novamente deve falhar (transição inválida)
    let mut db = open(&db_path);
    let err = db
        .change_status(&void_req, AssignedStatus::Void, dt(2026, 4, 12))
        .unwrap_err();
    assert!(matches!(
        err,
        NumeradorDomainError::InvalidStatusTransition(_)
    ));

    // Emitir novo ofício após anulação — contador continua (números anulados não são reutilizados)
    let r2 = db
        .assign(
            &req(
                "doc-seguinte",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                None,
                None,
            ),
            dt(2026, 4, 11),
            "ref-seg",
        )
        .unwrap();
    assert_eq!(r2.sequence_value, 2);
    assert_eq!(r2.number_value, "OF/2026/00002");
}

// ─── Cenário 7: Filtros e contagens estilo dashboard ─────────────────────────

#[test]
fn dashboard_contagens_e_filtros() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("dashboard.db");

    let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let db = open(&db_path);
    db.upsert(&oficio_seq("sf-setubal", from, None)).unwrap();
    db.upsert(&despacho_seq("sf-setubal", from)).unwrap();

    let mut db = open(&db_path);

    // Emitir ofícios em diferentes meses de 2026
    let jan = dt(2026, 1, 15);
    let mar = dt(2026, 3, 10);
    let jun = dt(2026, 6, 20);
    let out = dt(2026, 10, 5);

    db.assign(
        &req(
            "of-01",
            "sf-setubal",
            "oficio_at",
            "u1",
            Some("Janeiro 1"),
            None,
        ),
        jan,
        "ro1",
    )
    .unwrap();
    db.assign(
        &req(
            "of-02",
            "sf-setubal",
            "oficio_at",
            "u1",
            Some("Janeiro 2"),
            None,
        ),
        jan,
        "ro2",
    )
    .unwrap();
    db.assign(
        &req(
            "of-03",
            "sf-setubal",
            "oficio_at",
            "u2",
            Some("Março 1"),
            None,
        ),
        mar,
        "ro3",
    )
    .unwrap();
    db.assign(
        &req(
            "of-04",
            "sf-setubal",
            "oficio_at",
            "u2",
            Some("Junho 1"),
            None,
        ),
        jun,
        "ro4",
    )
    .unwrap();
    db.assign(
        &req(
            "of-05",
            "sf-setubal",
            "oficio_at",
            "u1",
            Some("Outubro 1"),
            None,
        ),
        out,
        "ro5",
    )
    .unwrap();
    db.assign(
        &req(
            "of-06",
            "sf-setubal",
            "oficio_at",
            "u1",
            Some("Outubro 2"),
            None,
        ),
        out,
        "ro6",
    )
    .unwrap();

    // Despachos
    db.assign(
        &req("desp-01", "sf-setubal", "despacho", "chefe", None, None),
        jan,
        "rd1",
    )
    .unwrap();
    db.assign(
        &req("desp-02", "sf-setubal", "despacho", "chefe", None, None),
        mar,
        "rd2",
    )
    .unwrap();
    db.assign(
        &req("desp-03", "sf-setubal", "despacho", "chefe", None, None),
        out,
        "rd3",
    )
    .unwrap();

    // Dashboard: totais por tipo (sequence_id)
    let total_oficios = db
        .count_assignments(&AssignmentFilter {
            sequence_id: Some("sf-setubal-oficio".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(total_oficios, 6);

    let total_despachos = db
        .count_assignments(&AssignmentFilter {
            sequence_id: Some("sf-setubal-despacho".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(total_despachos, 3);

    // Emitidos hoje (janela temporal)
    let inicio_outubro = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let fim_outubro = Utc.with_ymd_and_hms(2026, 10, 31, 23, 59, 59).unwrap();
    let outubro = db
        .count_assignments(&AssignmentFilter {
            assigned_after: Some(inicio_outubro),
            assigned_before: Some(fim_outubro),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(outubro, 3); // 2 ofícios + 1 despacho

    // Recentes: últimos 3
    let recentes = db
        .list_assignments(&AssignmentFilter::default(), 3)
        .unwrap();
    assert_eq!(recentes.len(), 3);
    // Mais recentes primeiro — os 3 de Outubro
    assert!(recentes.iter().all(|a| a.period_key == "2026"));

    // Por actor
    let by_u1 = db
        .list_assignments(&AssignmentFilter::default(), 100)
        .unwrap()
        .into_iter()
        .filter(|a| a.assigned_by.id == "u1")
        .count();
    assert_eq!(by_u1, 4); // of-01, of-02, of-05, of-06
}

// ─── Cenário 8: Simulação — dois anos de actividade municipal ────────────────

#[test]
fn simulacao_municipio_dois_anos() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("municipio.db");

    // --- Configuração inicial ---
    let from_2024 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let from_2026 = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let db = open(&db_path);

    // SF Setúbal: ofícios e despachos
    db.upsert(&oficio_seq("sf-setubal", from_2024, None))
        .unwrap();
    db.upsert(&despacho_seq("sf-setubal", from_2024)).unwrap();

    // SF Sintra: apenas ofícios (começa em 2026)
    db.upsert(&oficio_seq("sf-sintra", from_2026, None))
        .unwrap();

    // Certidões (reset mensal) para SF Setúbal
    db.upsert(&seq(
        "sf-setubal-certidao",
        "sf-setubal",
        "certidao",
        4,
        ResetPolicy::Monthly,
        vec![
            FormatPart::Literal("CERT/".into()),
            FormatPart::Period,
            FormatPart::Literal("/".into()),
            FormatPart::Sequence,
        ],
        from_2024,
        None,
    ))
    .unwrap();

    let mut db = open(&db_path);

    // --- 2025: Actividade do SF Setúbal ---

    // Q1 2025
    let of_2025_01 = db
        .assign(
            &req(
                "of-2025-001",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Pedido de esclarecimento"),
                Some("ACSS"),
            ),
            dt(2025, 1, 8),
            "rof-2025-001",
        )
        .unwrap();
    assert_eq!(of_2025_01.number_value, "OF/2025/00001");

    let of_2025_02 = db
        .assign(
            &req(
                "of-2025-002",
                "sf-setubal",
                "oficio_at",
                "mmatos",
                Some("Resposta a ofício"),
                Some("DGO"),
            ),
            dt(2025, 2, 14),
            "rof-2025-002",
        )
        .unwrap();
    assert_eq!(of_2025_02.number_value, "OF/2025/00002");

    let dp_2025_01 = db
        .assign(
            &req("dp-2025-001", "sf-setubal", "despacho", "chefe", None, None),
            dt(2025, 2, 20),
            "rdp-2025-001",
        )
        .unwrap();
    assert_eq!(dp_2025_01.number_value, "DESP/2025/0001");

    // Certidões Jan/Fev 2025
    let c_jan1 = db
        .assign(
            &req(
                "cert-2025-jan-1",
                "sf-setubal",
                "certidao",
                "balcao",
                Some("Certidão de residência"),
                None,
            ),
            dt(2025, 1, 10),
            "rc-jan1",
        )
        .unwrap();
    assert_eq!(c_jan1.number_value, "CERT/2025-01/0001");

    let c_jan2 = db
        .assign(
            &req(
                "cert-2025-jan-2",
                "sf-setubal",
                "certidao",
                "balcao",
                Some("Certidão predial"),
                None,
            ),
            dt(2025, 1, 22),
            "rc-jan2",
        )
        .unwrap();
    assert_eq!(c_jan2.number_value, "CERT/2025-01/0002");

    let c_fev1 = db
        .assign(
            &req(
                "cert-2025-fev-1",
                "sf-setubal",
                "certidao",
                "balcao",
                None,
                None,
            ),
            dt(2025, 2, 5),
            "rc-fev1",
        )
        .unwrap();
    assert_eq!(c_fev1.number_value, "CERT/2025-02/0001"); // reset mensal

    // Q4 2025 — último ofício do ano
    let of_2025_10 = db
        .assign(
            &req(
                "of-2025-010",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                Some("Relatório anual"),
                Some("Câmara Municipal"),
            ),
            dt(2025, 12, 18),
            "rof-2025-010",
        )
        .unwrap();
    assert_eq!(of_2025_10.sequence_value, 3);
    assert_eq!(of_2025_10.number_value, "OF/2025/00003");

    // --- 2026: Novo ano — todos os contadores anuais reiniciam ---

    let of_2026_01 = db
        .assign(
            &req(
                "of-2026-001",
                "sf-setubal",
                "oficio_at",
                "mmatos",
                Some("Abertura de ano"),
                Some("DGAL"),
            ),
            dt(2026, 1, 5),
            "rof-2026-001",
        )
        .unwrap();
    assert_eq!(of_2026_01.number_value, "OF/2026/00001"); // reiniciou!
    assert_eq!(of_2026_01.period_key, "2026");

    let dp_2026_01 = db
        .assign(
            &req("dp-2026-001", "sf-setubal", "despacho", "chefe", None, None),
            dt(2026, 1, 10),
            "rdp-2026-001",
        )
        .unwrap();
    assert_eq!(dp_2026_01.number_value, "DESP/2026/0001"); // reiniciou!

    let of_2026_02 = db
        .assign(
            &req(
                "of-2026-002",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                None,
                Some("ATA"),
            ),
            dt(2026, 3, 12),
            "rof-2026-002",
        )
        .unwrap();
    assert_eq!(of_2026_02.number_value, "OF/2026/00002");

    // SF Sintra começa em 2026
    let of_sintra_01 = db
        .assign(
            &req(
                "sintra-001",
                "sf-sintra",
                "oficio_at",
                "afernandes",
                Some("Pedido inicial"),
                None,
            ),
            dt(2026, 2, 1),
            "rsintra-001",
        )
        .unwrap();
    assert_eq!(of_sintra_01.number_value, "OF/2026/00001");
    assert_eq!(of_sintra_01.sequence_id, "sf-sintra-oficio");

    // Ofício de Setúbal e Sintra têm OF/2026/00001 — sem conflito
    let of_set = db
        .get_by_target(&NumberingKind::Document, "of-2026-001")
        .unwrap()
        .unwrap();
    let of_sin = db
        .get_by_target(&NumberingKind::Document, "sintra-001")
        .unwrap()
        .unwrap();
    assert_eq!(of_set.number_value, "OF/2026/00001");
    assert_eq!(of_sin.number_value, "OF/2026/00001");
    assert_ne!(of_set.sequence_id, of_sin.sequence_id); // sequências diferentes

    // Anular um ofício de 2026 (erro de emissão)
    let void_req = ChangeStatusRequest {
        kind: NumberingKind::Document,
        target: TargetRef {
            id: "of-2026-002".into(),
            target_type: "document".into(),
        },
        actor: ActorRef {
            id: "chefe".into(),
            name: None,
        },
        reason: "Emitido para entidade errada".into(),
        correlation_id: None,
    };
    db.change_status(&void_req, AssignedStatus::Void, dt(2026, 3, 13))
        .unwrap();

    // Novo ofício depois da anulação — contador não recua
    let of_2026_03 = db
        .assign(
            &req(
                "of-2026-003",
                "sf-setubal",
                "oficio_at",
                "ccosta",
                None,
                Some("ATA"),
            ),
            dt(2026, 3, 13),
            "rof-2026-003",
        )
        .unwrap();
    assert_eq!(of_2026_03.sequence_value, 3);
    assert_eq!(of_2026_03.number_value, "OF/2026/00003");

    // --- Verificações de integridade final ---

    // Totais por entidade/tipo/ano
    let total_of_setubal_2025 = db
        .count_assignments(&AssignmentFilter {
            sequence_id: Some("sf-setubal-oficio".into()),
            period_key: Some("2025".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(total_of_setubal_2025, 3);

    let total_of_setubal_2026 = db
        .count_assignments(&AssignmentFilter {
            sequence_id: Some("sf-setubal-oficio".into()),
            period_key: Some("2026".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(total_of_setubal_2026, 3); // inclui o anulado

    // Apenas activos de 2026
    let activos_2026 = db
        .list_assignments(
            &AssignmentFilter {
                sequence_id: Some("sf-setubal-oficio".into()),
                period_key: Some("2026".into()),
                status: Some(AssignedStatus::Assigned),
                ..Default::default()
            },
            50,
        )
        .unwrap();
    assert_eq!(activos_2026.len(), 2); // of-2026-001 e of-2026-003

    // Uniqueness global: nenhum sequence_value repetido por (sequence_id, period_key)
    let all = db
        .list_assignments(&AssignmentFilter::default(), 1000)
        .unwrap();
    let mut seen: std::collections::HashSet<(&str, &str, u64)> = std::collections::HashSet::new();
    for a in &all {
        let key = (
            a.sequence_id.as_str(),
            a.period_key.as_str(),
            a.sequence_value,
        );
        assert!(seen.insert(key), "sequence_value duplicado: {:?}", key);
    }

    // Persistência: reabrir e verificar totais globais
    drop(db);
    let db = open(&db_path);
    let grand_total = db.count_assignments(&AssignmentFilter::default()).unwrap();
    assert_eq!(
        grand_total,
        all.len() as u64,
        "totais persistem após reabrir DB"
    );
}
