use crate::infrastructure::portable_paths::PortablePaths;
use crate::infrastructure::sqlite_repo::SqliteRepository;

#[derive(Debug, Clone)]
pub struct EconomistChainItem {
    pub chain_title: String,
    pub economist_trigger: String,
    pub us_mapping: String,
    pub cn_mapping: String,
    pub divergence_insight: String,
    pub related_cn_symbols: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EconomistReportRow {
    pub report_id: String,
    pub date: String,
    pub title_zh: String,
    pub rel_path: String,
    pub article_count: usize,
    pub core_count: usize,
    pub acked: bool,
    pub acked_at: Option<String>,
    pub us_score: f64,
    pub cn_score: f64,
    pub executive_summary_zh: String,
    pub chains: Vec<EconomistChainItem>,
}

#[derive(Debug, Clone)]
pub struct EconomistSnapshot {
    pub unacked_count: usize,
    pub reports: Vec<EconomistReportRow>,
    pub backtest_hit_rate_pct: f64,
    pub us_hit_rate_pct: f64,
    pub cn_hit_rate_pct: f64,
    pub bearish_risk_sectors_zh: Vec<String>,
}

pub struct EconomistBridge {
    paths: PortablePaths,
}

fn extract_cn_symbols_from_text(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    if bytes.len() < 9 {
        return found;
    }
    let mut i = 0;
    while i + 9 <= bytes.len() {
        let prev_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let next_ok = i + 9 == bytes.len() || !bytes[i + 9].is_ascii_alphanumeric();
        if prev_ok
            && next_ok
            && bytes[i..i + 6].iter().all(|b| b.is_ascii_digit())
            && bytes[i + 6] == b'.'
        {
            let suf = &bytes[i + 7..i + 9];
            if suf.eq_ignore_ascii_case(b"SZ")
                || suf.eq_ignore_ascii_case(b"SH")
                || suf.eq_ignore_ascii_case(b"BJ")
            {
                let digits = std::str::from_utf8(&bytes[i..i + 6]).unwrap_or("");
                let suffix = std::str::from_utf8(suf).unwrap_or("").to_ascii_uppercase();
                let sym = format!("{}.{}", digits, suffix);
                if !found.contains(&sym) {
                    found.push(sym);
                }
                i += 9;
                continue;
            }
        }
        i += 1;
    }
    found
}

fn inject_embed_flag_on_html(raw: &str) -> String {
    if raw.contains("data-embed=") {
        return raw.to_string();
    }
    let lower = raw.to_ascii_lowercase();
    if let Some(pos) = lower.find("<html") {
        let insert_at = pos + 5;
        let mut out = String::with_capacity(raw.len() + 16);
        out.push_str(&raw[..insert_at]);
        out.push_str(" data-embed=\"1\"");
        out.push_str(&raw[insert_at..]);
        out
    } else {
        raw.to_string()
    }
}

fn inline_echarts_script_tags(html: &str, echarts_js: &str) -> String {
    if !html.contains("echarts.min.js") {
        return html.to_string();
    }
    let safe_js = echarts_js
        .replace("</script", "<\\/script")
        .replace("</SCRIPT", "<\\/SCRIPT");
    let inline_tag = format!("<script data-inlined-echarts=\"1\">{}</script>", safe_js);

    let mut out = String::with_capacity(html.len() + safe_js.len() + 64);
    let mut cursor = 0usize;
    while let Some(rel_pos) = html[cursor..].find("echarts.min.js") {
        let hit = cursor + rel_pos;
        let before = &html[cursor..hit];
        let after = &html[hit..];
        let script_open = before
            .rfind("<script")
            .or_else(|| before.rfind("<SCRIPT"));
        let script_close = after
            .find("</script>")
            .or_else(|| after.find("</SCRIPT>"));
        if let (Some(open_rel), Some(close_rel)) = (script_open, script_close) {
            let open_abs = cursor + open_rel;
            let close_end_abs = hit + close_rel + "</script>".len();
            out.push_str(&html[cursor..open_abs]);
            out.push_str(&inline_tag);
            cursor = close_end_abs;
        } else {
            out.push_str(&html[cursor..hit + "echarts.min.js".len()]);
            cursor = hit + "echarts.min.js".len();
        }
    }
    out.push_str(&html[cursor..]);
    out
}

fn prepare_embedded_report_html(raw: &str, echarts_js: Option<&str>) -> String {
    let with_flag = inject_embed_flag_on_html(raw);
    let with_echarts = match echarts_js {
        Some(js) if !js.is_empty() => inline_echarts_script_tags(&with_flag, js),
        _ => with_flag,
    };
    let embed_style = "<style data-economist-embed=\"1\">html[data-embed=\"1\"] #ackBtn,html[data-embed=\"1\"] .ack-btn{display:none!important;}</style>";
    if with_echarts.contains("data-economist-embed=") {
        return with_echarts;
    }
    let lower = with_echarts.to_ascii_lowercase();
    if let Some(head_close) = lower.find("</head>") {
        let mut out = String::with_capacity(with_echarts.len() + embed_style.len());
        out.push_str(&with_echarts[..head_close]);
        out.push_str(embed_style);
        out.push_str(&with_echarts[head_close..]);
        out
    } else {
        format!("{}{}", embed_style, with_echarts)
    }
}

fn extract_date_token(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for w in bytes.windows(10) {
        if w[..4].iter().all(|b| b.is_ascii_digit())
            && w[4] == b'-'
            && w[5..7].iter().all(|b| b.is_ascii_digit())
            && w[7] == b'-'
            && w[8..10].iter().all(|b| b.is_ascii_digit())
        {
            return std::str::from_utf8(w).ok().map(|v| v.to_string());
        }
    }
    None
}

fn collect_html_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_html_files(&p, out, depth + 1);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("html"))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

