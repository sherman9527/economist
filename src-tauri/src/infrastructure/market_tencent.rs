use crate::domain::types::Symbol;

#[derive(Debug, Clone)]
pub struct RawTencentQuote {
    pub symbol: Symbol,
    pub name_zh: String,
    pub latest_yuan: f64,
    pub prev_close_yuan: f64,
    pub open_yuan: f64,
    pub high_yuan: f64,
    pub low_yuan: f64,
    pub volume_lots: f64,
    pub amount_wan: f64,
    pub turnover_pct: f64,
    pub circ_mv_yi: f64,
    pub total_mv_yi: f64,
    pub vol_ratio: f64,
    pub vwap_yuan: f64,
    pub is_suspended: bool,
    pub raw_ts: String,
}

#[derive(Debug, Clone)]
pub struct SimpleBar {
    pub time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub amount_yuan: f64,
    pub turnover_pct: f64,
}

pub fn generate_full_ashare_probe_codes() -> Vec<String> {
    let mut codes = Vec::with_capacity(10_500);
    for c in 600_000..=601_999 {
        codes.push(format!("sh{:06}", c));
    }
    for c in 603_000..=603_999 {
        codes.push(format!("sh{:06}", c));
    }
    for c in 605_000..=605_599 {
        codes.push(format!("sh{:06}", c));
    }
    for c in 688_001..=688_999 {
        codes.push(format!("sh{:06}", c));
    }
    codes.push("sh689009".to_string());

    for c in 1..=1_999 {
        codes.push(format!("sz{:06}", c));
    }
    for c in 2_001..=3_049 {
        codes.push(format!("sz{:06}", c));
    }
    codes.push("sz003816".to_string());
    for c in 300_001..=301_799 {
        codes.push(format!("sz{:06}", c));
    }

    for c in 920_000..=920_999 {
        codes.push(format!("bj{:06}", c));
    }
    codes
}

pub fn parse_tencent_quote_batch(decoded_text: &str) -> Vec<RawTencentQuote> {
    let mut out = Vec::new();
    for raw_stmt in decoded_text.split(';') {
        let stmt = raw_stmt.trim();
        if stmt.is_empty() || stmt.starts_with("v_pv_none_match") {
            continue;
        }
        let Some(eq_idx) = stmt.find('=') else {
            continue;
        };
        let header = stmt[..eq_idx].trim();
        let rhs = stmt[eq_idx + 1..].trim().trim_matches('"');
        if rhs.is_empty() || !rhs.contains('~') {
            continue;
        }
        let parts: Vec<&str> = rhs.split('~').collect();
        if parts.len() <= 45 {
            continue;
        }

        let name_zh = parts[1].trim();
        let digits = parts[2].trim();
        if name_zh.is_empty() || digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }

        let prefixed = if header.contains("sh") {
            format!("sh{}", digits)
        } else if header.contains("sz") {
            format!("sz{}", digits)
        } else if header.contains("bj") {
            format!("bj{}", digits)
        } else {
            digits.to_string()
        };
        let Ok(symbol) = Symbol::parse(&prefixed) else {
            continue;
        };

        let pf = |idx: usize| -> f64 {
            parts
                .get(idx)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(0.0)
        };

        let latest_yuan = pf(3);
        let prev_close_yuan = pf(4);
        let open_yuan = pf(5);
        let volume_lots = pf(6);
        let raw_ts = parts.get(30).copied().unwrap_or("").trim().to_string();
        let high_yuan = pf(33);
        let low_yuan = pf(34);
        let amount_wan = pf(37);
        let turnover_pct = pf(38);
        let status_flag = parts.get(40).copied().unwrap_or("").trim();
        let circ_mv_yi = pf(44);
        let total_mv_yi = pf(45);
        let vol_ratio = pf(49);
        let vwap_raw = pf(51);
        let vwap_yuan = if vwap_raw > 0.0 {
            vwap_raw
        } else if amount_wan > 0.0 && volume_lots > 0.0 {
            (amount_wan * 100.0) / volume_lots
        } else if latest_yuan > 0.0 {
            latest_yuan
        } else {
            prev_close_yuan
        };

        if latest_yuan <= 0.0 && prev_close_yuan <= 0.0 {
            continue;
        }
        if total_mv_yi <= 0.0 && circ_mv_yi <= 0.0 {
            continue;
        }

        let hhmm: u32 = if raw_ts.len() >= 12 {
            raw_ts[8..12].parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        let is_suspended = status_flag.starts_with('S')
            || status_flag.starts_with('D')
            || status_flag.contains('停')
            || (open_yuan <= 0.0 && volume_lots <= 0.0 && hhmm >= 931);

        out.push(RawTencentQuote {
            symbol,
            name_zh: name_zh.to_string(),
            latest_yuan: if latest_yuan > 0.0 {
                latest_yuan
            } else {
                prev_close_yuan
            },
            prev_close_yuan,
            open_yuan,
            high_yuan,
            low_yuan,
            volume_lots,
            amount_wan,
            turnover_pct,
            circ_mv_yi,
            total_mv_yi: if total_mv_yi > 0.0 {
                total_mv_yi
            } else {
                circ_mv_yi
            },
            vol_ratio,
            vwap_yuan,
            is_suspended,
            raw_ts,
        });
    }
    out
}

