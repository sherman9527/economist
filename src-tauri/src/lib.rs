pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use crate::application::rotation_scheduler::spawn_rotation_scheduler;
use crate::application::terminal_service::TerminalService;
use crate::infrastructure::portable_paths::PortablePaths;
use crate::interface::commands::{
    ack_economist_report, bootstrap_terminal, close_simulated_position, get_economist_report_html,
    get_stock_kline, place_simulated_buy, run_market_scan, run_storage_hygiene,
    update_plugin_config, wake_agent_session,
};
use std::sync::Arc;

pub fn run() {
    let portable_paths =
        PortablePaths::resolve().expect("解析便携式运行根目录(./)失败");
    portable_paths.apply_process_env();

    let service = Arc::new(
        TerminalService::initialize(portable_paths.clone())
            .expect("初始化量化终端核心引擎失败"),
    );
    let setup_service = service.clone();
    let webview_data_dir = portable_paths.webview_data_dir();

    tauri::Builder::default()
        .manage(service)
        .setup(move |app| {
            let _main_window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("经济学人 · A股小市值双日换手量化指挥台")
            .inner_size(1480.0, 920.0)
            .min_inner_size(390.0, 560.0)
            .resizable(true)
            .data_directory(webview_data_dir)
            .build()?;

            spawn_rotation_scheduler(app.handle().clone(), setup_service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_terminal,
            run_market_scan,
            get_stock_kline,
            place_simulated_buy,
            close_simulated_position,
            update_plugin_config,
            ack_economist_report,
            get_economist_report_html,
            run_storage_hygiene,
            wake_agent_session,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 2 便携式量化终端失败");
}
