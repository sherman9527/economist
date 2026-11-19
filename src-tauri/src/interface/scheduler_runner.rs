use crate::application::rotation_scheduler::cst_30m_rotation_slot_bucket;
use crate::application::terminal_service::TerminalService;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub fn spawn_rotation_scheduler(app_handle: AppHandle, service: Arc<TerminalService>) {
    tauri::async_runtime::spawn(async move {
        let mut last_slot_key = String::new();
        let mut last_hygiene_at: Option<Instant> = None;
        let hygiene_interval = Duration::from_secs(6 * 3600);
        let mut interval = tokio::time::interval(Duration::from_secs(20));

        loop {
            interval.tick().await;
            let cst_now = chrono::Utc::now() + chrono::Duration::hours(8);
            use chrono::{Datelike, Timelike};
            let wd = cst_now.weekday().num_days_from_monday();
            let h = cst_now.hour();
            let m = cst_now.minute();

            if let Some(bucket) = cst_30m_rotation_slot_bucket(wd, h, m) {
                let slot_key = format!("{} {}", cst_now.format("%Y-%m-%d"), bucket);
                if slot_key != last_slot_key {
                    last_slot_key = slot_key;
                    if let Ok(scan) = service.run_market_scan(true).await {
                        if let Some(alert) = scan.alert {
                            let _ = app_handle.emit("quant://rotation-alert", alert);
                        }
                    }
                    if bucket == "15:00" {
                        let _ = service.run_anti_overfit_iteration("POST_CLOSE_1500").await;
                    }
                    let _ = service.run_storage_hygiene().await;
                    last_hygiene_at = Some(Instant::now());
                    continue;
                }
            }

            let needs_off_hours_hygiene = last_hygiene_at
                .map(|t| t.elapsed() >= hygiene_interval)
                .unwrap_or(true);
            if needs_off_hours_hygiene {
                let _ = service.run_storage_hygiene().await;
                last_hygiene_at = Some(Instant::now());
            }
        }
    });
}
