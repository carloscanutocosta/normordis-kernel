# Regras de Negócio — core-validation

`core-validation` valida estrutura, integridade e coerência formal. Não decide
mérito administrativo, autorização, interpretação legal ou permissões.

---

## ValidationIssue

### VAL-R01 — rule_id obrigatório

`rule_id` é obrigatório e identifica uma regra canónica (ex: `ORG-R01`).

### VAL-R02 — severity canónica

`severity` deve ser `info`, `warning` ou `error`.

### VAL-R03 — message obrigatório e técnico

`message` é obrigatório e deve conter contexto técnico. Não deve transportar
dados pessoais (ver RGPD art.º 5.1.c).

### VAL-R04 — field identifica campo afectado

`field` é opcional e, quando presente, identifica o campo específico afectado
pela issue.

---

## ValidationResult

### VAL-R05 — Campos de identificação obrigatórios

`validation_id`, `target_type` e `target_id` são obrigatórios.

### VAL-R06 — overall_status reflecte estado global

`overall_status` deve reflectir fielmente o estado global da validação:
`passed` só quando todos os issues são `info` ou `warning` sem issues de `error`.

### VAL-R07 — failed bloqueia progressão

`overall_status = failed` bloqueia progressão. `execution_error` indica falha
de infra e não deve ser tratado como regra falhada.

### VAL-R08 — overridden requer justificação

`overall_status = overridden` requer justificação auditável no `message`
do outcome. Overrides sem justificação são inválidos.

---

## Integridade

### VAL-R09 — Hash SHA-256 canónico

Hash SHA-256 canónico é lowercase hexadecimal com exactamente 64 caracteres.

### VAL-R10 — Manifests ordenam por path antes do hash

Manifests devem ordenar entradas por path canónico antes de calcular `list_hash`.
O resultado do hash é determinístico para o mesmo conjunto de artefactos.
