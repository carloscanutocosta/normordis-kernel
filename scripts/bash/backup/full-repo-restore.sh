#!/usr/bin/env bash
# Restaura um ZIP criado por full-repo-backup.sh para uma pasta explicitamente indicada.
set -euo pipefail

BACKUP_DIR="${NORMORDIS_BACKUP_DIR:-/mnt/normordis-backup/backups/repos/normordis-kernel}"
BACKUP_FILE=""
RESTORE_DIR=""
REBUILD=0
LIST=0
FORCE=0

usage() { printf '%s\n' 'Uso: bash scripts/bash/backup/full-repo-restore.sh [--list] [--backup-dir DIR] [--backup-file ZIP] --restore-dir DIR [--rebuild] [--force]'; }
while (($#)); do
  case "$1" in
    --list) LIST=1 ;;
    --backup-dir) BACKUP_DIR="${2:?Falta a pasta}"; shift ;;
    --backup-file) BACKUP_FILE="${2:?Falta o ZIP}"; shift ;;
    --restore-dir) RESTORE_DIR="${2:?Falta o destino}"; shift ;;
    --rebuild) REBUILD=1 ;;
    --force) FORCE=1 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Argumento desconhecido: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done
if (( LIST )); then
  find "$BACKUP_DIR" -maxdepth 1 -type f -name 'normordis-kernel-*.zip' -printf '%TY-%Tm-%Td %TH:%TM %10s %f\n' 2>/dev/null | sort -r || true
  exit 0
fi
[[ -n "$RESTORE_DIR" ]] || { printf '%s\n' '--restore-dir é obrigatório.' >&2; exit 2; }
command -v unzip >/dev/null 2>&1 || { printf '%s\n' 'unzip não encontrado.' >&2; exit 1; }
if [[ -z "$BACKUP_FILE" ]]; then
  BACKUP_FILE="$(find "$BACKUP_DIR" -maxdepth 1 -type f -name 'normordis-kernel-*.zip' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n1 | cut -d' ' -f2-)"
fi
[[ -n "$BACKUP_FILE" && -f "$BACKUP_FILE" ]] || { printf 'Backup não encontrado: %s\n' "$BACKUP_FILE" >&2; exit 1; }
if [[ -d "$RESTORE_DIR" ]] && [[ -n "$(find "$RESTORE_DIR" -mindepth 1 -maxdepth 1 -print -quit)" ]] && (( ! FORCE )); then
  printf 'O destino não está vazio; usa --force para continuar: %s\n' "$RESTORE_DIR" >&2
  exit 1
fi
mkdir -p "$RESTORE_DIR"
unzip -q "$BACKUP_FILE" -d "$RESTORE_DIR"
if (( REBUILD )); then (cd "$RESTORE_DIR" && cargo fetch); fi
printf 'RESTAURO CONCLUÍDO: %s\n' "$RESTORE_DIR"
