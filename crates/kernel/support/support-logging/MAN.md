# MAN.md

## Nome

support-logging

## Posicao arquitetural

```text
crates/kernel/support/support-logging
```

Pertence a `kernel/support` porque fornece diagnostico tecnico transversal,
headless e reutilizavel.

## Contrato publico

- `LoggingConfig`
- `LogLevel`
- `LogEvent`
- `TechnicalLogger`
- `FileLogger`
- `log_mini_error()`
- `LogError`

## Formato

O formato unico e `JSON Lines`:

```text
uma linha = um JSON valido
```

Nao ha logs multiline complexos nesta fase.

## Fronteira com auditoria

```text
support-logging = diagnostico tecnico operacional
core-audit = evidencia institucional auditavel
```

`support-logging` nao deve ser usado para prova institucional, cadeia de
custodia, historico funcional de negocio ou evidencias juridicas.

## Segurança

Nao logar automaticamente:

- passwords;
- recovery passphrases;
- secret keys;
- ciphertext completo;
- plaintext;
- dados pessoais desnecessarios;
- payloads documentais completos;
- stack traces completas por defeito.

`log_mini_error()` usa apenas `MiniError::to_public()`.

## Politicas de producao

`LoggingConfig` inclui:

- `min_level`: nivel minimo escrito no ficheiro;
- `max_message_chars`: limite de caracteres da mensagem;
- `max_details_bytes`: limite do JSON serializado em `details`;
- `flush_each_event`: controla `flush()` por evento.

Antes de escrever, o logger:

- remove quebras de linha de `component`, `code` e `message`;
- trunca mensagens longas;
- redige chaves sensiveis comuns em `details`, como `password`,
  `passphrase`, `secret`, `token`, `key`, `ciphertext`, `plaintext`,
  `payload`, `authorization` e `cookie`;
- substitui `details` demasiado grandes por `[TRUNCATED]`.

## Rotacao e retencao

- Rotacao por `max_file_size_mb`.
- Padrao: `app.log -> app.1.log -> app.2.log`.
- `max_files` limita ficheiros rotacionados.
- `retention_days` remove logs geridos antigos.
- Sem compressao, async runtime ou scheduler em background nesta fase.

## Limitacoes atuais

- Ficheiro local apenas.
- Sem compressao.
- Sem logger global.
- Retencao baseada em modified time do filesystem.

## ToDo

- Avaliar redatores configuraveis por componente.
- Avaliar compressao em rotacao se houver necessidade.

## Teste de stress

Existe um teste ignorado por defeito para 1 minuto de escrita concorrente em
JSONL, com validacao de rotação e redaction:

```text
cargo test -p support-logging --test stress_tests -- --ignored --nocapture
```

## Ultima revisao

2026-05-11
