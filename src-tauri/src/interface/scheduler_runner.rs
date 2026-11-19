use crate::application::rotation_scheduler::{
    cst_30m_rotation_slot_bucket, is_post_close_walk_forward_window_cst,
};
use crate::application::terminal_service::TerminalService;
use crate::domain::trading_calendar::is_ashare_trading_day;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub fn spawn_rotation_scheduler(app_handle: AppHandle, service: Arc<TerminalService>) {
    tauri::async_runtime::spawn(async move {
        let mut last_slot_key = String::new();
        let mut last_post_close_date = String::new();
        let mut last_hygiene_at: Option<Instant> = None;
        let hygiene_interval = Duration::from_secs(6 * 3600);
        let mut interval = tokio::time::interval(Duration::from_secs(20));

        loop {
            interval.tick().await;
            let cst_now = chrono::Utc::now() + chrono::Duration::hours(8);
            use chrono::{Datelike, Timelike};
            let cst_date = cst_now.date_naive();
            let date_str = cst_date.format("%Y-%m-%d").to_string();
            let wd = cst_now.weekday().num_days_from_monday();
            let h = cst_now.hour();
            let m = cst_now.minute();

            if is_ashare_trading_day(cst_date) {
                if let Some(bucket) = cst_30m_rotation_slot_bucket(wd, h, m) {
                    let slot_key = format!("{} {}", date_str, bucket);
                    if slot_key != last_slot_key {
                        last_slot_key = slot_key;
                        if let Ok(scan) = service.run_market_scan(true).await {
                            if let Some(alert) = scan.alert {
                                let _ = app_handle.emit("quant://rotation-alert", alert);
                            }
                            let _ = app_handle.emit("quant://state-synced", &date_str);
                        }
                        let _ = service.run_storage_hygiene().await;
                        last_hygiene_at = Some(Instant::now());
                        continue;
                    }
                }

                if is_post_close_walk_forward_window_cst(cst_date, h, m)
                    && last_post_close_date != date_str
                {
                    last_post_close_date = date_str.clone();
                    let already_ran = service
                        .repo
                        .has_post_close_evolution_for_date(&date_str)
                        .unwrap_or(false);
                    if !already_ran {
                        if service.scan_cache.read().await.is_none() {
                            let _ = service.run_market_scan(false).await;
                        }
                        if service
                            .run_anti_overfit_iteration("POST_CLOSE_1505")
                            .await
                            .is_ok()
                        {
                            let _ = app_handle.emit("quant://state-synced", &date_str);
                        }
                    }
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
