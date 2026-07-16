# Regras de Negócio — core-org

Estas regras definem o contrato institucional da estrutura orgânica. Os schemas
validam a forma interoperável; estas regras capturam invariantes temporais,
hierárquicas e jurídicas.

---

## OrgUnit

### ORG-R01 — level >= 1

`level` deve ser maior ou igual a 1.

### ORG-R02 — Unidade raiz não tem pai

Unidade de nível 1 não pode ter `parent_id`.

### ORG-R03 — Unidade não-raiz tem pai

Unidade de nível superior a 1 deve ter `parent_id`.

### ORG-R04 — Nomes obrigatórios

`short_name` e `full_name` são obrigatórios e não podem ser apenas whitespace.

### ORG-R05 — valid_until posterior a valid_from

`valid_until`, se presente, deve ser estritamente posterior a `valid_from`.
Implementações devem rejeitar ranges invertidos mesmo que o schema aceite
(invariante de camada 3).

### ORG-R06 — Unidades extinct não transitam de estado

Uma unidade com `status = extinct` não pode mudar para qualquer outro estado.
Esta regra é verificada pela máquina de estados do serviço, não por fixture
individual — requer contexto de transição entre dois estados.

### ORG-R07 — Contactos com formato mínimo

Contactos opcionais, quando presentes, devem preservar formato estrutural mínimo:
`email` deve ser RFC 5321 válido; `phone`/`fax` devem ter pelo menos 7 dígitos.

---

## OrgPosition

### ORG-R08 — Cargo Extinct não pode ser referenciado em nova Delegation

Um cargo com `status = extinct` não pode ser `from_position` nem `to_position`
em novas delegações. Regra verificada pelo serviço; requer contexto de dois registos.

### ORG-R09 — PositionKind::Outro deve ser não vazio

`PositionKind::Outro(String)` deve ter a string não vazia após trim.

---

## Delegation

### ORG-R10 — valid_until posterior a valid_from

`valid_until`, se presente, deve ser estritamente posterior a `valid_from`.

### ORG-R11 — Delegação não pode ser auto-delegação

`from_position` e `to_position` devem ser distintos.
Implementações devem rejeitar auto-delegações mesmo que o schema não o expresse
directamente (invariante de camada 3).

---

## LegalInstrument

### ORG-R12 — effective_until posterior a effective_from

`effective_until`, se presente, deve ser estritamente posterior a `effective_from`.

### ORG-R13 — reference e description obrigatórios

`reference` e `description` são obrigatórios e não podem ser apenas whitespace.