pub async fn fetch_tencent_full_universe(
    client: &reqwest::Client,
    batch_url: &str,
    known_codes: Option<&[String]>,
    chunk_size: usize,
    max_concurrency: usize,
) -> Result<Vec<RawTencentQuote>, String> {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    use tokio::task::JoinSet;

    let owned_codes;
    let codes_slice: &[String] = match known_codes {
        Some(slice) if !slice.is_empty() => slice,
        _ => {
            owned_codes = generate_full_ashare_probe_codes();
            &owned_codes
        }
    };

    let step = chunk_size.clamp(200, 650);
    let chunks: Vec<String> = codes_slice.chunks(step).map(|s| s.join(",")).collect();
    let base = batch_url.trim_end_matches('/').to_string();
    let sem = Arc::new(Semaphore::new(max_concurrency.clamp(2, 16)));
    let mut set = JoinSet::new();

    for joined in chunks {
        let c = client.clone();
        let b = base.clone();
        let sem_clone = sem.clone();
        set.spawn(async move {
            let _permit = sem_clone.acquire_owned().await;
            let sep = if b.ends_with("q=") {
                ""
            } else if b.contains('?') {
                "&q="
            } else {
                "/q="
            };
            let url = format!("{}{}{}", b, sep, joined);
            let resp = c
                .get(&url)
                .header("Referer", "https://gu.qq.com/")
                .send()
                .await
                .map_err(|e| format!("请求腾讯批量行情失败: {}", e))?;
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| format!("读取腾讯行情响应流失败: {}", e))?;
            let (decoded, _, _) = encoding_rs::GBK.decode(&bytes);
            Ok::<Vec<RawTencentQuote>, String>(parse_tencent_quote_batch(&decoded))
        });
    }

    let mut dedup: HashMap<String, RawTencentQuote> = HashMap::with_capacity(6_000);
    let mut last_err = None;
    while let Some(joined_res) = set.join_next().await {
        match joined_res {
            Ok(Ok(batch)) => {
                for q in batch {
                    dedup.insert(q.symbol.canonical(), q);
                }
            }
            Ok(Err(e)) => last_err = Some(e),
            Err(e) => last_err = Some(format!("并发任务异常: {}", e)),
        }
    }
    if dedup.is_empty() {
        return Err(last_err.unwrap_or_else(|| "腾讯全市场行情返回为空".to_string()));
    }
    let mut all: Vec<RawTencentQuote> = dedup.into_values().collect();
    all.sort_by(|a, b| a.symbol.digits.cmp(&b.symbol.digits));
    Ok(all)
}

