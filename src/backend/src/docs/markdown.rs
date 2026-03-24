use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use crate::docs::vault_config::{self, VaultConfigCache};

fn embed_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"!\[\[([^\]]+)\]\]").unwrap())
}

fn highlight_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"==([^=]+)==").unwrap())
}

fn wikilink_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap())
}

fn img_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"<img\s+([^>]*?)src="([^"]+)"([^>]*?)>"#).unwrap())
}

fn callout_icon_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r#"<span class="vellum-callout-icon-placeholder" data-type="([^"]*?)"></span>"#,
        )
        .unwrap()
    })
}

fn tag_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(^|[\s,(>])#([a-zA-Z][a-zA-Z0-9_/-]*)").unwrap())
}

fn inline_code_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"`[^`]+`").unwrap())
}

pub fn render_document(
    raw: &str,
    doc_path: &str,
    vault_name: &str,
    vault_root: &Path,
    vault_configs: &VaultConfigCache,
    user_roles: &[String],
    default_roles: &[String],
    auth_mode: &crate::config::AuthMode,
) -> (HashMap<String, serde_json::Value>, String) {
    let (frontmatter, body) = parse_frontmatter(raw);
    let body = resolve_embeds(
        body, vault_root, vault_name, &mut HashSet::new(), 0,
        vault_configs, user_roles, default_roles, auth_mode,
    );
    let body = replace_highlights(&body);
    let body = replace_wikilinks(&body, vault_name);
    let body = preprocess_callouts(&body);
    let html = render_markdown(&body);
    let html = rewrite_image_paths(&html, doc_path, vault_name);
    let html = linkify_tags(&html, vault_name);
    let mut sanitizer = ammonia::Builder::default();
    sanitizer.add_tag_attributes("a", &["href", "class"]);
    sanitizer.add_tag_attributes("img", &["src", "alt"]);
    sanitizer.add_tag_attributes("div", &["class"]);
    sanitizer.add_tag_attributes("p", &["class"]);
    sanitizer.add_tag_attributes("span", &["class", "data-type"]);
    sanitizer.add_tags(&["mark", "details", "summary"]);
    for tag in &["h1", "h2", "h3", "h4", "h5", "h6"] {
        sanitizer.add_tag_attributes(tag, &["id"]);
    }
    let sanitized = sanitizer.clean(&html).to_string();
    let sanitized = restore_callout_icons(&sanitized);
    (frontmatter, sanitized)
}

// Embeds: ![[note]] -> inline content of referenced note
fn resolve_embeds(
    body: &str,
    vault_root: &Path,
    vault_name: &str,
    visited: &mut HashSet<String>,
    depth: u8,
    vault_configs: &VaultConfigCache,
    user_roles: &[String],
    default_roles: &[String],
    auth_mode: &crate::config::AuthMode,
) -> String {
    if depth > 5 {
        return body.to_string();
    }

    let re = embed_re();
    re.replace_all(body, |caps: &regex::Captures| {
        let target = caps[1].split('|').next().unwrap_or(&caps[1]).trim();

        let resolved_path = match resolve_file_path(target, vault_root) {
            Some(p) => p,
            None => return format!("*[Embed not found: {target}]*"),
        };

        // Check role-based access for the embedded file
        if *auth_mode != crate::config::AuthMode::None {
            let roles = vault_config::resolve_roles(vault_configs, &resolved_path, default_roles);
            if !vault_config::check_access(user_roles, &roles) {
                return format!("*[Access denied: {target}]*");
            }
        }

        if visited.contains(&resolved_path) {
            return format!("*[Circular embed: {target}]*");
        }

        let full_path = vault_root.join(&resolved_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                visited.insert(resolved_path);
                let (_, embed_body) = parse_frontmatter(&content);
                let resolved = resolve_embeds(
                    embed_body, vault_root, vault_name, visited, depth + 1,
                    vault_configs, user_roles, default_roles, auth_mode,
                );
                format!("\n<div class=\"vellum-embed\">\n\n{resolved}\n\n</div>\n")
            }
            Err(_) => format!("*[Embed not found: {target}]*"),
        }
    })
    .to_string()
}

