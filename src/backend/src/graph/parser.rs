use std::sync::OnceLock;

fn wikilink_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap())
}

fn inline_code_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"`[^`]+`").unwrap())
}

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = wikilink_re();
    let mut links = Vec::new();

    let mut in_code_block = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        let cleaned = remove_inline_code(line);

        for cap in re.captures_iter(&cleaned) {
            if let Some(target) = cap.get(1) {
                let target = target.as_str();
                // Handle [[target|display text]] - take target part
                let target = target.split('|').next().unwrap_or(target).trim();
                if !target.is_empty() {
                    links.push(target.to_string());
                }
            }
        }
    }

    links
}

fn remove_inline_code(line: &str) -> String {
    inline_code_re().replace_all(line, "").to_string()
}
