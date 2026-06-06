# Regras de Negócio — core-audit

Estas regras transcendem o JSON Schema. Um evento pode ser válido segundo o schema
mas inválido segundo estas regras. Qualquer implementação conforme deve aplicar ambas.

---

## AuditEvent

### AUDIT-R01 — actor_id não pode ser vazio após trim

`actor.actor_id` deve ter pelo menos um caractere não-branco após `.trim()`.
Um `actor_id` com apenas espaços é inválido mesmo que passe `minLength: 1`.

### AUDIT-R02 — occurred_at_utc deve estar em UTC

O campo `occurred_at_utc` deve terminar em `Z` ou `+00:00`.
Timestamps com offset positivo ou negativo são rejeitados.

### AUDIT-R03 — event_id deve ser UUID v4

O `event_id` deve ser um UUID v4 válido (RFC 4122). Implementações não devem
aceitar identificadores arbitrários que não sigam o formato UUID v4.

### AUDIT-R04 — control_id, se presente, deve ser não vazio após trim

Quando `control_id` é incluído no JSON, não pode ser uma string vazia nem só espaços.

### AUDIT-R05 — event_type deve seguir o padrão domínio.acção

Formato obrigatório: `{domínio}.{acção}` em minúsculas, sem espaços.
Exemplos válidos: `user.login`, `document.sign`, `org_unit.create`.
Exemplos inválidos: `UserLogin`, `document sign`, ``.

---

## AuditChain

### CHAIN-R01 — sequence é monotonicamente crescente

Cada novo elo (`AuditChainLink`) deve ter `sequence` estritamente maior que o elo anterior.
Não é permitido repetir nem decrementar a sequência.

### CHAIN-R02 — encadeamento de hashes

`record_hash` de cada elo é calculado sobre o conteúdo do evento + `previous_record_hash`
do elo anterior (ou `null` para o primeiro evento). Qualquer elo com hash inconsistente
invalida toda a cadeia a partir desse ponto.

### CHAIN-R03 — first link has no previous

O primeiro elo da cadeia (`sequence = 1`) deve ter `previous_record_hash = null`.
Qualquer outro valor é inválido.

---

## ControlDefinition

### CTRL-R01 — control_id segue o padrão CTRL-CATEGORIA-NNN

Formato: `CTRL-` + categoria canónica em maiúsculas + hífen + número com 3 dígitos.
Exemplos: `CTRL-AUTH-001`, `CTRL-INT-003`, `CTRL-TRACE-012`.

### CTRL-R02 — valid_to posterior a valid_from

Quando `valid_to` está presente, deve ser estritamente posterior a `valid_from`.

### CTRL-R03 — controlos inactivos não devem ser executados

Um `ControlExecution` não deve referenciar um `ControlDefinition` com `active = false`.
Excepção: re-execução de auditoria histórica com `result = dispensed`.

---

## ControlExecution

### EXEC-R01 — event_id deve referenciar evento existente

`event_id` numa `ControlExecution` deve corresponder a um `AuditEvent.event_id`
já registado. Execuções órfãs (sem evento correspondente) são inválidas.

### EXEC-R02 — evidence_ref, se presente, deve ser URI ou chave válida

`evidence_ref` deve identificar um recurso recuperável: URI relativa ou UUID
de um documento no arquivo. Strings arbitrárias são aceites pela spec mas
implementações devem validar o formato internamente.
