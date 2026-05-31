use std::sync::Arc;

use chrono::Local;
use tokio::time::{sleep, Duration};

use crate::service::MaintenanceService;

/// Inicia o agendador diário que executa a manutenção à hora configurada.
///
/// Se a hora agendada já passou hoje (app iniciada tardiamente), corre imediatamente.
/// O lock singleton garante idempotência — se já correu hoje, retorna silenciosamente.
/// Deve ser lançado com `tokio::spawn`.
pub async fn start(service: Arc<MaintenanceService>) {
    if scheduled_time_passed_today(&service) {
        if let Err(e) = service.run("startup").await {
            eprintln!("[infra-backup] startup maintenance run failed: {e}");
        }
    }

    loop {
        let delay = next_run_delay(&service);
        sleep(delay).await;

        let svc = Arc::clone(&service);
        tokio::spawn(async move {
            if let Err(e) = svc.run("scheduler").await {
                eprintln!("[infra-backup] maintenance run failed: {e}");
            }
        });

        // Aguarda 90 segundos antes de recalcular, evitando duplo disparo no mesmo minuto.
        sleep(Duration::from_secs(90)).await;
    }
}

fn scheduled_time_passed_today(service: &MaintenanceService) -> bool {
    let (hour, minute) = service.config().schedule_hour_minute();
    let now = Local::now();
    let today_target = now
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .expect("schedule time must be valid");
    now >= today_target
}

fn next_run_delay(service: &MaintenanceService) -> Duration {
    let (hour, minute) = service.config().schedule_hour_minute();

    let now = Local::now();
    let today_target = now
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .and_then(|dt| dt.and_local_timezone(Local).single())
        .expect("schedule time must be valid");

    let target = if today_target > now {
        today_target
    } else {
        today_target + chrono::Duration::days(1)
    };

    let secs = (target - now).num_seconds().max(1) as u64;
    Duration::from_secs(secs)
}