// Obsidian-style file resolution: exact path, exact+.md, search by filename
fn resolve_file_path(target: &str, vault_root: &Path) -> Option<String> {
    // 1. Exact path with .md
    let with_md = if target.ends_with(".md") {
        target.to_string()
    } else {
        format!("{target}.md")
    };
    if vault_root.join(&with_md).exists() {
        return Some(with_md);
    }

    // 2. Exact path as-is
    if vault_root.join(target).exists() {
        return Some(target.to_string());
    }

    // 3. Search by filename in vault (Obsidian-style)
    let search_name = if target.ends_with(".md") {
        target.to_string()
    } else {
        format!("{target}.md")
    };
    find_file_recursive(vault_root, vault_root, &search_name)
}

fn find_file_recursive(dir: &Path, vault_root: &Path, filename: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if let Ok(ft) = entry.file_type() {
            if ft.is_file() && name == filename {
                return Some(
                    path.strip_prefix(vault_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string(),
                );
            }
            if ft.is_dir() {
                if let Some(found) = find_file_recursive(&path, vault_root, filename) {
                    return Some(found);
                }
            }
        }
    }
    None
}

// Highlights: ==text== -> <mark>text</mark>
fn replace_highlights(body: &str) -> String {
    highlight_re().replace_all(body, "<mark>$1</mark>").to_string()
}

