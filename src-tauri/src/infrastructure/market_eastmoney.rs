use crate::domain::types::Symbol;
use crate::infrastructure::market_tencent::SimpleBar;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ClistEnrichmentMeta {
    pub industry_sw1: String,
    pub listing_days: u32,
    pub main_net_inflow_wan: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PitfallExternalSignals {
    pub net_insider_reduction_90d_pct: f64,
    pub restricted_unlock_30d_pct: f64,
    pub holder_num_change_qoq_pct: Option<f64>,
    pub holder_notice_date: Option<String>,
    pub announcement_titles_60d: Vec<String>,
}

pub fn parse_eastmoney_kline_json(payload: &str) -> Result<Vec<SimpleBar>, String> {
    let root: serde_json::Value =
        serde_json::from_str(payload).map_err(|e| format!("解析东财K线JSON失败: {}", e))?;
    let klines = root
        .get("data")
        .and_then(|d| d.get("klines"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| "东财K线响应缺少 data.klines".to_string())?;

    let mut out = Vec::with_capacity(klines.len());
    for item in klines {
        let Some(csv) = item.as_str() else {
            continue;
        };
        let cols: Vec<&str> = csv.split(',').collect();
        if cols.len() < 11 {
            continue;
        }
        let pf = |idx: usize| cols[idx].trim().parse::<f64>().unwrap_or(0.0);
        out.push(SimpleBar {
            time: cols[0].trim().to_string(),
            open: pf(1),
            close: pf(2),
            high: pf(3),
            low: pf(4),
            volume: pf(5),
            amount_yuan: pf(6),
            turnover_pct: pf(10),
        });
    }
    if out.is_empty() {
        return Err("东财K线数据为空".to_string());
    }
    Ok(out)
}

pub fn parse_holder_notice_json(payload: &str) -> (Option<f64>, Option<String>) {
    parse_holder_notice_json_as_of(payload, None)
}

pub fn parse_holder_notice_json_as_of(
    payload: &str,
    as_of_date: Option<&str>,
) -> (Option<f64>, Option<String>) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(payload) else {
        return (None, None);
    };
    let Some(rows) = root
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
    else {
        return (None, None);
    };
    let pit_rows: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|row| {
            let dt = row
                .get("HOLD_NOTICE_DATE")
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(10).collect::<String>())
                .unwrap_or_default();
            if dt.len() < 10 {
                return false;
            }
            match as_of_date {
                Some(cutoff) if cutoff.len() >= 10 => dt.as_str() <= &cutoff[..10],
                _ => true,
            }
        })
        .collect();

    for (idx, row) in pit_rows.iter().enumerate() {
        let notice_dt = row
            .get("HOLD_NOTICE_DATE")
            .and_then(|v| v.as_str())
            .map(|s| s.chars().take(10).collect::<String>())
            .filter(|s| !s.is_empty());
        if let Some(ratio) = row
            .get("HOLDER_NUM_RATIO")
            .and_then(|v| v.as_f64())
            .filter(|x| x.is_finite())
        {
            return (Some(ratio), notice_dt);
        }
        if let (Some(curr_num), Some(prev_num)) = (
            row.get("HOLDER_NUM").and_then(|v| v.as_f64()),
            pit_rows
                .get(idx + 1)
                .and_then(|p| p.get("HOLDER_NUM"))
                .and_then(|v| v.as_f64()),
        ) {
            if prev_num > 0.0 && curr_num > 0.0 {
                let computed = ((curr_num - prev_num) / prev_num) * 100.0;
                return (Some(computed), notice_dt);
            }
        }
    }
    (None, None)
}

pub fn derive_ulist_url(clist_url: &str) -> String {
    if let Some(prefix) = clist_url.strip_suffix("/clist/get") {
        format!("{}/ulist.np/get", prefix)
    } else {
        "https://push2.eastmoney.com/api/qt/ulist.np/get".to_string()
    }
}

