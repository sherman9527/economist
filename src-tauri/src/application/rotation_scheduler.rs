use crate::application::terminal_service::TerminalService;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub fn is_cst_30m_rotation_slot(
    weekday_mon0: u32,
    hour: u32,
    minute: u32,
) -> bool {
    if weekday_mon0 >= 5 {
        return false;
    }
    if minute != 0 && minute != 30 {
        return false;
    }
    let total_min = hour * 60 + minute;
    // Morning session: 09:30..=11:30 (570..=690); Afternoon session: 13:00..=15:00 (780..=900)
    (570..=690).contains(&total_min) || (780..=900).contains(&total_min)
}

pub fn spawn_rotation_scheduler(app_handle: AppHandle, service: Arc<TerminalService>) {
    tauri::async_runtime::spawn(async move {
        let mut last_slot_key = String::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(20));
        loop {
            interval.tick().await;
            let cst_now = chrono::Utc::now() + chrono::Duration::hours(8);
            use chrono::{Datelike, Timelike};
            let wd = cst_now.weekday().num_days_from_monday();
            let h = cst_now.hour();
            let m = cst_now.minute();
            if is_cst_30m_rotation_slot(wd, h, m) {
                let slot_key = cst_now.format("%Y-%m-%d %H:%M").to_string();
                if slot_key != last_slot_key {
                    last_slot_key = slot_key;
                    if let Ok(scan) = service.run_market_scan(true).await {
                        if let Some(alert) = scan.alert {
                            let _ = app_handle.emit("quant://rotation-alert", alert);
                        }
                    }
                    let _ = service.run_storage_hygiene().await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact_30m_ashare_trading_slots_mon_to_fri() {
        assert!(is_cst_30m_rotation_slot(0, 9, 30));
        assert!(is_cst_30m_rotation_slot(2, 11, 30));
        assert!(is_cst_30m_rotation_slot(4, 13, 0));
        assert!(is_cst_30m_rotation_slot(4, 14, 30));
        assert!(is_cst_30m_rotation_slot(4, 15, 0));

        assert!(!is_cst_30m_rotation_slot(0, 12, 0)); // lunch break
        assert!(!is_cst_30m_rotation_slot(5, 10, 0)); // Saturday
        assert!(!is_cst_30m_rotation_slot(1, 10, 15)); // non-30m boundary
    }
}
