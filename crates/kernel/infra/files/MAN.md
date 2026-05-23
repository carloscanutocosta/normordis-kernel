# NAME

support-files

# SYNOPSIS

Biblioteca headless para resolucao de layout tecnico e utilitarios de nomes de ficheiro.

# DESCRIPTION

`support-files` transforma `PathsConfig` num `FileLayout` resolvido, garante a existencia das diretorias tecnicas da app e gera nomes tecnicos sanitizados para ficheiros.

# PUBLIC CONTRACT

## Tipos publicos

- `FileLayout`
- `FilesError`

## Funcoes publicas

- `resolve_layout`
- `ensure_directories`
- `generate_technical_filename`

# INVARIANTS

- o layout inclui `apps/.database`, `apps/.assets`, `tmp` e `apps/.logs`
- os ficheiros em `tmp` com mais de 7 dias sao removidos automaticamente ao iniciar a app
- `ensure_directories` cria todas as diretorias em falta
- nomes tecnicos gerados usam apenas caracteres seguros

# STATUS

Proposto

# LAST REVIEW

2026-03-25