impl EconomistBridge {
    pub fn new(paths: PortablePaths) -> Self {
        Self { paths }
    }

    pub fn load_and_sync(&self, repo: &SqliteRepository) -> Result<EconomistSnapshot, String> {
        use crate::infrastructure::portable_paths::normalize_rel_path;

        let ack_path = self.paths.data_dir().join("ack_state.json");
        let ack_root: serde_json::Value = if ack_path.is_file() {
            std::fs::read_to_string(&ack_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({ "reports": {} }))
        } else {
            serde_json::json!({ "reports": {} })
        };
        let ack_reports_obj = ack_root
            .get("reports")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Load runs/analysis_*.json indexed by date
        let runs_dir = self.paths.data_dir().join("runs");
        let mut runs_map: std::collections::BTreeMap<String, serde_json::Value> =
            std::collections::BTreeMap::new();
        if let Ok(entries) = std::fs::read_dir(&runs_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                    if fname.starts_with("analysis_") && fname.ends_with(".json") {
                        if let Some(dt) = extract_date_token(fname) {
                            if let Ok(raw) = std::fs::read_to_string(&p) {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                                    runs_map.insert(dt, v);
                                }
                            }
                        }
                    }
                }
            }
        }
        let latest_run_date = runs_map.keys().next_back().cloned();

        // Discover HTML reports from ./reports/** AND ack_state.json
        let mut html_files = Vec::new();
        collect_html_files(&self.paths.reports_dir(), &mut html_files, 0);

        let mut report_rows: std::collections::BTreeMap<String, EconomistReportRow> =
            std::collections::BTreeMap::new();
        let mut bearish_risk_sectors_zh: Vec<String> = Vec::new();

        for html_path in html_files {
            let Ok(stripped) = html_path.strip_prefix(self.paths.root()) else {
                continue;
            };
            let rel_candidate = format!("./{}", stripped.to_string_lossy().replace('\\', "/"));
            let Ok(rel_path) = normalize_rel_path(&rel_candidate) else {
                continue;
            };
            let fname = html_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("report.html");
            let Some(date) = extract_date_token(fname) else {
                continue;
            };

            let ack_meta = ack_reports_obj.get(&date);
            let mut title_zh = ack_meta
                .and_then(|m| m.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut article_count = ack_meta
                .and_then(|m| m.get("article_count"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let mut core_count = ack_meta
                .and_then(|m| m.get("core_count"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let acked = ack_meta
                .and_then(|m| m.get("acked"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let acked_at = ack_meta
                .and_then(|m| m.get("acked_at"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut us_score = 55.0_f64;
            let mut cn_score = 62.0_f64;
            let mut executive_summary_zh =
                format!("《经济学人》{} 全球宏观与中美跨市场映射深度简报", date);
            let mut chains = Vec::new();

            if let Some(run_val) = runs_map.get(&date) {
                if article_count == 0 {
                    article_count = run_val
                        .get("total_articles")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                }
                if core_count == 0 {
                    core_count = run_val
                        .get("core_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                }
                if let Some(ms) = run_val.get("macro_synthesis") {
                    if title_zh.is_empty() {
                        if let Some(hl) = ms.get("headline_zh").and_then(|v| v.as_str()) {
                            title_zh = hl.to_string();
                        }
                    }
                    if let Some(es) = ms.get("executive_summary").and_then(|v| v.as_str()) {
                        executive_summary_zh = es.to_string();
                    }
                    if let Some(sc) = ms
                        .get("us_market_overview")
                        .and_then(|u| u.get("sentiment_score"))
                        .and_then(|v| v.as_f64())
                    {
                        us_score = sc;
                    }
                    if let Some(cn_ov) = ms.get("cn_market_overview") {
                        if let Some(sc) = cn_ov.get("sentiment_score").and_then(|v| v.as_f64()) {
                            cn_score = sc;
                        }
                        if latest_run_date.as_deref() == Some(date.as_str()) {
                            if let Some(uw_arr) = cn_ov
                                .get("long_short_allocation")
                                .and_then(|ls| ls.get("underweight"))
                                .and_then(|v| v.as_array())
                            {
                                for uw in uw_arr {
                                    if let Some(txt) = uw.as_str() {
                                        let label = txt
                                            .split(&['（', '('][..])
                                            .next()
                                            .unwrap_or(txt)
                                            .trim();
                                        if !label.is_empty()
                                            && !bearish_risk_sectors_zh.iter().any(|x| x == label)
                                        {
                                            bearish_risk_sectors_zh.push(label.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(ch_arr) = ms.get("cross_market_chains").and_then(|v| v.as_array())
                    {
                        for ch in ch_arr {
                            let cn_map = ch
                                .get("cn_mapping")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let related_cn_symbols = extract_cn_symbols_from_text(&cn_map);
                            chains.push(EconomistChainItem {
                                chain_title: ch
                                    .get("chain_title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("跨市场传导链")
                                    .to_string(),
                                economist_trigger: ch
                                    .get("economist_trigger")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                us_mapping: ch
                                    .get("us_mapping")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                cn_mapping: cn_map,
                                divergence_insight: ch
                                    .get("divergence_insight")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                related_cn_symbols,
                            });
                        }
                    }
                }
            }

            if title_zh.is_empty() {
                title_zh = format!("《经济学人》精读 · {} 跨市场深度研报", date);
            }

            report_rows.insert(
                date.clone(),
                EconomistReportRow {
                    report_id: date.clone(),
                    date,
                    title_zh,
                    rel_path,
                    article_count,
                    core_count,
                    acked,
                    acked_at,
                    us_score,
                    cn_score,
                    executive_summary_zh,
                    chains,
                },
            );
        }

        let mut reports: Vec<EconomistReportRow> = report_rows.into_values().collect();
        reports.sort_by(|a, b| b.date.cmp(&a.date));

        // Read hit-rate statistics from ./data/backtest_ledger.json
        let mut backtest_hit_rate_pct = 52.4_f64;
        let mut us_hit_rate_pct = 59.6_f64;
        let mut cn_hit_rate_pct = 46.8_f64;
        let bt_path = self.paths.data_dir().join("backtest_ledger.json");
        if bt_path.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&bt_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(rep_obj) = val.get("reports").and_then(|v| v.as_object()) {
                        let mut total_eval = 0.0_f64;
                        let mut total_hits = 0.0_f64;
                        let mut us_sum = 0.0_f64;
                        let mut cn_sum = 0.0_f64;
                        let mut cnt = 0.0_f64;
                        for r in rep_obj.values() {
                            total_eval +=
                                r.get("total_evaluated").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            total_hits +=
                                r.get("total_hits").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            if let Some(u) = r.get("us_hit_rate").and_then(|v| v.as_f64()) {
                                us_sum += u;
                                cnt += 1.0;
                            }
                            if let Some(c) = r.get("cn_hit_rate").and_then(|v| v.as_f64()) {
                                cn_sum += c;
                            }
                        }
                        if total_eval > 0.0 {
                            backtest_hit_rate_pct =
                                ((total_hits / total_eval) * 1000.0).round() / 10.0;
                        }
                        if cnt > 0.0 {
                            us_hit_rate_pct = ((us_sum / cnt) * 10.0).round() / 10.0;
                            cn_hit_rate_pct = ((cn_sum / cnt) * 10.0).round() / 10.0;
                        }
                    }
                }
            }
        }

        repo.upsert_economist_index(&reports)?;
        let unacked_count = reports.iter().filter(|r| !r.acked).count();

        Ok(EconomistSnapshot {
            unacked_count,
            reports,
            backtest_hit_rate_pct,
            us_hit_rate_pct,
            cn_hit_rate_pct,
            bearish_risk_sectors_zh,
        })
    }

    pub fn set_report_ack(
        &self,
        repo: &SqliteRepository,
        report_id: &str,
        acked: bool,
    ) -> Result<EconomistSnapshot, String> {
        let ack_path = self.paths.data_dir().join("ack_state.json");
        let mut root: serde_json::Value = if ack_path.is_file() {
            std::fs::read_to_string(&ack_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({ "reports": {} }))
        } else {
            serde_json::json!({ "reports": {} })
        };

        let now_str = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        if !root.is_object() {
            root = serde_json::json!({ "reports": {} });
        }
        let root_obj = root.as_object_mut().unwrap();
        root_obj.insert(
            "updated_at".to_string(),
            serde_json::Value::String(now_str.clone()),
        );
        let reports_val = root_obj
            .entry("reports".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if !reports_val.is_object() {
            *reports_val = serde_json::json!({});
        }
        let reports_map = reports_val.as_object_mut().unwrap();
        let entry = reports_map
            .entry(report_id.to_string())
            .or_insert_with(|| serde_json::json!({ "report_id": report_id, "date": report_id }));
        if let Some(item_obj) = entry.as_object_mut() {
            item_obj.insert("acked".to_string(), serde_json::Value::Bool(acked));
            item_obj.insert(
                "acked_at".to_string(),
                if acked {
                    serde_json::Value::String(now_str)
                } else {
                    serde_json::Value::Null
                },
            );
        }

        let pretty = serde_json::to_string_pretty(&root)
            .map_err(|e| format!("序列化 ack_state.json 失败: {}", e))?;
        let tmp_path = self.paths.data_dir().join("ack_state.json.tmp");
        std::fs::write(&tmp_path, &pretty)
            .map_err(|e| format!("写入临时 ./data/ack_state.json.tmp 失败: {}", e))?;
        std::fs::rename(&tmp_path, &ack_path)
            .map_err(|e| format!("原子替换 ./data/ack_state.json 失败: {}", e))?;

        self.load_and_sync(repo)
    }

    pub fn read_report_html(&self, rel_path: &str) -> Result<String, String> {
        let raw = self
            .paths
            .resolve_guarded_report_path(rel_path)
            .and_then(|p| std::fs::read_to_string(p).map_err(|e| format!("读取研报HTML失败: {}", e)))?;
        let echarts_path = self.paths.reports_dir().join("assets").join("echarts.min.js");
        let echarts_js = std::fs::read_to_string(&echarts_path).ok();
        Ok(prepare_embedded_report_html(&raw, echarts_js.as_deref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn syncs_economist_reports_and_toggles_ack_in_isolated_sandbox() {
        let sandbox = PathBuf::from(".temp").join("test_economist_bridge");
        let _ = std::fs::remove_dir_all(&sandbox);
        let paths = PortablePaths::from_root(sandbox.clone()).unwrap();
        std::fs::create_dir_all(paths.data_dir().join("runs")).unwrap();
        std::fs::create_dir_all(paths.reports_dir().join("assets")).unwrap();
        std::fs::create_dir_all(paths.reports_dir().join("2026/11/18")).unwrap();
        std::fs::write(
            paths.reports_dir().join("assets/echarts.min.js"),
            "window.echarts={tag:\"</script>\"};",
        )
        .unwrap();
        std::fs::write(
            paths.reports_dir().join("2026/11/18/economist_report_2026-11-18.html"),
            "<!doctype html><html><head><script src=\"/reports/assets/echarts.min.js\"></script></head><body>Economist Report</body></html>",
        )
        .unwrap();
        std::fs::write(
            paths.data_dir().join("ack_state.json"),
            r#"{"updated_at":"2026-11-18 09:00:00","reports":{"2026-11-18":{"report_id":"2026-11-18","date":"2026-11-18","title":"深圳峰会与AI重估","rel_path":"2026/11/18/economist_report_2026-11-18.html","article_count":14,"core_count":11,"acked":false,"acked_at":null}}}"#,
        )
        .unwrap();
        std::fs::write(
            paths.data_dir().join("runs").join("analysis_2026-11-18.json"),
            r#"{"macro_synthesis":{"executive_summary":"跨市场摘要","us_market_overview":{"sentiment_score":55},"cn_market_overview":{"sentiment_score":68,"long_short_allocation":{"underweight":["高负债民企地产开发商（受制于新开工不足）"]}},"cross_market_chains":[{"chain_title":"主线一","economist_trigger":"触发","us_mapping":"AMZN","cn_mapping":"中际旭创 300308.SZ、金山办公 688111.SH","divergence_insight":"分化"}]}}"#,
        )
        .unwrap();

        let repo = SqliteRepository::new(&paths.sqlite_db_path()).unwrap();
        let bridge = EconomistBridge::new(paths);
        let snap = bridge.load_and_sync(&repo).unwrap();
        assert_eq!(snap.unacked_count, 1);
        assert_eq!(snap.reports.len(), 1);
        assert!(snap.reports[0].chains[0]
            .related_cn_symbols
            .contains(&"300308.SZ".to_string()));

        let after_ack = bridge.set_report_ack(&repo, "2026-11-18", true).unwrap();
        assert_eq!(after_ack.unacked_count, 0);
        assert!(after_ack.reports[0].acked);

        let embedded_html = bridge
            .read_report_html("./reports/2026/11/18/economist_report_2026-11-18.html")
            .unwrap();
        assert!(embedded_html.contains("data-embed=\"1\""));
        assert!(embedded_html.contains("data-inlined-echarts=\"1\""));
        assert!(embedded_html.contains("<\\/script>"));
        assert!(!embedded_html.contains("src=\"/reports/assets/echarts.min.js\""));
    }

    #[test]
    fn extract_cn_symbols_handles_multibyte_cjk_after_six_digits_and_dot() {
        let sample = "营收规模突破 100000.亿元，重点跟踪中际旭创 300308.SZ、金山办公 688111.sh，忽略伪造代码 123456.SZX 与 999999.中";
        let syms = extract_cn_symbols_from_text(sample);
        assert_eq!(syms, vec!["300308.SZ".to_string(), "688111.SH".to_string()]);
    }
}
