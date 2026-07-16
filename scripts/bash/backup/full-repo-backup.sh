#!/usr/bin/env bash
# Cria um ZIP restaurável do repositório, incluindo .git e sem artefactos locais.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
DEST_DIR="${NORMORDIS_BACKUP_DIR:-/mnt/normordis-backup/backups/repos/normordis-kernel}"
KEEP_LAST=7

usage() { printf '%s\n' 'Uso: bash scripts/bash/backup/full-repo-backup.sh [--dest-dir DIR] [--keep-last N]'; }
while (($#)); do
  case "$1" in
    --dest-dir) DEST_DIR="${2:?Falta o destino}"; shift ;;
    --keep-last) KEEP_LAST="${2:?Falta o número}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Argumento desconhecido: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done
[[ "$KEEP_LAST" =~ ^[0-9]+$ ]] || { printf '%s\n' '--keep-last tem de ser um inteiro não negativo.' >&2; exit 2; }
command -v zip >/dev/null 2>&1 || { printf '%s\n' 'zip não encontrado.' >&2; exit 1; }

timestamp="$(date -u '+%Y%m%d-%H%M%S')"
zip_path="$DEST_DIR/normordis-kernel-$timestamp.zip"
stage="$(mktemp -d "${TMPDIR:-/tmp}/normordis-kernel-backup.XXXXXX")"
cleanup() { rm -rf "$stage"; }
trap cleanup EXIT
mkdir -p "$DEST_DIR"

printf 'A preparar backup de %s\n' "$ROOT"
tar -C "$ROOT" \
  --exclude='./target' --exclude='./.logs' --exclude='./artifacts' --exclude='./tmp' \
  --exclude='./.env' --exclude='./.env.*' --exclude='./secrets' --exclude='./_backups' \
  -cf - . | tar -C "$stage" -xf -
(cd "$stage" && zip -qr "$zip_path" .)
zip -T "$zip_path" >/dev/null

if (( KEEP_LAST > 0 )); then
  mapfile -t old_backups < <(find "$DEST_DIR" -maxdepth 1 -type f -name 'normordis-kernel-*.zip' -printf '%T@ %p\n' | sort -nr | tail -n +$((KEEP_LAST + 1)) | cut -d' ' -f2-)
  ((${#old_backups[@]} == 0)) || rm -f -- "${old_backups[@]}"
fi
printf 'BACKUP CONCLUÍDO: %s\n' "$zip_path"
