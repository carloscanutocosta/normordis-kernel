pub const METRICS_SQLITE_MIGRATIONS: &[&str] = &[
    // migration 1 — schema base: eventos operacionais e governação
    r#"
    -- Eventos métricos operacionais emitidos pelas apps.
    CREATE TABLE IF NOT EXISTS metric_events (
        id                  TEXT PRIMARY KEY,
        metric_code         TEXT NOT NULL,
        metric_version_id   TEXT,
        evaluation_cycle_id TEXT,
        value               REAL NOT NULL,
        unit                TEXT,
        correlation_id      TEXT,
        entity_type         TEXT,
        entity_id           TEXT,
        state               TEXT,
        org_unit_id         TEXT,
        source_app          TEXT,
        version             TEXT,
        valid_at            TEXT,
        labels_json         TEXT,
        payload_json        TEXT,
        timestamp           TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_metric_events_code
        ON metric_events (metric_code);
    CREATE INDEX IF NOT EXISTS idx_metric_events_timestamp
        ON metric_events (timestamp DESC);
    CREATE INDEX IF NOT EXISTS idx_metric_events_cycle
        ON metric_events (evaluation_cycle_id)
        WHERE evaluation_cycle_id IS NOT NULL;
    CREATE INDEX IF NOT EXISTS idx_metric_events_org_unit
        ON metric_events (org_unit_id)
        WHERE org_unit_id IS NOT NULL;
    CREATE INDEX IF NOT EXISTS idx_metric_events_entity
        ON metric_events (entity_type, entity_id)
        WHERE entity_type IS NOT NULL AND entity_id IS NOT NULL;

    -- Definições métricas governadas pelo órgão de gestão.
    CREATE TABLE IF NOT EXISTS metric_definitions (
        id                  TEXT PRIMARY KEY,
        code                TEXT NOT NULL UNIQUE,
        name                TEXT NOT NULL,
        description         TEXT NOT NULL,
        purpose             TEXT NOT NULL,
        owner_org_unit_id   TEXT NOT NULL,
        governance_owner    TEXT NOT NULL,
        status              TEXT NOT NULL DEFAULT 'draft',
        created_at          TEXT NOT NULL,
        created_by          TEXT NOT NULL,
        updated_at          TEXT,
        updated_by          TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_metric_definitions_code
        ON metric_definitions (code);
    CREATE INDEX IF NOT EXISTS idx_metric_definitions_status
        ON metric_definitions (status);

    -- Versões de definições métricas (fórmula, vigência, evidências).
    CREATE TABLE IF NOT EXISTS metric_versions (
        id                      TEXT PRIMARY KEY,
        metric_definition_id    TEXT NOT NULL REFERENCES metric_definitions(id),
        version                 TEXT NOT NULL,
        status                  TEXT NOT NULL DEFAULT 'draft',
        valid_from              TEXT NOT NULL,
        valid_to                TEXT,
        formula_ref             TEXT NOT NULL,
        calculation_binding_json TEXT,
        evidence_requirements_json TEXT NOT NULL DEFAULT '[]',
        approval_ref            TEXT,
        published_at            TEXT,
        created_at              TEXT NOT NULL,
        created_by              TEXT NOT NULL,
        UNIQUE (metric_definition_id, version)
    );

    CREATE INDEX IF NOT EXISTS idx_metric_versions_definition
        ON metric_versions (metric_definition_id, status);

    -- Targets (objectivos/limiares) por versão de métrica e âmbito.
    CREATE TABLE IF NOT EXISTS target_definitions (
        id                  TEXT PRIMARY KEY,
        metric_version_id   TEXT NOT NULL REFERENCES metric_versions(id),
        scope_type          TEXT NOT NULL,
        scope_id            TEXT NOT NULL,
        target_value        REAL NOT NULL,
        unit                TEXT NOT NULL,
        thresholds_json     TEXT NOT NULL DEFAULT '[]',
        valid_from          TEXT NOT NULL,
        valid_to            TEXT,
        created_at          TEXT NOT NULL,
        created_by          TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_targets_version
        ON target_definitions (metric_version_id);
    CREATE INDEX IF NOT EXISTS idx_targets_scope
        ON target_definitions (metric_version_id, scope_id);

    -- Ciclos formais de avaliação (SIADAP, BSC, etc.).
    CREATE TABLE IF NOT EXISTS evaluation_cycles (
        id                  TEXT PRIMARY KEY,
        code                TEXT NOT NULL UNIQUE,
        name                TEXT NOT NULL,
        cycle_type          TEXT NOT NULL,
        period_start        TEXT NOT NULL,
        period_end          TEXT NOT NULL,
        governance_context  TEXT,
        status              TEXT NOT NULL DEFAULT 'planned',
        created_at          TEXT NOT NULL,
        created_by          TEXT NOT NULL
    );

    -- Instâncias de indicador por ciclo, unidade orgânica e responsável.
    CREATE TABLE IF NOT EXISTS indicator_instances (
        id                      TEXT PRIMARY KEY,
        metric_version_id       TEXT NOT NULL REFERENCES metric_versions(id),
        evaluation_cycle_id     TEXT NOT NULL REFERENCES evaluation_cycles(id),
        org_unit_id             TEXT NOT NULL,
        responsible_actor_id    TEXT NOT NULL,
        scope                   TEXT,
        status                  TEXT NOT NULL DEFAULT 'pending',
        created_at              TEXT NOT NULL,
        created_by              TEXT NOT NULL,
        closed_at               TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_instances_cycle
        ON indicator_instances (evaluation_cycle_id);
    CREATE INDEX IF NOT EXISTS idx_instances_cycle_org
        ON indicator_instances (evaluation_cycle_id, org_unit_id);

    -- Resultados de medição calculados para instâncias de indicador.
    CREATE TABLE IF NOT EXISTS measurement_results (
        id                          TEXT PRIMARY KEY,
        indicator_instance_id       TEXT NOT NULL REFERENCES indicator_instances(id),
        metric_version_id           TEXT NOT NULL REFERENCES metric_versions(id),
        value                       REAL NOT NULL,
        unit                        TEXT NOT NULL,
        status                      TEXT NOT NULL DEFAULT 'calculated',
        calculated_at               TEXT NOT NULL,
        calculated_by               TEXT NOT NULL,
        calculation_snapshot_hash   TEXT,
        quality_flags_json          TEXT NOT NULL DEFAULT '[]',
        valid_at                    TEXT,
        rectifies_result_id         TEXT REFERENCES measurement_results(id)
    );

    CREATE INDEX IF NOT EXISTS idx_results_instance
        ON measurement_results (indicator_instance_id, status);

    -- Ligações de evidência entre resultados e fontes de dados.
    CREATE TABLE IF NOT EXISTS evidence_links (
        id                      TEXT PRIMARY KEY,
        measurement_result_id   TEXT NOT NULL REFERENCES measurement_results(id),
        evidence_type           TEXT NOT NULL,
        core_ref                TEXT NOT NULL,
        resource_id             TEXT NOT NULL,
        correlation_id          TEXT,
        hash                    TEXT,
        valid_at                TEXT,
        linked_at               TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_evidence_result
        ON evidence_links (measurement_result_id);
    "#,
    // migration 2 — governance audit log
    r#"
    CREATE TABLE IF NOT EXISTS metric_governance_log (
        id              TEXT PRIMARY KEY,
        entity_type     TEXT NOT NULL,
        entity_id       TEXT NOT NULL,
        action          TEXT NOT NULL,
        from_value      TEXT,
        to_value        TEXT NOT NULL,
        changed_by      TEXT NOT NULL,
        changed_at      TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_governance_log_entity
        ON metric_governance_log (entity_type, entity_id);
    CREATE INDEX IF NOT EXISTS idx_governance_log_changed_at
        ON metric_governance_log (changed_at DESC);
    "#,
    // migration 3 — payload_json column on measurement_results
    r#"
    ALTER TABLE measurement_results ADD COLUMN payload_json TEXT;
    "#,
];
