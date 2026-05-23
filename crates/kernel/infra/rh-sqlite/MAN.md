# rh-sqlite

## Contrato publico

- `UsersSqliteStore`
- `UsersSqliteError`
- `RH_SQLITE_MIGRATIONS`

## Regras

- Usa o schema historico `local_user` e `current_user_context` para preservar compatibilidade de dados.
- Persiste `core_rh::UserIdentity`.
- Resolve `core_rh::UserContext`.
- Mantem helpers transacionais usados por runtimes e apps.

## Invariantes

- O modelo institucional fica em `core-rh`.
- SQLite fica apenas neste adapter de infra.
- Nao ha dependencias de UI, Tauri, rede ou autenticacao externa.

## Limitacoes atuais

- Nao gere passwords, tokens ou segredo.
- Nao implementa autorizacao complexa.
- Nao faz migracao de schema para multiplos papeis funcionais; `Role` continua a ser atributo principal registavel.

## ToDo

- Avaliar migracao futura para tabela separada de papeis quando houver RBAC real.
- Remover queries diretas de apps sobre `local_user` quando houver ports mais completos de consulta.
