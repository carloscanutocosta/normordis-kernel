# MAN.md

## Nome

support-miniapp-runtime

## Tipo

Biblioteca headless (Rust puro), agnóstica de UI/UX

## Objetivo

Fornecer um ponto mínimo de integração entre bibliotecas transversais para fluxos base de mini-apps locais.

## Âmbito

- contexto mínimo de app
- criação de instâncias documentais
- ligação mínima entre utilizador, orgânica, auditoria e numeração

## Fora de âmbito

- UI
- persistência concreta
- workflow complexo
- regras específicas de negócio

## Princípio de neutralidade de UI/UX

Esta biblioteca é agnóstica de UI/UX e não depende de qualquer framework visual.

## Contrato público

### Tipos públicos

- `MiniAppContext`
- `CreateDocumentRequest`

### Funções públicas

- `create_document_instance(...)`
- `create_document_created_event(...)`
- `allocate_document_number(...)`
- `record_document_created(...)`, gravando em um `core_audit::AuditStore`
- `author_from_context(...)`

### Erros públicos

- `RuntimeError`

## Invariantes

- a integração não pode introduzir dependência de UI
- o contexto da app deve ser válido antes de qualquer operação
- eventos de auditoria usam o contrato institucional `core-audit`, não o contrato legado `support-audit`
- a biblioteca não substitui regras de domínio da mini-app

## Compatibilidade e versionamento

Segue SemVer. Alterações breaking exigem revisão explícita deste `MAN.md`.

## Estado

Proposto

## Última revisão

2026-05-12
