use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Statutory SSE/SZSE weekday closures (2025–2027 mainland China exchange calendar).
const CN_EXCHANGE_HOLIDAYS: &[&str] = &[
    // 2025
    "2025-01-01",
    "2025-01-28", "2025-01-29", "2025-01-30", "2025-01-31", "2025-02-03", "2025-02-04",
    "2025-04-04",
    "2025-05-01", "2025-05-02", "2025-05-05",
    "2025-06-02",
    "2025-10-01", "2025-10-02", "2025-10-03", "2025-10-06", "2025-10-07", "2025-10-08",
    // 2026
    "2026-01-01", "2026-01-02",
    "2026-02-16", "2026-02-17", "2026-02-18", "2026-02-19", "2026-02-20", "2026-02-23",
    "2026-04-06",
    "2026-05-01", "2026-05-04", "2026-05-05",
    "2026-06-19",
    "2026-09-25",
    "2026-10-01", "2026-10-02", "2026-10-05", "2026-10-06", "2026-10-07",
    // 2027
    "2027-01-01",
    "2027-02-05", "2027-02-08", "2027-02-09", "2027-02-10", "2027-02-11", "2027-02-12",
    "2027-04-05",
    "2027-05-03", "2027-05-04", "2027-05-05",
    "2027-06-09",
    "2027-09-15",
    "2027-10-01", "2027-10-04", "2027-10-05", "2027-10-06", "2027-10-07",
];

pub fn is_ashare_trading_day(date: NaiveDate) -> bool {
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    let s = date.format("%Y-%m-%d").to_string();
    !CN_EXCHANGE_HOLIDAYS.contains(&s.as_str())
}

pub fn is_ashare_trading_day_str(date_str: &str) -> bool {
    let prefix: String = date_str.chars().take(10).collect();
    match NaiveDate::parse_from_str(&prefix, "%Y-%m-%d") {
        Ok(d) => is_ashare_trading_day(d),
        Err(_) => true,
    }
}

pub fn next_ashare_trading_date(from_date: NaiveDate) -> NaiveDate {
    let mut d = from_date + Duration::days(1);
    for _ in 0..30 {
        if is_ashare_trading_day(d) {
            return d;
        }
        d += Duration::days(1);
    }
    from_date + Duration::days(1)
}

pub fn next_ashare_trading_date_str(date_str: &str) -> String {
    let prefix: String = date_str.chars().take(10).collect();
    match NaiveDate::parse_from_str(&prefix, "%Y-%m-%d") {
        Ok(d) => next_ashare_trading_date(d).format("%Y-%m-%d").to_string(),
        Err(_) => date_str.to_string(),
    }
}

pub fn effective_ashare_trade_date(date: NaiveDate) -> NaiveDate {
    let mut d = date;
    for _ in 0..30 {
        if is_ashare_trading_day(d) {
            return d;
        }
        d -= Duration::days(1);
    }
    date
}

/// Maps *The Economist* cross-market bearish prose themes to canonical EastMoney `f100` / SW1 labels.
pub fn matches_economist_bearish_sector(
    industry_sw1: &str,
    bearish_sectors: &[String],
) -> Option<String> {
    let ind = industry_sw1.trim();
    if ind.is_empty() {
        return None;
    }
    const PROSE_TO_SW1: &[(&[&str], &[&str])] = &[
        (
            &["地产开发", "房企", "房地产", "住宅地产"],
            &["房地产开发", "房地产服务"],
        ),
        (
            &["建筑建材", "水泥", "传统基建"],
            &["工程建设", "水泥建材", "装修建材", "装修装饰"],
        ),
        (&["光伏产能", "光伏组件", "硅料过剩"], &["光伏设备"]),
        (&["传统车企", "燃油车"], &["乘用车", "商用车"]),
        (&["煤炭", "传统火电"], &["煤炭行业", "煤炭开采", "电力行业"]),
        (&["白酒", "高端白酒"], &["酿酒行业", "白酒Ⅱ"]),
        (&["人力外包", "IT外包"], &["IT服务Ⅱ"]),
    ];

    for raw_sec in bearish_sectors {
        let sec = raw_sec.trim();
        if sec.is_empty() {
            continue;
        }
        if sec.len() <= 24 && (sec == ind || (ind.len() >= 3 && sec.contains(ind))) {
            return Some(sec.to_string());
        }
        for (prose_keys, sw1_targets) in PROSE_TO_SW1 {
            if sw1_targets.iter().any(|t| *t == ind || ind.contains(t))
                && prose_keys.iter().any(|k| sec.contains(k))
            {
                return Some(sec.to_string());
            }
        }
    }
    None
}
