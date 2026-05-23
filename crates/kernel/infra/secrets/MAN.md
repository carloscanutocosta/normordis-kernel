# MAN.md

## Nome

infra-secrets

## Posicao arquitetural

```text
crates/kernel/infra/secrets
```

Este crate pertence a `kernel/infra` porque integra o Mini-Kernel RS com um
mecanismo concreto de protecao de segredos do sistema operativo.

## Objetivo

Proteger material de chave usado por `support-crypto` e futuro
`support-storage`, mantendo `support-crypto` livre de dependencias de sistema
operativo, filesystem, UI ou Tauri.

## Contrato publico

- `SecretScope`
- `SecretsConfig`
- `ProtectedSecret`
- `SecretProtector`
- `ProtectedKeyProvider`
- `generate_secret_key()`
- `create_portable_key_provider()`
- `load_portable_key_provider()`
- `RecoveryPassphrasePolicy`
- `PassphraseSecretProtector`
- `DpapiSecretProtector` em Windows

## Fluxo esperado

1. Gerar uma chave com `generate_secret_key()`.
2. Proteger os bytes com `SecretProtector::protect()`.
3. Persistir `ProtectedSecret` numa camada adequada.
4. Criar `ProtectedKeyProvider` com o `ProtectedSecret` e `KeyId`.
5. Passar o provider ao futuro `support-storage`.

Para o modelo sem instalacao adicional e multi-maquina, preferir:

```text
create_portable_key_provider()
load_portable_key_provider()
```

## Segurança

- Em Windows, usa DPAPI.
- Para portabilidade, suporta `PassphraseSecretProtector`.
- `ProtectedSecret` pode ser serializado, mas nao deve ser tratado como
  plaintext.
- Erros publicos nao expõem segredos nem bytes protegidos.
- Material desprotegido temporario usa `Zeroizing<Vec<u8>>`.

## Modos de protecao

### Windows DPAPI

`ProtectedSecret` protegido por Windows DPAPI nao e portavel por defeito:

- `SecretScope::CurrentUser`: so deve ser desprotegido pelo mesmo utilizador no
  mesmo perfil Windows;
- `SecretScope::LocalMachine`: fica ligado a maquina Windows local.

Isto e desejavel para protecao local, mas nao resolve migracao entre
computadores. Para outro Windows, e necessario um fluxo explicito de exportacao,
recovery key, passphrase de recuperacao ou recifragem/rekey.

### Portable passphrase

`PassphraseSecretProtector` cria `ProtectedSecret` com backend
`portable-passphrase-v1`. Este modo e portavel entre Windows diferentes, e entre
plataformas futuras, desde que a mesma passphrase/recovery secret esteja
disponivel.

Trade-off: deixa de depender da identidade local do sistema operativo e passa a
depender da seguranca operacional da passphrase.

A politica minima atual exige pelo menos 16 caracteres uteis.

## Limitacoes atuais

- Implementacao OS-bound apenas para Windows DPAPI.
- Nao fornece persistencia do `ProtectedSecret`.
- Nao fornece rotacao de chaves automatica.
- Nao implementa keyring Linux/macOS.
- Nao fornece ainda UI, escrow ou governanca de recovery passphrase.

## ToDo

- Adicionar adapters macOS Keychain e Linux Secret Service quando necessario.
- Definir runtime/bootstrap responsavel por persistir e recuperar
  `ProtectedSecret`.
- Adicionar estrategia de rotacao e recifragem coordenada com `support-storage`.
- Definir politica de recovery passphrase, escrow e rotacao organizacional.

## Ultima revisao

2026-05-11
