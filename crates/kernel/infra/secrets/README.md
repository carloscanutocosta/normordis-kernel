# infra-secrets

Crate headless de infraestrutura para protecao de segredos do Mini-Kernel RS.

## Objetivo

Fornecer um adapter pequeno para proteger material sensivel usado por
componentes como `support-storage` e `support-crypto`, sem acoplar criptografia
a UI, Tauri ou persistencia concreta.

## Responsabilidade

- Proteger/desproteger segredos atraves de `SecretProtector`.
- Fornecer `ProtectedSecret` serializavel para persistencia por outra camada.
- Expor `ProtectedKeyProvider` compatível com `support_crypto::KeyProvider` e
  `support_crypto::KeyResolver`.
- Gerar chaves aleatorias de 32 bytes para `support-crypto`.
- Usar Windows DPAPI em Windows.
- Usar protecao portavel por passphrase/recovery secret quando a chave tiver de
  funcionar noutras maquinas.

## Nao responsabilidade

- Nao decide onde persistir o `ProtectedSecret`.
- Nao implementa UI de unlock.
- Nao cifra dados de negocio.
- Nao substitui `support-crypto`.
- Nao depende de Tauri.

## Estado

Base operacional para runtime/bootstrap ou futuro adapter de keyring.

O modelo principal recomendado para maquinas sem instalacao adicional e:

```text
portable-passphrase-v1
```

DPAPI fica disponivel como cache/protecao local opcional.

## Modos de protecao

### Windows DPAPI

Um `ProtectedSecret` criado com Windows DPAPI nao deve ser assumido como
portavel para outro computador. Em `CurrentUser`, fica associado ao utilizador e
perfil Windows; em `LocalMachine`, fica associado a maquina. Migracao entre PCs
exige fluxo proprio de exportacao/recovery/rekey.

### Portable passphrase

`PassphraseSecretProtector` protege o segredo com uma passphrase/recovery secret
usando `support-crypto`. O `ProtectedSecret` resultante pode ser levado para
outra maquina e desprotegido com a mesma passphrase.

Este modo e mais universal, mas a seguranca passa a depender da qualidade,
guarda e politica operacional dessa passphrase.

Helpers recomendados:

- `create_portable_key_provider()`
- `load_portable_key_provider()`

A recovery passphrase e validada por politica minima antes de proteger ou
reabrir a chave.

## Testes

```text
cargo test -p infra-secrets
```
