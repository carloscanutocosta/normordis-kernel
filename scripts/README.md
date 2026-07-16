# Scripts

Os scripts estão organizados por shell. Bash é a opção principal para o servidor Linux; os equivalentes PowerShell mantêm a compatibilidade com Windows.

## Bash (Linux)

- `bash/build-release.sh` — fmt, clippy, testes e build release, com cache Cargo fora do checkout.
- `bash/security/release-gate.sh` — manifesto, RustSec, licenças, build e relatório JSON.
- `bash/security/{generate-manifest,verify-manifest,audit-deps,check-licenses}.sh` — passos individuais de segurança.
- `bash/backup/full-repo-backup.sh` — cria um ZIP restaurável com `.git`, excluindo artefactos e credenciais locais.
- `bash/backup/full-repo-restore.sh` — restaura um ZIP para uma pasta indicada; recusa destinos não vazios sem `--force`.
- `bash/backup/sync-google-drive.sh` — envia ZIP para Google Drive, verifica checksums e mantém cinco cópias cloud.

```bash
bash scripts/bash/build-release.sh
bash scripts/bash/build-release.sh --skip-tests --with-docs
bash scripts/bash/build-release.sh --cargo-target-dir /srv/normordis/build-cache/normordis-kernel/target
bash scripts/bash/security/release-gate.sh --skip-build
bash scripts/bash/backup/full-repo-backup.sh --dest-dir /mnt/normordis-backup/backups/repos/normordis-kernel --keep-last 7
bash scripts/bash/backup/full-repo-restore.sh --list
bash scripts/bash/backup/full-repo-restore.sh --restore-dir /tmp/normordis-kernel-restore
bash scripts/bash/backup/sync-google-drive.sh --dry-run
RCLONE_CONFIG="$HOME/.config/rclone/rclone.conf" bash scripts/bash/backup/sync-google-drive.sh
```

Por omissão, o backup Bash usa `/mnt/normordis-backup/backups/repos/normordis-kernel`; pode ser alterado por `--dest-dir` ou `NORMORDIS_BACKUP_DIR`. Os ZIP excluem `target/`, logs, artefactos, temporários e ficheiros `.env`/`secrets`.

O builder Bash usa `~/.cache/normordis-kernel/target` por omissão, evitando criar `target/` dentro do checkout. Em servidores, indicar `--cargo-target-dir /srv/normordis/build-cache/normordis-kernel/target` permite manter compilação e temporários descartáveis fora do projeto.

O modelo 3-2-1 mantém o checkout ativo, um ZIP no SSD e cinco cópias offsite em `gdrive:backups/projetos/normordis-kernel`. A sincronização usa `rclone copy --immutable`, não propaga eliminações locais, valida a transferência com `rclone check --one-way` e só depois aplica a retenção remota. A credencial OAuth fica exclusivamente em `~/.config/rclone/rclone.conf`, fora do Git.

Depois de confirmar `rclone listremotes` e o mount do SSD, instala os templates de agendamento:

```bash
mkdir -p ~/.config/systemd/user /srv/normordis/logs/backups
cp scripts/systemd/normordis-kernel-backup.service scripts/systemd/normordis-kernel-backup.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now normordis-kernel-backup.timer
```

Os logs ficam em `/srv/normordis/logs/backups/normordis-kernel-backup.log`.

## PowerShell (Windows)

- `powershell/build-release.ps1`
- `powershell/security/release-gate.ps1` e os passos individuais em `powershell/security/`
- `powershell/backup/full-repo-backup.ps1`
- `powershell/backup/full-repo-restore.ps1`

```powershell
.\scripts\powershell\build-release.ps1
.\scripts\powershell\security\release-gate.ps1 -SkipBuild
.\scripts\powershell\backup\full-repo-backup.ps1
```
