# Regras de Negócio — core-security

`core-security` define autorização institucional reutilizável. Não autentica
identidades, não persiste políticas e não depende de IAM, SQLite, UI ou runtime.

---

## Policy

### SEC-R01 — policy_id e version obrigatórios

`policy_id` e `version` são obrigatórios e não podem ser apenas whitespace.

### SEC-R02 — mode canónico

`mode` deve ser `baseline` ou `strict`.

### SEC-R03 — rules não vazio

`rules` deve conter pelo menos uma regra. Uma política sem regras não pode
ser aplicada e é rejeitada.

### SEC-R04 — Rule.code obrigatório

Cada `Rule.code` é obrigatório e não pode ser apenas whitespace.

### SEC-R05 — valid_to posterior a valid_from

`valid_to`, se presente, deve ser estritamente posterior a `valid_from`.

---

## Autorização

### SEC-R06 — Produção é deny-by-default

Produção é deny-by-default sem políticas activas. Ausência de política não
equivale a permissão — é negação implícita.

### SEC-R07 — strict exige delegação explícita

`strict` exige delegação explícita para operações governadas. O modo `baseline`
permite operações sem delegação desde que a política o autorize.

### SEC-R08 — Delegações expiradas não autorizam

Delegações expiradas ou revogadas não autorizam operações. A verificação de
expiração é responsabilidade do chamador, não da política.

### SEC-R09 — Fail-closed em produção

Falhas de auditoria, eventos ou histórico SoD são fail-closed em produção.
Erros de infra não devem ser tratados como permissão implícita.
