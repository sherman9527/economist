use economist_quant_terminal_lib::infrastructure::economist_bridge::EconomistBridge;
use economist_quant_terminal_lib::infrastructure::portable_paths::PortablePaths;
use std::path::PathBuf;

fn temp_portable_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "econ_r5_{}_{}",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp portable root");
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Round 5 Gate 1: When `<head lang="zh-CN">` carries attributes, `EconomistBridge::read_report_html`
/// MUST still inject `<meta data-economist-csp="1" ...>` immediately after the opening `<head...>` tag,
/// BEFORE any inlined `<script>` tag inside `<head>`.
#[test]
fn round5_embed_csp_precedes_scripts_even_when_head_has_attributes() {
    let root = temp_portable_root("head_attr_csp");
    let paths = PortablePaths::from_root(root.clone()).expect("portable paths");
    let assets_dir = paths.reports_dir().join("assets");
    let day_dir = paths.reports_dir().join("2026").join("11").join("18");
    std::fs::create_dir_all(&assets_dir).unwrap();
    std::fs::create_dir_all(&day_dir).unwrap();

    std::fs::write(
        assets_dir.join("echarts.min.js"),
        "window.__ECHARTS_READY__ = true;",
    )
    .unwrap();
    std::fs::write(
        day_dir.join("economist_report_2026-11-18.html"),
        concat!(
            "<!DOCTYPE html><html lang=\"zh-CN\"><head data-theme=\"dark\">",
            "<meta charset=\"utf-8\">",
            "<script src=\"../../../assets/echarts.min.js\"></script>",
            "</head><body><h1>Report</h1></body></html>"
        ),
    )
    .unwrap();

    let bridge = EconomistBridge::new(paths);
    let rendered = bridge
        .read_report_html("./reports/2026/11/18/economist_report_2026-11-18.html")
        .expect("embedded html");

    let csp_pos = rendered
        .find("data-economist-csp=\"1\"")
        .expect("CSP meta injected");
    let script_pos = rendered
        .find("data-inlined-echarts=\"1\"")
        .expect("ECharts script inlined");

    assert!(
        csp_pos < script_pos,
        "CSP meta tag ({}) MUST appear BEFORE inlined script ({}) inside <head ...> so CSP governs all scripts",
        csp_pos,
        script_pos
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Round 5 Gate 2: `tauri.conf.json` CSP must explicitly permit Tauri v2 custom protocol origins
/// (`http://tauri.localhost` / `tauri:`) in `connect-src` and `worker-src 'self' blob:` for canvas charts,
/// while preserving `default-src 'self'`, `script-src 'self'`, `object-src 'none'`, and `base-uri 'none'`.
#[test]
fn round5_tauri_csp_supports_webview2_custom_protocol_and_chart_workers() {
    let conf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let raw = std::fs::read_to_string(&conf_path).expect("read tauri.conf.json");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    let csp = json
        .pointer("/app/security/csp")
        .and_then(|v| v.as_str())
        .expect("csp string");

    assert!(csp.contains("http://tauri.localhost"), "CSP connect-src must allow http://tauri.localhost: {csp}");
    assert!(csp.contains("worker-src 'self' blob:"), "CSP must allow self/blob chart workers: {csp}");
    assert!(csp.contains("script-src 'self'"), "CSP must preserve strict script-src 'self': {csp}");
    assert!(!csp.contains("unsafe-eval"), "CSP must forbid unsafe-eval: {csp}");
}

/// Round 5 Gate 3: `stage_portable_release.ps1` must stage the complete runtime Python module set
/// (`scripts/*.py` + `scripts/renderer_pkg/*.py`) rather than only `analyzer.py`, so moving
/// `./portable-release/` anywhere preserves standalone Economist report generation.
#[test]
fn round5_stage_portable_script_copies_complete_economist_python_package() {
    let script_path = repo_root().join("scripts").join("stage_portable_release.ps1");
    let script = std::fs::read_to_string(&script_path).expect("read stage_portable_release.ps1");
    assert!(
        script.contains("renderer_pkg"),
        "stage_portable_release.ps1 must copy scripts/renderer_pkg for standalone portable execution"
    );
}
