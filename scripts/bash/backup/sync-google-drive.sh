#!/usr/bin/env bash
# Copia os backups locais para Google Drive sem alterar históricos existentes.
set -Eeuo pipefail

SOURCE_DIR="${NORMORDIS_BACKUP_DIR:-/mnt/normordis-backup/backups/repos/normordis-kernel}"
REMOTE="${NORMORDIS_RCLONE_REMOTE:-gdrive:backups/projetos/normordis-kernel}"
DRY_RUN=false
KEEP_LAST=5

usage() {
  cat <<'EOF'
Uso: sync-google-drive.sh [--source-dir PASTA] [--remote REMOTE]
                            [--keep-last N] [--dry-run]

Copia os backups locais para Google Drive e mantém os N mais recentes.
Por omissão, usa gdrive:backups/projetos/normordis-kernel.
EOF
}

while (($#)); do
  case "$1" in
    --source-dir) SOURCE_DIR="${2:?Falta a pasta após --source-dir.}"; shift ;;
    --remote) REMOTE="${2:?Falta o destino após --remote.}"; shift ;;
    --keep-last)
      KEEP_LAST="${2:?Falta o número após --keep-last.}"; shift
      [[ "$KEEP_LAST" =~ ^[1-9][0-9]*$ ]] || { printf '%s\n' '--keep-last requer um inteiro positivo.' >&2; exit 2; } ;;
    --dry-run) DRY_RUN=true ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'Opção desconhecida: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

command -v rclone >/dev/null 2>&1 || { printf '%s\n' 'O comando rclone não está instalado.' >&2; exit 1; }
[[ -d "$SOURCE_DIR" ]] || { printf 'A pasta local de backups não existe: %s\n' "$SOURCE_DIR" >&2; exit 1; }
[[ "$REMOTE" == *:* ]] || { printf 'Destino rclone inválido (esperado remote:caminho): %s\n' "$REMOTE" >&2; exit 2; }

REMOTE_NAME="${REMOTE%%:*}:"
rclone listremotes | grep -Fqx -- "$REMOTE_NAME" || {
  printf 'Remote rclone não configurado: %s\n' "$REMOTE_NAME" >&2
  exit 1
}

LOCK_FILE="${TMPDIR:-/tmp}/normordis-kernel-rclone.lock"
exec 9>"$LOCK_FILE"
flock -n 9 || { printf '%s\n' 'Já existe uma cópia cloud do normordis-kernel em execução.' >&2; exit 1; }

printf '>>> [CLOUD] Origem: %s\n>>> [CLOUD] Destino: %s\n' "$SOURCE_DIR" "$REMOTE"
COPY_ARGS=(copy "$SOURCE_DIR" "$REMOTE" --immutable --checksum --check-first --transfers 4 --checkers 8)
if $DRY_RUN; then
  COPY_ARGS+=(--dry-run)
  printf '%s\n' '>>> [CLOUD] Simulação: nenhum ficheiro será enviado.'
fi
rclone "${COPY_ARGS[@]}"

if ! $DRY_RUN; then
  printf '%s\n' '>>> [CLOUD] A verificar os ficheiros enviados...'
  rclone check "$SOURCE_DIR" "$REMOTE" --one-way
fi

mapfile -t REMOTE_BACKUPS < <(
  rclone lsf "$REMOTE" --files-only 2>/dev/null \
    | grep -E '^normordis-kernel-[0-9]{8}-[0-9]{6}[.]zip$' \
    | LC_ALL=C sort -r || true
) || true
if ((${#REMOTE_BACKUPS[@]} > KEEP_LAST)); then
  OLD_BACKUPS=("${REMOTE_BACKUPS[@]:KEEP_LAST}")
  if $DRY_RUN; then
    printf '>>> [CLOUD] Simulação da retenção: %d backup(s) seriam removidos:\n' "${#OLD_BACKUPS[@]}"
    printf '    %s\n' "${OLD_BACKUPS[@]}"
  else
    for BACKUP_NAME in "${OLD_BACKUPS[@]}"; do
      rclone deletefile "${REMOTE%/}/$BACKUP_NAME"
      printf '>>> [CLOUD] Retenção: removido %s\n' "$BACKUP_NAME"
    done
  fi
fi

printf '>>> [CLOUD] Concluído com sucesso; retenção: %d backup(s).\n' "$KEEP_LAST"