// Callouts: > [!type] title -> styled div
// Processes blockquote lines starting with [!type]
fn preprocess_callouts(body: &str) -> String {
    let mut result = String::new();
    let mut lines = body.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(callout) = parse_callout_start(line) {
            let mut content = String::new();
            // Collect continuation lines (starting with >)
            while let Some(next) = lines.peek() {
                if let Some(stripped) = next.strip_prefix('>') {
                    content.push_str(stripped.strip_prefix(' ').unwrap_or(stripped));
                    content.push('\n');
                    lines.next();
                } else {
                    break;
                }
            }

            let color_class = callout_color(&callout.callout_type);
            let title = if callout.title.is_empty() {
                capitalize(&callout.callout_type)
            } else {
                callout.title
            };
            let safe_title = html_escape(&title);
            let icon_placeholder = format!(
                r#"<span class="vellum-callout-icon-placeholder" data-type="{}"></span>"#,
                html_escape(&callout.callout_type)
            );

            result.push_str(&format!(
                "<div class=\"vellum-callout {color_class}\">\n<p class=\"vellum-callout-title\">{icon_placeholder} {safe_title}</p>\n\n{content}\n</div>\n\n"
            ));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

struct CalloutStart {
    callout_type: String,
    title: String,
}

fn parse_callout_start(line: &str) -> Option<CalloutStart> {
    let stripped = line.strip_prefix('>')?.trim_start();
    let rest = stripped.strip_prefix("[!")?;
    let end = rest.find(']')?;
    let callout_type = rest[..end].to_lowercase();
    let title = rest[end + 1..].trim().to_string();
    Some(CalloutStart { callout_type, title })
}

fn lucide_svg(inner: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="vellum-callout-icon">{inner}</svg>"#
    )
}

fn callout_icon(callout_type: &str) -> String {
    let inner = match callout_type {
        "note" => r#"<line x1="12" y1="20" x2="12" y2="10"></line><line x1="6" y1="20" x2="6" y2="14"></line><line x1="18" y1="20" x2="18" y2="4"></line>"#,
        "tip" | "hint" => r#"<path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"></path><path d="M9 18h6"></path><path d="M10 22h4"></path>"#,
        "warning" | "caution" | "attention" => r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line>"#,
        "danger" | "error" => r#"<path d="M7.86 2h8.28L22 7.86v8.28L16.14 22H7.86L2 16.14V7.86z"></path><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line>"#,
        "info" => r#"<circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line>"#,
        "question" | "faq" | "help" => r#"<circle cx="12" cy="12" r="10"></circle><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path><line x1="12" y1="17" x2="12.01" y2="17"></line>"#,
        "success" | "check" | "done" => r#"<circle cx="12" cy="12" r="10"></circle><path d="m9 12 2 2 4-4"></path>"#,
        "failure" | "fail" | "missing" => r#"<circle cx="12" cy="12" r="10"></circle><path d="m15 9-6 6"></path><path d="m9 9 6 6"></path>"#,
        "bug" => r#"<path d="m8 2 1.88 1.88"></path><path d="M14.12 3.88 16 2"></path><path d="M9 7.13v-1a3.003 3.003 0 1 1 6 0v1"></path><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6"></path><path d="M12 20v-9"></path><path d="M6.53 9C4.6 8.8 3 7.1 3 5"></path><path d="M6 13H2"></path><path d="M3 21c0-2.1 1.7-3.9 3.8-4"></path><path d="M20.97 5c0 2.1-1.6 3.8-3.5 4"></path><path d="M22 13h-4"></path><path d="M17.2 17c2.1.1 3.8 1.9 3.8 4"></path>"#,
        "example" => r#"<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"></path><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"></path>"#,
        "quote" | "cite" => r#"<path d="M6 17h3l2-4V7H5v6h3"></path><path d="M14 17h3l2-4V7h-6v6h3"></path>"#,
        "abstract" | "summary" | "tldr" => r#"<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z"></path><path d="M14 2v4a2 2 0 0 0 2 2h4"></path><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><line x1="10" y1="9" x2="8" y2="9"></line>"#,
        _ => r#"<line x1="12" y1="20" x2="12" y2="10"></line><line x1="6" y1="20" x2="6" y2="14"></line><line x1="18" y1="20" x2="18" y2="4"></line>"#,
    };
    lucide_svg(inner)
}

fn callout_color(callout_type: &str) -> &'static str {
    match callout_type {
        "note" => "callout-blue",
        "tip" | "hint" => "callout-green",
        "warning" | "caution" | "attention" => "callout-yellow",
        "danger" | "error" => "callout-red",
        "info" => "callout-blue",
        "question" | "faq" | "help" => "callout-yellow",
        "success" | "check" | "done" => "callout-green",
        "failure" | "fail" | "missing" => "callout-red",
        "bug" => "callout-red",
        "example" => "callout-purple",
        "quote" | "cite" => "callout-gray",
        "abstract" | "summary" | "tldr" => "callout-blue",
        _ => "callout-blue",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn restore_callout_icons(html: &str) -> String {
    let re = callout_icon_re();
    re.replace_all(html, |caps: &regex::Captures| {
        let callout_type = caps[1]
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"");
        callout_icon(&callout_type)
    })
    .to_string()
}

fn rewrite_image_paths(html: &str, doc_path: &str, vault_name: &str) -> String {
    let doc_dir = Path::new(doc_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let re = img_re();
    re.replace_all(html, |caps: &regex::Captures| {
        let pre = &caps[1];
        let src = &caps[2];
        let post = &caps[3];

        if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("/api/") {
            return caps[0].to_string();
        }

        let resolved = if src.starts_with("./") {
            let relative = &src[2..];
            if doc_dir.is_empty() {
                relative.to_string()
            } else {
                format!("{doc_dir}/{relative}")
            }
        } else if src.starts_with('/') {
            src[1..].to_string()
        } else if doc_dir.is_empty() {
            src.to_string()
        } else {
            format!("{doc_dir}/{src}")
        };

        format!(r#"<img {pre}src="/api/assets/{vault_name}/{resolved}"{post}>"#)
    })
    .to_string()
}

fn replace_wikilinks(body: &str, vault_name: &str) -> String {
    let re = wikilink_re();
    re.replace_all(body, |caps: &regex::Captures| {
        let inner = &caps[1];
        let (target, display) = match inner.split_once('|') {
            Some((t, d)) => (t.trim(), d.trim()),
            None => (inner.trim(), inner.trim()),
        };
        let href = if target.ends_with(".md") {
            format!("/docs/{vault_name}/{target}")
        } else {
            format!("/docs/{vault_name}/{target}.md")
        };
        format!("[{display}]({href})")
    })
    .to_string()
}

fn parse_frontmatter(raw: &str) -> (HashMap<String, serde_json::Value>, &str) {
    if !raw.starts_with("---") {
        return (HashMap::new(), raw);
    }

    if let Some(end) = raw[3..].find("\n---") {
        let yaml_str = &raw[3..3 + end].trim();
        let body = &raw[3 + end + 4..];

        let frontmatter: HashMap<String, serde_json::Value> =
            match serde_yaml::from_str(yaml_str) {
                Ok(map) => map,
                Err(_) => HashMap::new(),
            };

        (frontmatter, body.trim_start_matches('\n'))
    } else {
        (HashMap::new(), raw)
    }
}

fn linkify_tags(html: &str, vault_name: &str) -> String {
    let tag_regex = tag_re();

    // Split HTML into segments: tags (inside < >) and text (outside)
    // Skip replacement inside <code>, <pre>, and <a> elements
    let mut result = String::with_capacity(html.len());
    let mut skip_depth: usize = 0;
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Find end of this HTML tag
            let tag_end = match html[pos..].find('>') {
                Some(i) => pos + i + 1,
                None => {
                    result.push_str(&html[pos..]);
                    break;
                }
            };
            let tag_content = &html[pos..tag_end];

            // Check if entering/leaving a skip element
            let lower = tag_content.to_ascii_lowercase();
            if lower.starts_with("<code") || lower.starts_with("<pre") || lower.starts_with("<a ") || lower.starts_with("<a>") {
                skip_depth += 1;
            } else if lower.starts_with("</code") || lower.starts_with("</pre") || lower.starts_with("</a") {
                skip_depth = skip_depth.saturating_sub(1);
            }

            result.push_str(tag_content);
            pos = tag_end;
        } else {
            // Find next HTML tag or end
            let next_tag = html[pos..].find('<').map(|i| pos + i).unwrap_or(html.len());
            let text = &html[pos..next_tag];

            if skip_depth == 0 {
                let replaced = tag_regex.replace_all(text, |caps: &regex::Captures| {
                    let prefix = &caps[1];
                    let tag = &caps[2];
                    format!(
                        r#"{prefix}<a href="/docs/{vault_name}?tag={tag}" class="vellum-tag">#{tag}</a>"#
                    )
                });
                result.push_str(&replaced);
            } else {
                result.push_str(text);
            }

            pos = next_tag;
        }
    }

    result
}

// Extract all tags (frontmatter + inline) from raw markdown content
pub fn extract_tags(raw: &str) -> Vec<String> {
    let (frontmatter, body) = parse_frontmatter(raw);
    let mut tags = Vec::new();

    // Frontmatter tags
    if let Some(fm_tags) = frontmatter.get("tags") {
        if let Some(arr) = fm_tags.as_array() {
            for v in arr {
                if let Some(s) = v.as_str() {
                    tags.push(s.to_string());
                }
            }
        }
    }

    let tag_regex = tag_re();
    let mut in_code_block = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        // Skip headings
        if trimmed.starts_with('#') && trimmed.chars().nth(1).is_none_or(|c| c == ' ' || c == '#') {
            continue;
        }
        // Extract inline tags, skipping those inside inline code
        let mut search_text = trimmed.to_string();
        search_text = inline_code_re().replace_all(&search_text, "").to_string();

        for caps in tag_regex.captures_iter(&search_text) {
            tags.push(caps[2].to_string());
        }
    }

    tags
}

