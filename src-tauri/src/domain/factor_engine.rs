use crate::domain::types::Fen;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    Literal(f64),
    Var(String),
    UnaryNeg(Box<ExprNode>),
    Binary {
        op: BinaryOp,
        lhs: Box<ExprNode>,
        rhs: Box<ExprNode>,
    },
    Call {
        func: String,
        args: Vec<ExprNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
}

fn tokenize_expr(expr: &str) -> Result<Vec<Token>, String> {
    let bytes = expr.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => {
                out.push(Token::Plus);
                i += 1;
            }
            '-' => {
                out.push(Token::Minus);
                i += 1;
            }
            '*' => {
                out.push(Token::Star);
                i += 1;
            }
            '/' => {
                out.push(Token::Slash);
                i += 1;
            }
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '0'..='9' | '.' => {
                let start = i;
                i += 1;
                while i < bytes.len() && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let s = &expr[start..i];
                let val = s
                    .parse::<f64>()
                    .map_err(|_| format!("无效数值字面量: {}", s))?;
                out.push(Token::Num(val));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_')
                {
                    i += 1;
                }
                out.push(Token::Ident(expr[start..i].to_string()));
            }
            _ => return Err(format!("因子表达式含非法字符: '{}'", c)),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn parse_add_sub(&mut self) -> Result<ExprNode, String> {
        let mut node = self.parse_mul_div()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.next();
            let rhs = self.parse_mul_div()?;
            node = ExprNode::Binary {
                op,
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
        }
        Ok(node)
    }

    fn parse_mul_div(&mut self) -> Result<ExprNode, String> {
        let mut node = self.parse_unary()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };
            self.next();
            let rhs = self.parse_unary()?;
            node = ExprNode::Binary {
                op,
                lhs: Box::new(node),
                rhs: Box::new(rhs),
            };
        }
        Ok(node)
    }

    fn parse_unary(&mut self) -> Result<ExprNode, String> {
        match self.peek() {
            Some(Token::Minus) => {
                self.next();
                Ok(ExprNode::UnaryNeg(Box::new(self.parse_unary()?)))
            }
            Some(Token::Plus) => {
                self.next();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<ExprNode, String> {
        match self.next() {
            Some(Token::Num(v)) => Ok(ExprNode::Literal(v)),
            Some(Token::LParen) => {
                let inner = self.parse_add_sub()?;
                match self.next() {
                    Some(Token::RParen) => Ok(inner),
                    _ => Err("因子表达式缺少右括号 ')'".to_string()),
                }
            }
            Some(Token::Ident(name)) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();
                    let allowed = ["abs", "ln", "sqrt", "min", "max", "clip", "sign", "pow"];
                    if !allowed.contains(&name.as_str()) {
                        return Err(format!("不允许的因子函数: {}", name));
                    }
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_add_sub()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    match self.next() {
                        Some(Token::RParen) => Ok(ExprNode::Call { func: name, args }),
                        _ => Err(format!("函数 {} 调用缺少 ')'", name)),
                    }
                } else {
                    Ok(ExprNode::Var(name))
                }
            }
            other => Err(format!("因子表达式语法错误: {:?}", other)),
        }
    }
}

pub fn parse_factor_expr(expr: &str) -> Result<ExprNode, String> {
    let tokens = tokenize_expr(expr)?;
    if tokens.is_empty() {
        return Err("因子表达式为空".to_string());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let root = parser.parse_add_sub()?;
    if parser.pos != parser.tokens.len() {
        return Err("因子表达式尾部存在多余符号".to_string());
    }
    Ok(root)
}

pub fn eval_factor_ast(node: &ExprNode, vars: &BTreeMap<String, f64>) -> f64 {
    let raw = match node {
        ExprNode::Literal(v) => *v,
        ExprNode::Var(name) => vars.get(name).copied().unwrap_or(0.0),
        ExprNode::UnaryNeg(inner) => -eval_factor_ast(inner, vars),
        ExprNode::Binary { op, lhs, rhs } => {
            let l = eval_factor_ast(lhs, vars);
            let r = eval_factor_ast(rhs, vars);
            match op {
                BinaryOp::Add => l + r,
                BinaryOp::Sub => l - r,
                BinaryOp::Mul => l * r,
                BinaryOp::Div => {
                    if r.abs() < 1e-9 {
                        0.0
                    } else {
                        l / r
                    }
                }
            }
        }
        ExprNode::Call { func, args } => {
            let a0 = args.first().map(|n| eval_factor_ast(n, vars)).unwrap_or(0.0);
            let a1 = args.get(1).map(|n| eval_factor_ast(n, vars)).unwrap_or(0.0);
            let a2 = args.get(2).map(|n| eval_factor_ast(n, vars)).unwrap_or(0.0);
            match func.as_str() {
                "abs" => a0.abs(),
                "ln" => a0.max(1e-6).ln(),
                "sqrt" => a0.max(0.0).sqrt(),
                "min" => a0.min(a1),
                "max" => a0.max(a1),
                "clip" => {
                    let lo = a1.min(a2);
                    let hi = a1.max(a2);
                    a0.clamp(lo, hi)
                }
                "sign" => {
                    if a0 > 0.0 {
                        1.0
                    } else if a0 < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                }
                "pow" => a0.powf(a1.clamp(-4.0, 4.0)),
                _ => 0.0,
            }
        }
    };
    if raw.is_finite() {
        raw
    } else {
        0.0
    }
}

fn median_of(slice: &[f64]) -> f64 {
    if slice.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = slice
        .iter()
        .copied()
        .map(|x| if x.is_finite() { x } else { 0.0 })
        .collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        0.5 * (sorted[mid - 1] + sorted[mid])
    }
}