pub async fn fetch_tencent_fallback_klines(
    client: &reqwest::Client,
    fqkline_url: &str,
    mkline_url: &str,
    symbol: &Symbol,
    period: &str,
    limit: usize,
    circ_mv_yi: f64,
) -> Result<Vec<SimpleBar>, String> {
    let tc = symbol.tencent_code();
    let url = if period == "30m" {
        let mk = mkline_url.replace("://web.ifzq.gtimg.cn/", "://ifzq.gtimg.cn/");
        format!("{}?param={},m30,,{}", mk, tc, limit.clamp(20, 240))
    } else {
        let fq = fqkline_url.replace("://ifzq.gtimg.cn/", "://web.ifzq.gtimg.cn/");
        format!(
            "{}?param={},day,,,{},qfq",
            fq,
            tc,
            limit.clamp(30, 240)
        )
    };

    let mut last_err = String::new();
    for attempt in 0..3usize {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(120 * (attempt as u64))).await;
        }
        let resp_res = client
            .get(&url)
            .header("Referer", "https://gu.qq.com/")
            .send()
            .await;
        let resp = match resp_res {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("请求腾讯备用K线失败: {}", e);
                continue;
            }
        };
        if !resp.status().is_success() {
            last_err = format!("腾讯备用K线HTTP状态异常 {} ({})", resp.status(), tc);
            continue;
        }
        let body = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                last_err = format!("读取腾讯备用K线失败: {}", e);
                continue;
            }
        };
        if body.trim().is_empty() {
            last_err = format!("腾讯备用K线返回空响应 ({})", tc);
            continue;
        }
        match parse_tencent_fallback_kline_json(&body, symbol, period, circ_mv_yi) {
            Ok(bars) if !bars.is_empty() => return Ok(bars),
            Ok(_) => last_err = format!("腾讯备用K线返回0根K线 ({})", tc),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

pub fn parse_tencent_fallback_kline_json(
    body: &str,
    symbol: &Symbol,
    period: &str,
    circ_mv_yi: f64,
) -> Result<Vec<SimpleBar>, String> {
    use crate::domain::types::BoardKind;

    let tc = symbol.tencent_code();
    let trimmed = body.trim();
    let json_slice = match (trimmed.find('{'), trimmed.rfind('}')) {
        (Some(s), Some(e)) if s < e => &trimmed[s..=e],
        _ => trimmed,
    };
    let root: serde_json::Value =
        serde_json::from_str(json_slice).map_err(|e| format!("解析腾讯备用K线JSON失败: {}", e))?;
    let stock_obj = root
        .get("data")
        .and_then(|d| d.get(&tc))
        .ok_or_else(|| format!("腾讯备用K线无标的节点 {}", tc))?;

    let arr = if period == "30m" {
        stock_obj.get("m30").and_then(|v| v.as_array())
    } else {
        stock_obj
            .get("qfqday")
            .or_else(|| stock_obj.get("day"))
            .and_then(|v| v.as_array())
    }
    .ok_or_else(|| "腾讯备用K线列表缺失".to_string())?;

    let pv = |v: &serde_json::Value| -> f64 {
        v.as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| v.as_f64())
            .unwrap_or(0.0)
    };

    let ref_latest_close = arr
        .iter()
        .rev()
        .find_map(|item| item.as_array().and_then(|r| r.get(2)).map(pv))
        .filter(|c| *c > 0.0)
        .unwrap_or(0.0);
    let est_float_shares = if circ_mv_yi > 0.0 && ref_latest_close > 0.0 {
        (circ_mv_yi * 100_000_000.0) / ref_latest_close
    } else {
        0.0
    };

    let mut bars = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(row) = item.as_array() else {
            continue;
        };
        if row.len() < 6 {
            continue;
        }
        let raw_t = row[0].as_str().unwrap_or("");
        let time = if period == "30m" && raw_t.len() == 12 {
            format!(
                "{}-{}-{} {}:{}",
                &raw_t[0..4],
                &raw_t[4..6],
                &raw_t[6..8],
                &raw_t[8..10],
                &raw_t[10..12]
            )
        } else {
            raw_t.to_string()
        };
        let open = pv(&row[1]);
        let close = pv(&row[2]);
        let high = pv(&row[3]);
        let low = pv(&row[4]);
        let raw_vol = pv(&row[5]);
        let explicit_amount_wan = row.get(8).map(pv).unwrap_or(0.0);

        let volume = if explicit_amount_wan > 0.0 && close > 0.0 && raw_vol > 0.0 {
            let implied_shares = (explicit_amount_wan * 10_000.0) / close;
            if (raw_vol - implied_shares).abs() < (raw_vol * 100.0 - implied_shares).abs() {
                raw_vol / 100.0
            } else {
                raw_vol
            }
        } else if matches!(symbol.board, BoardKind::Star | BoardKind::Bse) && period != "30m" {
            raw_vol / 100.0
        } else {
            raw_vol
        };

        let amount_yuan = if explicit_amount_wan > 0.0 {
            explicit_amount_wan * 10_000.0
        } else {
            volume * 100.0 * close
        };
        let turnover_pct = if est_float_shares > 0.0 {
            ((volume * 100.0) / est_float_shares) * 100.0
        } else if circ_mv_yi > 0.0 {
            (amount_yuan / (circ_mv_yi * 100_000_000.0)) * 100.0
        } else {
            0.0
        };
        bars.push(SimpleBar {
            time,
            open,
            high,
            low,
            close,
            volume,
            amount_yuan,
            turnover_pct,
        });
    }
    if bars.is_empty() {
        return Err("腾讯备用K线解析结果为空".to_string());
    }
    Ok(bars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gbk_decoded_tencent_quote_line_accurately() {
        let sample = "v_sz300308=\"51~中际旭创~300308~396.30~393.00~400.00~416368~216702~199667~396.30~38~396.29~4~396.28~13~396.27~21~396.26~2~396.32~12~396.35~2~396.40~1~396.44~1~396.46~2~~20261118161415~3.30~0.84~404.67~385.72~396.30/416368/16438620400~416368~1643862~3.75~18.02~~404.67~385.72~4.82~4398.68~4668.06~4.95~471.60~314.40~0.73~60~394.81~\";";
        let quotes = parse_tencent_quote_batch(sample);
        assert_eq!(quotes.len(), 1);
        let q = &quotes[0];
        assert_eq!(q.symbol.canonical(), "300308.SZ");
        assert_eq!(q.name_zh, "中际旭创");
        assert!((q.latest_yuan - 396.30).abs() < 1e-6);
        assert!((q.turnover_pct - 3.75).abs() < 1e-6);
        assert!((q.total_mv_yi - 4668.06).abs() < 1e-6);
        assert!((q.vwap_yuan - 394.81).abs() < 1e-6);
        assert!(!q.is_suspended);
    }
}
