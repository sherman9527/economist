use economist_quant_terminal_lib::application::terminal_service::TerminalService;
use economist_quant_terminal_lib::infrastructure::market_tencent::fetch_tencent_full_universe;
use economist_quant_terminal_lib::infrastructure::plugin_store::PluginStore;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use economist_quant_terminal_lib::infrastructure::telemetry_cleaner::contains_hardcoded_absolute_path;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(manifest_dir)
}

fn collect_owned_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name();
        let name_s = name.to_string_lossy();
        if name_s == "target"
            || name_s == ".build"
            || name_s == ".temp"
            || name_s == "icons"
            || name_s == "gen"
            || name_s == "node_modules"
        {
            continue;
        }
        if p.is_dir() {
            collect_owned_source_files(&p, out);
        } else if matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("rs" | "toml" | "json")
        ) {
            out.push(p);
        }
    }
}

fn compute_dir_bytes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut sum = 0_u64;
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(meta) = entry.metadata() {
            if meta.is_dir() {
                sum = sum.saturating_add(compute_dir_bytes(&p));
            } else {
                sum = sum.saturating_add(meta.len());
            }
        }
    }
    sum
}

#[test]
fn redline_1_zero_hardcoded_absolute_paths_across_owned_directories() {
    let root = workspace_root();
    let mut files = Vec::new();
    for owned in ["config", "plugins", "src-tauri"] {
        collect_owned_source_files(&root.join(owned), &mut files);
    }
    assert!(
        files.len() >= 25,
        "Expected at least 25 owned source/config/plugin files, found {}",
        files.len()
    );

    let mut violations = Vec::new();
    for file in &files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Failed reading {:?}: {}", file, e));
        if let Some(bad_snippet) = contains_hardcoded_absolute_path(&content) {
            let rel = file.strip_prefix(&root).unwrap_or(file);
            violations.push(format!("{:?} -> {}", rel, bad_snippet));
        }
    }
    assert!(
        violations.is_empty(),
        "Hardcoded absolute path violations detected:\n{}",
        violations.join("\n")
    );
}

#[test]
fn redline_2_every_owned_file_strictly_under_1000_lines_and_600_target() {
    let root = workspace_root();
    let mut files = Vec::new();
    for owned in ["config", "plugins", "src-tauri"] {
        collect_owned_source_files(&root.join(owned), &mut files);
    }

    for file in &files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Failed reading {:?}: {}", file, e));
        let line_count = content.lines().count();
        let rel = file.strip_prefix(&root).unwrap_or(file);
        assert!(
            line_count < 1000,
            "Hard redline violated: {:?} has {} lines (>= 1000)",
            rel,
            line_count
        );
        assert!(
            line_count <= 600,
            "Modular ceiling violated: {:?} has {} lines (> 600)",
            rel,
            line_count
        );
    }
}

#[test]
fn redline_3_portable_bundle_disabled_and_build_footprint_under_5gb() {
    let root = workspace_root();
    let tauri_conf_raw =
        std::fs::read_to_string(root.join("src-tauri").join("tauri.conf.json")).unwrap();
    let tauri_conf: serde_json::Value = serde_json::from_str(&tauri_conf_raw).unwrap();
    let bundle_active = tauri_conf
        .get("bundle")
        .and_then(|b| b.get("active"))
        .and_then(|v| v.as_bool());
    assert_eq!(
        bundle_active,
        Some(false),
        "tauri.conf.json must enforce bundle.active = false for portable x64 execution"
    );

    let build_bytes = compute_dir_bytes(&root.join(".build"));
    let cap_5gb: u64 = 5 * 1024 * 1024 * 1024;
    assert!(
        build_bytes < cap_5gb,
        "Build footprint {} bytes exceeded 5 GB cap ({} bytes)",
        build_bytes,
        cap_5gb
    );
}

#[tokio::test]
async fn redline_4_live_ashare_universe_fetches_over_5000_active_quotes() {
    let root = workspace_root();
    let paths = PortablePaths::from_root(root).unwrap();
    let bundle = PluginStore::new(paths).load_all().unwrap();

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) EconomistQuantTerminal/1.0")
        .timeout(Duration::from_millis(
            bundle.app_cfg.market_sources.request_timeout_ms.max(5000),
        ))
        .build()
        .unwrap();

    let quotes = fetch_tencent_full_universe(
        &client,
        &bundle.app_cfg.market_sources.tencent_batch_url,
        None,
        bundle.app_cfg.market_sources.batch_chunk_size,
        bundle.app_cfg.market_sources.max_concurrency,
    )
    .await
    .expect("Live Tencent GTimg full A-share batch ingestion must succeed");

    assert!(
        quotes.len() >= 5000,
        "Expected >= 5000 live active A-shares across SH/SZ/BJ, got {}",
        quotes.len()
    );

    let under_200yi_count = quotes
        .iter()
        .filter(|q| q.total_mv_yi > 0.0 && q.total_mv_yi < 200.0)
        .count();
    assert!(
        under_200yi_count >= 2500,
        "Expected >= 2500 small-cap (<200亿 RMB) live A-shares, got {}",
        under_200yi_count
    );
}

#[tokio::test]
async fn redline_5_end_to_end_terminal_bootstrap_and_dto_contract() {
    let root = workspace_root();
    let sandbox = root.join(".temp").join("test_e2e_bootstrap_sandbox");
    let _ = std::fs::remove_dir_all(&sandbox);
    let paths = PortablePaths::from_root(sandbox).unwrap();

    let svc = TerminalService::initialize(paths).unwrap();
    let boot = svc
        .bootstrap_terminal()
        .await
        .expect("End-to-end bootstrap_terminal with live market scan must succeed");

    assert!(boot.funnel.total_active_count >= 5000);
    assert!(boot.funnel.under_200_yi_count >= 2500);
    assert!(!boot.candidates.is_empty());
    assert!(boot.portfolio.open_slots_count <= 10);
    assert_eq!(boot.portfolio.max_slots, 10);
    assert!(
        ((boot.portfolio.cash_yuan + boot.portfolio.market_value_yuan)
            - boot.portfolio.total_equity_yuan)
            .abs()
            < 0.10
    );
    assert_eq!(boot.plugin_config.factors.len(), 6);
    assert_eq!(boot.telemetry.sessions.len(), 5);
    assert!(!boot.featured_kline.bars.is_empty());

    let serialized = serde_json::to_value(&boot).unwrap();
    assert!(serialized.get("featuredKline").is_some());
    assert!(serialized.get("pluginConfig").is_some());
    assert!(serialized["funnel"].get("under200YiCount").is_some());

    drop(svc);
    let _ = std::fs::remove_dir_all(&root.join(".temp").join("test_e2e_bootstrap_sandbox"));
}