pub fn winsorize_mad(values: &[f64], mad_mult: f64) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let med = median_of(values);
    let abs_dev: Vec<f64> = values.iter().map(|v| (v - med).abs()).collect();
    let mad = median_of(&abs_dev);
    let scaled_mad = (1.4826 * mad).max(1e-6);
    let band = mad_mult.max(1.0) * scaled_mad;
    values
        .iter()
        .map(|v| {
            if !v.is_finite() {
                med
            } else if mad < 1e-9 {
                *v
            } else {
                v.clamp(med - band, med + band)
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct CrossSectionSample {
    pub symbol: String,
    pub industry_sw1: String,
    pub circ_mv_yi: f64,
    pub raw_factors: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct CrossSectionScored {
    pub symbol: String,
    pub factor_z_scores: BTreeMap<String, f64>,
    pub conviction_score: f64,
}

pub fn neutralize_and_score_cross_section(
    samples: &[CrossSectionSample],
    factor_weights: &BTreeMap<String, f64>,
) -> Vec<CrossSectionScored> {
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }

    let ln_caps: Vec<f64> = samples
        .iter()
        .map(|s| s.circ_mv_yi.max(1.0).ln())
        .collect();
    let mean_x = ln_caps.iter().sum::<f64>() / (n as f64);
    let var_x: f64 = ln_caps.iter().map(|x| (x - mean_x).powi(2)).sum();

    let mut per_sample_z: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new(); n];
    let total_weight: f64 = factor_weights.values().map(|w| w.abs()).sum::<f64>().max(1e-6);

    for (factor_id, _) in factor_weights {
        let raw_vec: Vec<f64> = samples
            .iter()
            .map(|s| s.raw_factors.get(factor_id).copied().unwrap_or(0.0))
            .collect();
        let clipped = winsorize_mad(&raw_vec, 3.0);

        // Industry SW1 demeaning
        let mut ind_sum: BTreeMap<&str, (f64, usize)> = BTreeMap::new();
        for (idx, s) in samples.iter().enumerate() {
            let entry = ind_sum.entry(s.industry_sw1.as_str()).or_insert((0.0, 0));
            entry.0 += clipped[idx];
            entry.1 += 1;
        }
        let ind_demeaned: Vec<f64> = samples
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                let (sum, cnt) = ind_sum
                    .get(s.industry_sw1.as_str())
                    .copied()
                    .unwrap_or((0.0, 1));
                if cnt >= 2 {
                    clipped[idx] - (sum / (cnt as f64))
                } else {
                    clipped[idx]
                }
            })
            .collect();

        // OLS orthogonalization against ln(circ_mv_yi)
        let mean_y = ind_demeaned.iter().sum::<f64>() / (n as f64);
        let beta = if var_x > 1e-8 && n >= 3 {
            let cov_xy: f64 = (0..n)
                .map(|i| (ln_caps[i] - mean_x) * (ind_demeaned[i] - mean_y))
                .sum();
            cov_xy / var_x
        } else {
            0.0
        };
        let residuals: Vec<f64> = (0..n)
            .map(|i| (ind_demeaned[i] - mean_y) - beta * (ln_caps[i] - mean_x))
            .collect();

        let res_mean = residuals.iter().sum::<f64>() / (n as f64);
        let res_std = (residuals
            .iter()
            .map(|r| (r - res_mean).powi(2))
            .sum::<f64>()
            / (n as f64))
            .sqrt();

        for i in 0..n {
            let z = if res_std > 1e-8 {
                ((residuals[i] - res_mean) / res_std).clamp(-3.0, 3.0)
            } else {
                0.0
            };
            per_sample_z[i].insert(factor_id.clone(), (z * 1000.0).round() / 1000.0);
        }
    }

    samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut weighted_z = 0.0_f64;
            for (fid, w) in factor_weights {
                let z = per_sample_z[i].get(fid).copied().unwrap_or(0.0);
                weighted_z += z * (*w / total_weight);
            }
            let conviction = (62.0 + weighted_z * 16.0).clamp(0.0, 100.0);
            CrossSectionScored {
                symbol: s.symbol.clone(),
                factor_z_scores: per_sample_z[i].clone(),
                conviction_score: (conviction * 10.0).round() / 10.0,
            }
        })
        .collect()
}

