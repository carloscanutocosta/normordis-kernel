# MAN.md

## Nome

infra-backup

## Posicao arquitetural

```text
crates/kernel/infra/services/backup
```

Pertence a `kernel/infra/services` porque e um servico operacional sem conceito
de dominio proprio — igual a `infra-signing` e `infra-export`.

## Responsabilidade

Manutencao diaria automatica de mini-apps institucionais:

1. **Health check** — PRAGMA integrity_check + foreign key check + aviso WAL > 100 MB
2. **Backup** — VACUUM INTO (copia limpa SQLite) + tar.gz cifrado via `support-backup`
3. **Rotacao** — purga backups antigos alem de `keep_last`, com trilha de auditoria
4. **Agendamento** — loop async diario a hora configurada; corre imediatamente se a
   app arrancar apos a hora agendada

O `control.db` e separado das bases funcionais e nunca incluido no backup.

## Configuracao

```rust
MaintenanceConfig {
    schedule_time: "16:00",          // hora do agendamento diario
    destination_path: "//srv/bkp/",  // partilha de rede mapeada ou pasta local
    keep_last: 7,                    // numero de backups a reter
    db_paths: vec!["app.db", ...],   // bases SQLite a incluir
    control_db_path: "control.db",   // base de controlo (lock + logs)
    backup_passphrase: "...",        // passphrase de cifragem do arquivo final
}
```

## Integracao

```rust
use std::sync::Arc;
use infra_backup::{MaintenanceConfig, MaintenanceService};

let config = MaintenanceConfig { /* ... */ };
let svc = Arc::new(MaintenanceService::new(config)?);

// Agendador diario em background
tokio::spawn(infra_backup::scheduler::start(Arc::clone(&svc)));

// Trigger manual (ex: handler de administracao)
svc.run("manual").await?;
```

## Restore

```rust
// Listar runs bem-sucedidos
let runs = svc.repository().list_successful_backups().await?;
let run = &runs[0]; // mais recente

// Restaurar para diretorio de staging
let restored_files = svc.restore(run, Path::new("/tmp/restore")).await?;
// O chamador substitui as bases de dados ativas pelos ficheiros restaurados
```

O restore verifica o checksum SHA-256 antes de decifrar. Se o arquivo estiver
corrompido ou adulterado, retorna `MINI.BACKUP.ARCHIVE_FAILED`.

## Ciclo de manutencao

```text
start()
  └─ se hora agendada ja passou hoje → run("startup")
  └─ loop:
       sleep ate proxima hora agendada
       run("scheduler")
       sleep 90s (evita duplo disparo)

run(triggered_by):
  1. try_acquire_lock(run_date=hoje) → None se ja correu hoje (Ok silencioso)
  2. Para cada db_path (exceto control.db):
       a. health_check → HealthStatus
       b. Se nao Failed: VACUUM INTO staging/
       c. save_db_detail no control.db
  3. tar.gz do staging → cifrado com support-backup → .mbak no destino
  4. rotate_backups(keep_last)
  5. finalize_run(Success | Partial | Failed)
```

## Schema do control.db

```sql
maintenance_log (
  id TEXT PRIMARY KEY,
  run_date TEXT NOT NULL UNIQUE,  -- lock singleton por dia
  started_at TEXT NOT NULL,
  finished_at TEXT,
  triggered_by TEXT NOT NULL,
  status TEXT NOT NULL,           -- running | success | partial | failed | skipped
  backup_path TEXT,
  backup_size INTEGER,
  checksum TEXT,                  -- SHA-256 do ficheiro cifrado
  purged_at TEXT                  -- NULL enquanto activo; preenchido apos rotacao
)

maintenance_db_log (
  id TEXT PRIMARY KEY,
  maintenance_id TEXT NOT NULL REFERENCES maintenance_log(id),
  db_name TEXT NOT NULL,
  health_status TEXT NOT NULL,    -- ok | warning | failed
  health_detail TEXT,
  file_size_bytes INTEGER,
  wal_size_bytes INTEGER,
  last_modified_at TEXT,
  backup_included INTEGER NOT NULL DEFAULT 0,
  backup_status TEXT,
  error_message TEXT
)
```

## Contrato publico

- `MaintenanceService::new(config) -> Result<Self, BackupServiceError>`
- `MaintenanceService::run(triggered_by: &str) -> Result<(), BackupServiceError>`
- `MaintenanceService::restore(run, dest_dir) -> Result<Vec<PathBuf>, BackupServiceError>`
- `MaintenanceService::repository() -> &MaintenanceRepository`
- `MaintenanceRepository::try_acquire_lock`
- `MaintenanceRepository::finalize_run`
- `MaintenanceRepository::list_successful_backups`
- `MaintenanceRepository::list_all_runs`
- `MaintenanceRepository::get_run_details`
- `rotate_backups(repo, keep_last) -> Result<usize, BackupServiceError>`
- `scheduler::start(Arc<MaintenanceService>)`
- `MaintenanceConfig`, `MaintenanceRun`, `DbMaintenanceDetail`
- `MaintenanceStatus`, `HealthStatus`
- `BackupServiceError`

## Erros publicos

- `MINI.BACKUP.POLICY_INVALID`
- `MINI.BACKUP.IO_FAILED`
- `MINI.BACKUP.HEALTH_FAILED`
- `MINI.BACKUP.ARCHIVE_FAILED`
- `MINI.BACKUP.RETENTION_FAILED`
- `MINI.BACKUP.LOCK_HELD`
- `MINI.BACKUP.CONTROL_DB_FAILED`

## Garantias

- Um unico backup por dia por instancia (lock via `run_date UNIQUE`).
- Se uma BD falha o health check, as restantes sao processadas (status `Partial`).
- Backups purgados ficam no control.db com `purged_at` preenchido (auditoria permanente).
- O checksum SHA-256 e verificado no restore antes de decifrar.
- O staging usa `tempfile::tempdir()` — limpeza automatica no drop.
- O control.db e sempre excluido do backup.
- Arranque tardio (apos hora agendada) corre imediatamente e e idempotente.

## Limitacoes atuais

- Sem retry em caso de falha de escrita na partilha de rede.
- Sem timeout no VACUUM INTO (bases grandes podem bloquear o spawn_blocking).
- Backups `Partial` nao sao elegíveis para rotacao (so `success` e rotacionado).
- Sem notificacao de falha — o chamador deve capturar erros do scheduler.

## Ultima revisao

2026-05-21
