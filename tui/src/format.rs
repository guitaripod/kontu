//! Small display formatters (thin-space grouped euros, areas, optionals).

pub fn group_thousands(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(ch);
    }
    let grouped: String = out.chars().rev().collect();
    if neg {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub fn money(v: f64) -> String {
    format!("{}\u{202f}€", group_thousands(v.round() as i64))
}

pub fn money_opt(v: Option<i64>) -> String {
    match v {
        Some(v) => format!("{}\u{202f}€", group_thousands(v)),
        None => "—".to_string(),
    }
}

pub fn area_opt(v: Option<f64>) -> String {
    match v {
        Some(v) => format!("{:.0}\u{202f}m²", v),
        None => "—".to_string(),
    }
}

pub fn num_opt(v: Option<f64>) -> String {
    match v {
        Some(v) if v.fract() == 0.0 => format!("{:.0}", v),
        Some(v) => format!("{v:.1}"),
        None => "—".to_string(),
    }
}

pub fn int_opt(v: Option<i32>) -> String {
    v.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string())
}

pub fn str_opt(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "—".to_string())
}

pub fn ppm2_opt(v: Option<f64>) -> String {
    match v {
        Some(v) => format!("{}\u{202f}€/m²", group_thousands(v.round() as i64)),
        None => "—".to_string(),
    }
}

pub fn pct(v: f64) -> String {
    format!("{:.2}%", v * 100.0)
}
