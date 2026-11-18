use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PitfallDimension {
    InsiderReduction,
    NegativeNews,
    HolderSurge,
    MainOutflow,
    TechBreakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PitfallSeverity {
    Clear,
    Warn,
    Veto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitfallCheckItem {
    pub dimension: PitfallDimension,
    pub label_zh: String,
    pub passed: bool,
    pub severity: PitfallSeverity,
    pub detail_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitfallThresholds {
    pub insider_sell_veto_pct: f64,
    pub holder_surge_veto_pct: f64,
    pub holder_surge_warn_pct: f64,
    pub main_outflow_veto_ratio: f64,
    pub upper_shadow_veto_ratio: f64,
    pub regulatory_veto_keywords: Vec<String>,
}

impl Default for PitfallThresholds {
    fn default() -> Self {
        Self {
            insider_sell_veto_pct: 0.5,
            holder_surge_veto_pct: 25.0,
            holder_surge_warn_pct: 10.0,
            main_outflow_veto_ratio: -0.08,
            upper_shadow_veto_ratio: 0.55,
            regulatory_veto_keywords: vec![
                "立案".to_string(),
                "留置".to_string(),
                "行政处罚".to_string(),
                "退市风险".to_string(),
                "非标意见".to_string(),
                "违规担保".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct PitfallAuditInput {
    pub net_insider_reduction_90d_pct: f64,
    pub announcement_titles_60d: Vec<String>,
    pub industry_sw1: String,
    pub economist_bearish_sectors: Vec<String>,
    pub holder_num_change_qoq_pct: Option<f64>,
    pub holder_notice_date: Option<String>,
    pub main_net_inflow_ratio: f64,
    pub open_yuan: f64,
    pub high_yuan: f64,
    pub low_yuan: f64,
    pub close_yuan: f64,
    pub ma20_yuan: f64,
    pub pct_chg: f64,
}

#[derive(Debug, Clone)]
pub struct PitfallAuditReport {
    pub vetoed: bool,
    pub passed_count: usize,
    pub warn_penalty_pts: f64,
    pub items: Vec<PitfallCheckItem>,
}

pub fn audit_five_pitfalls(
    input: &PitfallAuditInput,
    thresholds: &PitfallThresholds,
) -> PitfallAuditReport {
    let mut items = Vec::with_capacity(5);
    let mut warn_penalty_pts = 0.0_f64;

    // 1. Insider net reduction in trailing 90 days
    if input.net_insider_reduction_90d_pct >= thresholds.insider_sell_veto_pct {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::InsiderReduction,
            label_zh: "大股东/高管减持".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!(
                "近90日净减持占流通盘 {:.2}% >= {:.2}% 否决线",
                input.net_insider_reduction_90d_pct, thresholds.insider_sell_veto_pct
            ),
        });
    } else {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::InsiderReduction,
            label_zh: "大股东/高管减持".to_string(),
            passed: true,
            severity: PitfallSeverity::Clear,
            detail_zh: format!(
                "近90日净减持 {:.2}%，无重要股东集中派发",
                input.net_insider_reduction_90d_pct.max(0.0)
            ),
        });
    }

    // 2. Regulatory risk announcements & Economist bearish sector mapping
    let hit_kw = input.announcement_titles_60d.iter().find_map(|title| {
        thresholds
            .regulatory_veto_keywords
            .iter()
            .find(|kw| title.contains(kw.as_str()))
            .map(|kw| (kw.clone(), title.clone()))
    });
    let hit_bearish = if !input.industry_sw1.trim().is_empty() {
        input
            .economist_bearish_sectors
            .iter()
            .find(|sec| {
                !sec.trim().is_empty()
                    && (sec.contains(&input.industry_sw1) || input.industry_sw1.contains(sec.as_str()))
            })
            .cloned()
    } else {
        None
    };

    if let Some((kw, title)) = hit_kw {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::NegativeNews,
            label_zh: "监管公告与宏观舆情".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!("近60日公告命中监管红线词「{}」: {}", kw, title),
        });
    } else if let Some(sec) = hit_bearish {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::NegativeNews,
            label_zh: "监管公告与宏观舆情".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!("命中《经济学人》跨市场强利空行业警示: {}", sec),
        });
    } else {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::NegativeNews,
            label_zh: "监管公告与宏观舆情".to_string(),
            passed: true,
            severity: PitfallSeverity::Clear,
            detail_zh: "近60日无立案/处罚/非标公告，未命中宏观利空行业".to_string(),
        });
    }

    // 3. Shareholder count QoQ surge (strictly PIT via HOLD_NOTICE_DATE)
    let notice_tag = input
        .holder_notice_date
        .as_deref()
        .unwrap_or("最新HOLD_NOTICE_DATE");
    match input.holder_num_change_qoq_pct {
        Some(qoq) if qoq > thresholds.holder_surge_veto_pct => {
            items.push(PitfallCheckItem {
                dimension: PitfallDimension::HolderSurge,
                label_zh: "股东户数筹码发散".to_string(),
                passed: false,
                severity: PitfallSeverity::Veto,
                detail_zh: format!(
                    "披露日({})股东户数环比激增 +{:.1}% > +{:.1}%，筹码显著发散",
                    notice_tag, qoq, thresholds.holder_surge_veto_pct
                ),
            });
        }
        Some(qoq) if qoq > thresholds.holder_surge_warn_pct => {
            warn_penalty_pts += 6.0;
            items.push(PitfallCheckItem {
                dimension: PitfallDimension::HolderSurge,
                label_zh: "股东户数筹码发散".to_string(),
                passed: true,
                severity: PitfallSeverity::Warn,
                detail_zh: format!(
                    "披露日({})股东户数环比 +{:.1}%，轻度发散扣减综合得分",
                    notice_tag, qoq
                ),
            });
        }
        Some(qoq) => {
            items.push(PitfallCheckItem {
                dimension: PitfallDimension::HolderSurge,
                label_zh: "股东户数筹码发散".to_string(),
                passed: true,
                severity: PitfallSeverity::Clear,
                detail_zh: format!("披露日({})股东户数环比 {:.1}%，筹码结构稳定", notice_tag, qoq),
            });
        }
        None => {
            items.push(PitfallCheckItem {
                dimension: PitfallDimension::HolderSurge,
                label_zh: "股东户数筹码发散".to_string(),
                passed: true,
                severity: PitfallSeverity::Clear,
                detail_zh: "最新披露期股东户数无异常激增记录".to_string(),
            });
        }
    }

    // 4. Main capital net outflow ratio
    if input.main_net_inflow_ratio < thresholds.main_outflow_veto_ratio {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::MainOutflow,
            label_zh: "主力资金净流向".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!(
                "主力净流出占成交额 {:.1}% < {:.1}%，呈现高换手拉高派发特征",
                input.main_net_inflow_ratio * 100.0,
                thresholds.main_outflow_veto_ratio * 100.0
            ),
        });
    } else if input.main_net_inflow_ratio < -0.03 {
        warn_penalty_pts += 4.0;
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::MainOutflow,
            label_zh: "主力资金净流向".to_string(),
            passed: true,
            severity: PitfallSeverity::Warn,
            detail_zh: format!(
                "主力资金小幅净流出 {:.1}%，需观察次日承接",
                input.main_net_inflow_ratio * 100.0
            ),
        });
    } else {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::MainOutflow,
            label_zh: "主力资金净流向".to_string(),
            passed: true,
            severity: PitfallSeverity::Clear,
            detail_zh: format!(
                "主力净流入占成交额 {:+.1}%，量价资金同向共振",
                input.main_net_inflow_ratio * 100.0
            ),
        });
    }

    // 5. Technical breakdown: below MA20 or upper-shadow bull trap
    let bar_range = (input.high_yuan - input.low_yuan).max(0.0);
    let body_top = input.open_yuan.max(input.close_yuan);
    let upper_shadow_ratio = if bar_range > 1e-6 {
        (input.high_yuan - body_top).max(0.0) / bar_range
    } else {
        0.0
    };
    let below_ma20 = input.ma20_yuan > 0.0 && input.close_yuan < input.ma20_yuan;
    let upper_shadow_trap =
        upper_shadow_ratio > thresholds.upper_shadow_veto_ratio && input.pct_chg < 3.0;

    if below_ma20 {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::TechBreakdown,
            label_zh: "MA20与上影线形态".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!(
                "现价 {:.2} 跌破20日均线 {:.2}，右侧趋势破位否决",
                input.close_yuan, input.ma20_yuan
            ),
        });
    } else if upper_shadow_trap {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::TechBreakdown,
            label_zh: "MA20与上影线形态".to_string(),
            passed: false,
            severity: PitfallSeverity::Veto,
            detail_zh: format!(
                "上影线占比 {:.0}% > {:.0}% 且涨幅滞涨({:+.2}%)，疑似冲高诱多",
                upper_shadow_ratio * 100.0,
                thresholds.upper_shadow_veto_ratio * 100.0,
                input.pct_chg
            ),
        });
    } else {
        items.push(PitfallCheckItem {
            dimension: PitfallDimension::TechBreakdown,
            label_zh: "MA20与上影线形态".to_string(),
            passed: true,
            severity: PitfallSeverity::Clear,
            detail_zh: format!(
                "现价 {:.2} 稳站MA20({:.2})上方，K线实体饱满(上影占比{:.0}%)",
                input.close_yuan,
                input.ma20_yuan,
                upper_shadow_ratio * 100.0
            ),
        });
    }

    let vetoed = items.iter().any(|x| x.severity == PitfallSeverity::Veto);
    let passed_count = items.iter().filter(|x| x.passed).count();

    PitfallAuditReport {
        vetoed,
        passed_count,
        warn_penalty_pts,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_input() -> PitfallAuditInput {
        PitfallAuditInput {
            net_insider_reduction_90d_pct: 0.0,
            announcement_titles_60d: vec!["关于获得发明专利证书的公告".to_string()],
            industry_sw1: "光模块与通信设备".to_string(),
            economist_bearish_sectors: vec!["传统民企地产".to_string()],
            holder_num_change_qoq_pct: Some(-4.2),
            holder_notice_date: Some("2026-10-28".to_string()),
            main_net_inflow_ratio: 0.065,
            open_yuan: 18.20,
            high_yuan: 19.10,
            low_yuan: 18.05,
            close_yuan: 18.95,
            ma20_yuan: 17.40,
            pct_chg: 4.12,
        }
    }

    #[test]
    fn passes_all_five_dimensions_when_clean() {
        let report = audit_five_pitfalls(&clean_input(), &PitfallThresholds::default());
        assert!(!report.vetoed);
        assert_eq!(report.passed_count, 5);
        assert_eq!(report.items.len(), 5);
    }

    #[test]
    fn vetoes_insider_regulatory_holder_outflow_and_tech_mines() {
        let cfg = PitfallThresholds::default();

        let mut insider = clean_input();
        insider.net_insider_reduction_90d_pct = 0.85;
        assert!(audit_five_pitfalls(&insider, &cfg).vetoed);

        let mut reg = clean_input();
        reg.announcement_titles_60d.push("关于收到中国证监会立案告知书的公告".to_string());
        assert!(audit_five_pitfalls(&reg, &cfg).vetoed);

        let mut holder = clean_input();
        holder.holder_num_change_qoq_pct = Some(31.5);
        assert!(audit_five_pitfalls(&holder, &cfg).vetoed);

        let mut outflow = clean_input();
        outflow.main_net_inflow_ratio = -0.11;
        assert!(audit_five_pitfalls(&outflow, &cfg).vetoed);

        let mut breakdown = clean_input();
        breakdown.close_yuan = 16.90; // < ma20 17.40
        assert!(audit_five_pitfalls(&breakdown, &cfg).vetoed);
    }
}