pub fn compute_initial_hard_stop_fen(
    entry_price_fen: Fen,
    prev_low_fen: Fen,
    atr14_fen: Fen,
    atr_mult: f64,
) -> Fen {
    let entry = entry_price_fen.0.max(2);
    let atr_stop = (entry as f64) - atr_mult.max(0.5) * (atr14_fen.0.max(1) as f64);
    let raw_candidate = if prev_low_fen.0 > 0 {
        (prev_low_fen.0 as f64).min(atr_stop)
    } else {
        atr_stop
    };
    let lower = ((entry as f64) * 0.925).round().max(1.0);
    let upper = ((entry as f64) * 0.960)
        .round()
        .min((entry - 1) as f64)
        .max(lower.min((entry - 1) as f64));
    let clipped = raw_candidate.clamp(lower.min(upper), upper).round() as i64;
    Fen(clipped.clamp(1, entry - 1))
}

pub fn compute_chandelier_stop_fen(
    entry_price_fen: Fen,
    current_stop_fen: Fen,
    highest_price_fen: Fen,
    mark_price_fen: Fen,
    atr14_fen: Fen,
    trigger_gain_pct: f64,
    chandelier_atr_mult: f64,
) -> Fen {
    let entry = entry_price_fen.0.max(2);
    let highest = highest_price_fen.0.max(mark_price_fen.0).max(entry);
    let gain_pct = ((highest - entry) as f64 / (entry as f64)) * 100.0;
    if gain_pct < trigger_gain_pct {
        return current_stop_fen;
    }
    let trail = (highest as f64
        - chandelier_atr_mult.max(1.0) * (atr14_fen.0.max(1) as f64))
    .round() as i64;
    let ceiling = (mark_price_fen.0 - 1).max(1);
    Fen(trail.max(current_stop_fen.0).min(ceiling))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn parses_and_evaluates_declarative_factor_ast_safely() {
        let ast = parse_factor_expr(
            "clip((ret_high_turn_5d - ret_low_turn_5d) * -1.0, -3.5, 3.5)",
        )
        .unwrap();
        let mut vars = BTreeMap::new();
        vars.insert("ret_high_turn_5d".to_string(), -2.1);
        vars.insert("ret_low_turn_5d".to_string(), 0.8);
        let val = eval_factor_ast(&ast, &vars);
        assert!((val - 2.9).abs() < 1e-6);

        // Division by zero must never panic or return NaN/Inf
        let div0 = parse_factor_expr("a / (b - b)").unwrap();
        vars.insert("a".to_string(), 10.0);
        vars.insert("b".to_string(), 3.0);
        assert!(eval_factor_ast(&div0, &vars).is_finite());
    }

    #[test]
    fn neutralizes_cross_section_and_bounds_conviction_score() {
        let mut weights = BTreeMap::new();
        weights.insert("F1".to_string(), 60.0);
        weights.insert("F2".to_string(), 40.0);

        let samples: Vec<CrossSectionSample> = (0..8)
            .map(|i| {
                let mut f = BTreeMap::new();
                f.insert("F1".to_string(), i as f64 * 0.5);
                f.insert("F2".to_string(), if i == 7 { 999.0 } else { (i as f64) * 0.3 });
                CrossSectionSample {
                    symbol: format!("00000{}.SZ", i + 1),
                    industry_sw1: if i % 2 == 0 { "电子".to_string() } else { "机械".to_string() },
                    circ_mv_yi: 30.0 + (i as f64) * 15.0,
                    raw_factors: f,
                }
            })
            .collect();

        let scored = neutralize_and_score_cross_section(&samples, &weights);
        assert_eq!(scored.len(), 8);
        for item in &scored {
            assert!((0.0..=100.0).contains(&item.conviction_score));
        }
    }

    proptest! {
        #[test]
        fn prop_initial_stop_strictly_below_entry_and_within_band(
            entry_fen in 300_i64..200_000_i64,
            prev_low_fen in 100_i64..200_000_i64,
            atr_fen in 10_i64..20_000_i64,
        ) {
            let entry = Fen(entry_fen);
            let stop = compute_initial_hard_stop_fen(entry, Fen(prev_low_fen), Fen(atr_fen), 2.0);
            prop_assert!(stop.0 > 0);
            prop_assert!(stop < entry);

            let min_allowed = ((entry_fen as f64) * 0.925).floor() as i64 - 1;
            let max_allowed = ((entry_fen as f64) * 0.960).ceil() as i64 + 1;
            prop_assert!(stop.0 >= min_allowed.max(1));
            prop_assert!(stop.0 <= max_allowed.min(entry_fen - 1));
        }
    }
}
