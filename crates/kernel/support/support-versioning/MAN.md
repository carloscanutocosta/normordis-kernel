# NAME

support-versioning

# SYNOPSIS

Biblioteca headless para versionamento funcional local e release notes persistentes em JSON.

# DESCRIPTION

`support-versioning` centraliza a representacao de release notes e uma store simples em ficheiro para guardar `version`, `novidades`, `problemas_conhecidos` e `updated_at_utc`.

# PUBLIC CONTRACT

## Tipos publicos

- `ReleaseNotes`
- `FileReleaseNotesStore`
- `VersioningError`

## Funcoes e metodos publicos

- `ReleaseNotes::new`
- `ReleaseNotes::validate`
- `FileReleaseNotesStore::new`
- `FileReleaseNotesStore::path`
- `FileReleaseNotesStore::ensure_exists`
- `FileReleaseNotesStore::load`
- `FileReleaseNotesStore::save`
- `FileReleaseNotesStore::set_version`
- `FileReleaseNotesStore::add_novidade`
- `FileReleaseNotesStore::add_problema_conhecido`

# INVARIANTS

- `version` nao pode ser vazia
- itens de `novidades` nao podem ser vazios
- itens de `problemas_conhecidos` nao podem ser vazios
- a persistencia usa JSON em ficheiro local

# STATUS

Proposto

# LAST REVIEW

2026-03-25