pub fn parse_clist_or_ulist_meta_json(
    payload: &str,
    today: chrono::NaiveDate,
) -> Vec<(String, ClistEnrichmentMeta)> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(payload) else {
        return Vec::new();
    };
    let Some(diff) = root
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
    else {
        return Vec::new();
    };
    let mut page_out = Vec::with_capacity(diff.len());
    for row in diff {
        let Some(digits) = row.get("f12").and_then(|v| v.as_str()) else {
            continue;
        };
        let industry_sw1 = row
            .get("f100")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "-")
            .unwrap_or("通用制造")
            .to_string();
        let f62_yuan = row.get("f62").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let listing_days = row
            .get("f26")
            .and_then(|v| {
                v.as_i64()
                    .map(|n| n.to_string())
                    .or_else(|| v.as_str().map(|s| s.to_string()))
            })
            .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y%m%d").ok())
            .map(|dt| (((today - dt).num_days().max(1) * 5) / 7).max(1) as u32)
            .unwrap_or(250);

        page_out.push((
            digits.to_string(),
            ClistEnrichmentMeta {
                industry_sw1,
                listing_days,
                main_net_inflow_wan: f62_yuan / 10_000.0,
            },
        ));
    }
    page_out
}

pub async fn fetch_missing_ulist_meta(
    client: &reqwest::Client,
    clist_url: &str,
    symbols: &[Symbol],
) -> HashMap<String, ClistEnrichmentMeta> {
    if symbols.is_empty() {
        return HashMap::new();
    }
    let cst = chrono::FixedOffset::east_opt(8 * 3600).expect("valid CST");
    let today = chrono::Utc::now().with_timezone(&cst).date_naive();
    let ulist_url = derive_ulist_url(clist_url);
    let mut out = HashMap::with_capacity(symbols.len());

    for chunk in symbols.chunks(60) {
        let secids: Vec<String> = chunk.iter().map(|s| s.eastmoney_secid()).collect();
        let u = format!(
            "{}?fltt=2&invt=2&fields=f12,f14,f26,f62,f100&secids={}",
            ulist_url,
            secids.join(",")
        );
        let Ok(resp) = client
            .get(&u)
            .header("Referer", "https://quote.eastmoney.com/")
            .send()
            .await
        else {
            continue;
        };
        let Ok(txt) = resp.text().await else {
            continue;
        };
        for (digits, meta) in parse_clist_or_ulist_meta_json(&txt, today) {
            out.insert(digits, meta);
        }
    }
    out
}

pub async fn fetch_top_turnover_clist_meta(
    client: &reqwest::Client,
    clist_url: &str,
    pages: usize,
) -> HashMap<String, ClistEnrichmentMeta> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    let cst = chrono::FixedOffset::east_opt(8 * 3600).expect("valid CST");
    let today = chrono::Utc::now().with_timezone(&cst).date_naive();
    let sem = Arc::new(Semaphore::new(6));
    let mut set = JoinSet::new();

    for pn in 1..=pages.clamp(1, 10) {
        let c = client.clone();
        let u = format!(
            "{}?pn={}&pz=100&po=1&np=1&fltt=2&invt=2&fid=f8&fs=m:0+t:6,m:0+t:80,m:1+t:2,m:1+t:23,m:0+t:81+s:2048&fields=f12,f14,f26,f62,f100",
            clist_url, pn
        );
        let sem_clone = sem.clone();
        set.spawn(async move {
            let _permit = sem_clone.acquire_owned().await;
            let Ok(resp) = c
                .get(&u)
                .header("Referer", "https://quote.eastmoney.com/")
                .send()
                .await
            else {
                return Vec::new();
            };
            let Ok(txt) = resp.text().await else {
                return Vec::new();
            };
            parse_clist_or_ulist_meta_json(&txt, today)
        });
    }

    let mut map = HashMap::with_capacity(pages * 100);
    while let Some(res) = set.join_next().await {
        if let Ok(page) = res {
            for (k, v) in page {
                map.insert(k, v);
            }
        }
    }
    map
}

