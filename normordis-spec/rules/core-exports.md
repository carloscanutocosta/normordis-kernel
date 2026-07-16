# Regras de Negócio — core-exports

`core-exports` define snapshots exportáveis e recibos atómicos de exportação.

---

## SourceRef

### EXP-R01 — Campos obrigatórios não vazios

`kind`, `subject_id` e `version` são obrigatórios. Nenhum pode ser apenas whitespace.

### EXP-R02 — kind identifica tipo de fonte

`kind` identifica o tipo de fonte exportada (ex: `core-org`, `core-rh`).

### EXP-R03 — subject_id identifica sujeito

`subject_id` identifica o sujeito dentro do tipo de fonte.

### EXP-R04 — version identifica versão exportada

`version` identifica a versão exportada.

---

## ExportSnapshot

### EXP-R05 — snapshot_id com formato canónico

`snapshot_id` deve seguir o formato `exp:{kind}:{subject_id}:{version}:{hash16}`.

### EXP-R06 — manifest.algorithm deve ser SHA-256

`manifest.algorithm` deve ser `SHA-256`.

### EXP-R07 — manifest.hash com prefixo sha256

`manifest.hash` deve ser `sha256:` seguido de 64 caracteres hexadecimais lowercase.

### EXP-R08 — item_count consistente

`manifest.item_count` deve coincidir com o número de artefactos do pacote.

### EXP-R09 — Hash determinístico

O hash do manifesto deve ser determinístico para o mesmo conteúdo.
Implementações não podem introduzir aleatoriedade no cálculo.

---

## ExportReceipt

### EXP-R10 — actor e correlation_id obrigatórios

`actor` e `correlation_id` são obrigatórios.

### EXP-R11 — Atomicidade snapshot + audit

Snapshot e audit event são produzidos atomicamente. A falha de um invalida o outro.