/// Generate a URL-safe anchor slug from heading text.
/// Lowercase, spaces to hyphens, strip non-alphanumeric except hyphens.
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn render_markdown(body: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, HeadingLevel};

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    let parser = Parser::new_ext(body, options);
    let mut html_output = String::new();
    let mut heading_text = String::new();
    let mut in_heading = false;
    let mut heading_level: Option<HeadingLevel> = None;
    let mut slug_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = Some(level);
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let base_slug = slugify(&heading_text);
                let count = slug_counts.entry(base_slug.clone()).or_insert(0);
                let slug = if *count == 0 {
                    base_slug.clone()
                } else {
                    format!("{base_slug}-{count}")
                };
                *count += 1;

                let level_num = match heading_level {
                    Some(HeadingLevel::H1) => 1,
                    Some(HeadingLevel::H2) => 2,
                    Some(HeadingLevel::H3) => 3,
                    Some(HeadingLevel::H4) => 4,
                    Some(HeadingLevel::H5) => 5,
                    Some(HeadingLevel::H6) => 6,
                    None => 1,
                };
                html_output.push_str(&format!(
                    "<h{level_num} id=\"{slug}\">{heading_text}</h{level_num}>\n"
                ));
                heading_text.clear();
                heading_level = None;
            }
            Event::Text(ref t) if in_heading => {
                heading_text.push_str(t);
            }
            Event::Code(ref t) if in_heading => {
                heading_text.push_str(t);
            }
            _ if in_heading => {
                // Skip inline formatting events inside headings - plain text only
            }
            other => {
                let mut tmp = String::new();
                pulldown_cmark::html::push_html(&mut tmp, std::iter::once(other));
                html_output.push_str(&tmp);
            }
        }
    }

    html_output
}