pub async fn fetch_eastmoney_klines(
    client: &reqwest::Client,
    kline_url: &str,
    symbol: &Symbol,
    period: &str,
    limit: usize,
) -> Result<Vec<SimpleBar>, String> {
    let klt = if period == "30m" { "30" } else { "101" };
    let url = format!(
        "{}?secid={}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt={}&fqt=1&end=20500101&lmt={}",
        kline_url,
        symbol.eastmoney_secid(),
        klt,
        limit.clamp(20, 240)
    );
    let body = client
        .get(&url)
        .header("Referer", "https://quote.eastmoney.com/")
        .send()
        .await
        .map_err(|e| format!("请求东财K线失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取东财K线响应失败: {}", e))?;
    parse_eastmoney_kline_json(&body)
}

pub async fn fetch_candidate_pitfall_signals(
    client: &reqwest::Client,
    datacenter_url: &str,
    notice_url: &str,
    symbol: &Symbol,
) -> PitfallExternalSignals {
    let cst = chrono::FixedOffset::east_opt(8 * 3600).expect("valid CST");
    let as_of = chrono::Utc::now()
        .with_timezone(&cst)
        .format("%Y-%m-%d")
        .to_string();
    fetch_candidate_pitfall_signals_as_of(client, datacenter_url, notice_url, symbol, &as_of).await
}

