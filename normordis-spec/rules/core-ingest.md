# Regras de Negócio — core-ingest

`core-ingest` define ingestão auditável de bundles exportados no fluxo
`validate -> hash -> scan -> route -> audit`.

---

## IngestSource

### INGEST-R01 — Campos obrigatórios não vazios

`kind`, `subject_id` e `version` são obrigatórios. Nenhum pode ser apenas whitespace.

### INGEST-R02 — kind identifica tipo interoperável

`kind` deve identificar o tipo interoperável do bundle. Valores desconhecidos
são rejeitados pelo router de ingestão.

### INGEST-R03 — subject_id corresponde ao bundle

`subject_id` deve corresponder ao sujeito declarado no bundle. Divergências
indicam corrupção ou manipulação do bundle.

### INGEST-R04 — version corresponde ao bundle

`version` deve corresponder à versão declarada no bundle. Divergências
indicam que o bundle não é a versão esperada.

---

## IngestEvidence

### INGEST-R05 — Toda decisão produz evidência

Toda decisão de ingestão (aceite ou rejeitada) deve produzir evidência.
Decisões sem evidência não são válidas nem auditáveis.

### INGEST-R06 — decision canónica

`decision` deve ser `accepted` ou `rejected`.

### INGEST-R07 — hash.algorithm declarado

`hash.algorithm` deve indicar o algoritmo usado para calcular o hash do bundle.

### INGEST-R08 — audit.emitted via caminho canónico

`audit.emitted = true` apenas quando o evento de audit foi construído pelo
caminho canónico do core-audit. Eventos emitidos por outro caminho invalidam
a cadeia de evidências.
