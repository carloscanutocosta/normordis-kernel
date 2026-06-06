# Fixtures de Migração

Esta pasta contém fixtures que documentam comportamento **anterior a um breaking change MAJOR**.

---

## Propósito

Quando um schema é alterado de forma breaking (ex: um campo muda de formato, um enum
perde um valor, uma fixture válida passa a inválida), o comportamento antigo deve ser
preservado aqui **antes de ser invalidado**, de forma a:

1. Documentar o que era válido na versão anterior.
2. Servir de base para guias de migração.
3. Permitir testar transformadores de dados históricos.

---

## Formato de nome

```
{schema}-pre-v{MAJOR}-{descrição}.json
```

Exemplos:
- `audit-event-pre-v2-control-id.json` — AuditEvent com o formato antigo de control_id
- `org-unit-pre-v2-no-status.json` — OrgUnit sem o campo status (antes de se tornar obrigatório)

---

## Quando usar

Seguir o protocolo de breaking change descrito em `GOVERNANCE.md`:

1. Antes de alterar o schema, criar aqui a fixture com o comportamento antigo.
2. Verificar que `cargo test -p spec-conformance` ainda passa com a fixture de migração
   (ela ainda é válida neste momento, antes da alteração).
3. Alterar o schema e as fixtures `valid/`/`invalid/` conforme o protocolo.
4. Após a alteração, a fixture de migração torna-se inválida pelo novo schema —
   esse comportamento é intencional e documentado.

---

## Estado actual

Nenhum breaking change MAJOR foi ainda introduzido (spec em `0.x`).
Esta pasta está vazia até ao primeiro incremento MAJOR.
