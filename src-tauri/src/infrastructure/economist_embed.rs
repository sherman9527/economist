use chrono::NaiveDate;

pub(crate) fn validate_canonical_report_id(report_id: &str) -> Result<String, String> {
    let trimmed = report_id.trim();
    if trimmed.len() != 10
        || !trimmed.bytes().enumerate().all(|(idx, b)| match idx {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        })
    {
        return Err(format!(
            "非法研报标识格式 '{}'（必须为 YYYY-MM-DD）",
            report_id
        ));
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| format!("非法研报日历日期 '{}'", trimmed))?;
    Ok(trimmed.to_string())
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

fn find_opening_head_end(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find("<head") {
        let pos = search_from + rel;
        let after_kw = pos + "<head".len();
        let next_ok = lower
            .as_bytes()
            .get(after_kw)
            .map(|b| *b == b'>' || b.is_ascii_whitespace())
            .unwrap_or(false);
        if next_ok {
            if let Some(gt_rel) = lower[after_kw..].find('>') {
                return Some(after_kw + gt_rel + 1);
            }
        }
        search_from = after_kw;
    }
    None
}

pub(crate) fn prepare_embedded_report_html(raw: &str, echarts_js: Option<&str>) -> String {
    let with_flag = inject_embed_flag_on_html(raw);
    let with_echarts = match echarts_js {
        Some(js) if !js.is_empty() => inline_echarts_script_tags(&with_flag, js),
        _ => with_flag,
    };
    if with_echarts.contains("data-economist-embed=") {
        return with_echarts;
    }
    let embed_csp_and_style = concat!(
        "<meta data-economist-csp=\"1\" http-equiv=\"Content-Security-Policy\" ",
        "content=\"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; ",
        "img-src data: blob:; font-src data:; connect-src 'none'; object-src 'none'; base-uri 'none';\">",
        "<style data-economist-embed=\"1\">html[data-embed=\"1\"] #ackBtn,html[data-embed=\"1\"] .ack-btn{display:none!important;}</style>"
    );
    if let Some(insert_at) = find_opening_head_end(&with_echarts) {
        let mut out = String::with_capacity(with_echarts.len() + embed_csp_and_style.len());
        out.push_str(&with_echarts[..insert_at]);
        out.push_str(embed_csp_and_style);
        out.push_str(&with_echarts[insert_at..]);
        out
    } else {
        let lower = with_echarts.to_ascii_lowercase();
        if let Some(head_close) = lower.find("</head>") {
            let mut out = String::with_capacity(with_echarts.len() + embed_csp_and_style.len());
            out.push_str(&with_echarts[..head_close]);
            out.push_str(embed_csp_and_style);
            out.push_str(&with_echarts[head_close..]);
            out
        } else {
            format!("{}{}", embed_csp_and_style, with_echarts)
        }
    }
}