pub async fn fetch_candidate_pitfall_signals_as_of(
    client: &reqwest::Client,
    datacenter_url: &str,
    notice_url: &str,
    symbol: &Symbol,
    as_of_date: &str,
) -> PitfallExternalSignals {
    let digits = &symbol.digits;
    let as_of_naive = chrono::NaiveDate::parse_from_str(
        &as_of_date.chars().take(10).collect::<String>(),
        "%Y-%m-%d",
    )
    .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    let as_of_str = as_of_naive.format("%Y-%m-%d").to_string();
    let cutoff_90d = (as_of_naive - chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    let cutoff_60d = (as_of_naive - chrono::Duration::days(60))
        .format("%Y-%m-%d")
        .to_string();
    let horizon_30d = (as_of_naive + chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();

    let holder_url = format!(
        "{}?reportName=RPT_HOLDERNUM_DET&columns=SECURITY_CODE,HOLD_NOTICE_DATE,END_DATE,HOLDER_NUM,HOLDER_NUM_RATIO&filter=(SECURITY_CODE%3D%22{}%22)&pageNumber=1&pageSize=6&sortTypes=-1&sortColumns=HOLD_NOTICE_DATE&source=WEB&client=WEB",
        datacenter_url, digits
    );
    let insider_url = format!(
        "{}?reportName=RPT_SHARE_HOLDER_INCREASE&columns=SECURITY_CODE,NOTICE_DATE,HOLDER_NAME,DIRECTION,CHANGE_FREE_RATIO&filter=(SECURITY_CODE%3D%22{}%22)&pageNumber=1&pageSize=50&sortTypes=-1&sortColumns=NOTICE_DATE&source=WEB&client=WEB",
        datacenter_url, digits
    );
    let unlock_url = format!(
        "{}?reportName=RPT_LIFT_STAGE&columns=SECURITY_CODE,LIFT_DATE,FREE_SHARES_RATIO&filter=(SECURITY_CODE%3D%22{}%22)&pageNumber=1&pageSize=20&sortTypes=-1&sortColumns=LIFT_DATE&source=WEB&client=WEB",
        datacenter_url, digits
    );
    let ann_url = format!(
        "{}?sr=-1&page_size=50&page_index=1&ann_type=A&stock_list={}",
        notice_url, digits
    );

    let (h_res, i_res, u_res, a_res) = tokio::join!(
        client.get(&holder_url).send(),
        client.get(&insider_url).send(),
        client.get(&unlock_url).send(),
        client.get(&ann_url).send()
    );

    let mut signals = PitfallExternalSignals::default();

    if let Ok(resp) = h_res {
        if let Ok(txt) = resp.text().await {
            let (ratio, dt) = parse_holder_notice_json_as_of(&txt, Some(&as_of_str));
            signals.holder_num_change_qoq_pct = ratio;
            signals.holder_notice_date = dt;
        }
    }

    if let Ok(resp) = i_res {
        if let Ok(txt) = resp.text().await {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(arr) = root
                    .get("result")
                    .and_then(|r| r.get("data"))
                    .and_then(|d| d.as_array())
                {
                    let mut net_reduction_pct = 0.0_f64;
                    for row in arr {
                        let dir = row.get("DIRECTION").and_then(|v| v.as_str()).unwrap_or("");
                        let nd: String = row
                            .get("NOTICE_DATE")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(10)
                            .collect();
                        if nd.as_str() >= cutoff_90d.as_str() && nd.as_str() <= as_of_str.as_str() {
                            let pct = row
                                .get("CHANGE_FREE_RATIO")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0)
                                .abs();
                            if dir.contains("减持") {
                                net_reduction_pct += pct;
                            } else if dir.contains("增持") {
                                net_reduction_pct -= pct;
                            }
                        }
                    }
                    signals.net_insider_reduction_90d_pct = net_reduction_pct.max(0.0);
                }
            }
        }
    }

    if let Ok(resp) = u_res {
        if let Ok(txt) = resp.text().await {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(arr) = root
                    .get("result")
                    .and_then(|r| r.get("data"))
                    .and_then(|d| d.as_array())
                {
                    let mut unlock_sum = 0.0_f64;
                    for row in arr {
                        let ld: String = row
                            .get("LIFT_DATE")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(10)
                            .collect();
                        if ld.as_str() >= as_of_str.as_str() && ld.as_str() <= horizon_30d.as_str()
                        {
                            unlock_sum += row
                                .get("FREE_SHARES_RATIO")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0)
                                .max(0.0);
                        }
                    }
                    signals.restricted_unlock_30d_pct = unlock_sum;
                }
            }
        }
    }

    if let Ok(resp) = a_res {
        if let Ok(txt) = resp.text().await {
            if let Ok(root) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(arr) = root
                    .get("data")
                    .and_then(|d| d.get("list"))
                    .and_then(|l| l.as_array())
                {
                    for item in arr {
                        let nd: String = item
                            .get("notice_date")
                            .or_else(|| item.get("display_time"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(10)
                            .collect();
                        let in_window = nd.is_empty()
                            || (nd.as_str() >= cutoff_60d.as_str()
                                && nd.as_str() <= as_of_str.as_str());
                        if in_window {
                            if let Some(title) = item.get("title").and_then(|v| v.as_str()) {
                                signals.announcement_titles_60d.push(title.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eastmoney_f61_turnover_klines_and_hold_notice_date() {
        let kline_raw = r#"{"rc":0,"data":{"code":"300308","klines":["2026-11-17,532.74,508.90,567.89,506.39,626967,33622447403.00,11.35,-6.11,-33.10,5.65","2026-11-18,510.00,525.00,530.00,505.00,710000,36800000000.00,4.90,3.16,16.10,6.42"]}}"#;
        let bars = parse_eastmoney_kline_json(kline_raw).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].time, "2026-11-17");
        assert!((bars[0].turnover_pct - 5.65).abs() < 1e-6);
        assert!((bars[1].turnover_pct - 6.42).abs() < 1e-6);

        let holder_raw = r#"{"result":{"data":[{"SECURITY_CODE":"300308","HOLD_NOTICE_DATE":"2026-10-28 00:00:00","HOLDER_NUM_RATIO":18.4}]}}"#;
        let (ratio, dt) = parse_holder_notice_json(holder_raw);
        assert!((ratio.unwrap() - 18.4).abs() < 1e-6);
        assert_eq!(dt.as_deref(), Some("2026-10-28"));

        assert_eq!(
            derive_ulist_url("https://push2.eastmoney.com/api/qt/clist/get"),
            "https://push2.eastmoney.com/api/qt/ulist.np/get"
        );
        let ulist_raw = r#"{"rc":0,"data":{"total":1,"diff":[{"f12":"600103","f14":"青山纸业","f26":19970703,"f62":-102873094.0,"f100":"造纸"}]}}"#;
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let parsed = parse_clist_or_ulist_meta_json(ulist_raw, today);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "600103");
        assert_eq!(parsed[0].1.industry_sw1, "造纸");
        assert!((parsed[0].1.main_net_inflow_wan - (-10287.3094)).abs() < 1e-3);
    }
}
