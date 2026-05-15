pub fn strip_ansi(s: &str) -> String {
    regex::Regex::new("\x1b\\[[0-9;]*m")
        .unwrap()
        .replace_all(s, "")
        .to_string()
}

pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}

pub fn format_savings(input: u32, output: u32) -> String {
    if input == 0 {
        return "0%".into();
    }
    let pct = ((input - output) as f64 / input as f64 * 100.0).round() as u32;
    format!("{}%", pct.min(100))
}
