# Manual: address-sqlite

## Contrato publico

- `POSTAL_CODE_TABLE`
- `AddressSqliteError`
- `SqliteAddressStore`
- `SqliteAddressStore::open`
- `SqliteAddressStore::from_connection`
- `SqliteAddressStore::connection`
- `SqliteAddressStore::lookup_postal_code`
- `SqliteAddressStore::lookup_postal_code_parts`

## Invariantes

- O adapter usa `adapter-sqlite` para abertura de base SQLite.
- A tabela esperada chama-se `platform_reference_postal_code`.
- O lookup pode devolver zero, um ou varios candidatos.
- A escolha do candidato nao pertence a este adapter.

## Limitacoes atuais

- A consulta ainda materializa rows atraves de `rusqlite::Connection` exposta pela ponte de compatibilidade de `adapter-sqlite`.
- Nao gere migrations da tabela de referencia postal.
- Nao valida existencia funcional da base de plataforma.

## ToDo

- Avaliar migração para API relacional controlada de `adapter-sqlite` quando houver contrato adequado para queries read-only especializadas.
