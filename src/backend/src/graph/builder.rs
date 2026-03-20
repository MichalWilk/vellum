use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::docs::markdown::extract_tags;
use crate::graph::parser;

fn image_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"!\[[^\]]*\]\(([^)]+)\)").unwrap())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

pub fn build_graph(vault_root: &Path) -> GraphData {
    let mut all_files_vec = Vec::new();
    crate::docs::walk::walk_vault_files(vault_root, |rel, _| {
        all_files_vec.push(rel.to_string());
    });
    let all_files: HashSet<String> = all_files_vec.iter().cloned().collect();

    // Build filename -> paths index for Obsidian-style resolution
    let mut name_index: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in &all_files_vec {
        let filename = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        name_index.entry(filename).or_default().push(file_path.clone());
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut tag_set: HashSet<String> = HashSet::new();
    let mut attachment_set: HashSet<String> = HashSet::new();

    let img_regex = image_re();

    for file_path in &all_files_vec {
        let label = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_path)
            .to_string();

        nodes.push(GraphNode {
            id: file_path.clone(),
            label,
            node_type: "doc".to_string(),
        });

        let full_path = vault_root.join(file_path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Wikilink edges
        let links = parser::extract_wikilinks(&content);
        for link in links {
            if let Some(resolved) = resolve_link(&link, &all_files, &name_index) {
                edges.push(GraphEdge {
                    source: file_path.clone(),
                    target: resolved,
                });
            }
        }

        // Tag edges
        let tags = extract_tags(&content);
        for tag in tags {
            let tag_id = format!("tag:{tag}");
            tag_set.insert(tag.clone());
            edges.push(GraphEdge {
                source: file_path.clone(),
                target: tag_id,
            });
        }

        // Attachment edges (image references)
        let doc_dir = Path::new(file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        for cap in img_regex.captures_iter(&content) {
            let src = cap[1].trim();
            if src.starts_with("http://") || src.starts_with("https://") {
                continue;
            }
            let resolved = if src.starts_with('/') {
                src[1..].to_string()
            } else if doc_dir.is_empty() {
                src.to_string()
            } else {
                format!("{doc_dir}/{src}")
            };
            attachment_set.insert(resolved.clone());
            edges.push(GraphEdge {
                source: file_path.clone(),
                target: format!("attachment:{resolved}"),
            });
        }
    }

    // Add tag nodes
    for tag in &tag_set {
        nodes.push(GraphNode {
            id: format!("tag:{tag}"),
            label: format!("#{tag}"),
            node_type: "tag".to_string(),
        });
    }

    // Add attachment nodes
    for att in &attachment_set {
        let label = Path::new(att)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(att)
            .to_string();
        nodes.push(GraphNode {
            id: format!("attachment:{att}"),
            label,
            node_type: "attachment".to_string(),
        });
    }

    GraphData { nodes, edges }
}

fn resolve_link(
    target: &str,
    all_files: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
) -> Option<String> {
    // 1. Exact match with .md
    if target.ends_with(".md") && all_files.contains(target) {
        return Some(target.to_string());
    }

    // 2. Append .md
    let with_md = format!("{}.md", target);
    if all_files.contains(&with_md) {
        return Some(with_md);
    }

    // 3. Search by filename (Obsidian-style)
    let filename = Path::new(target)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(target);

    if let Some(paths) = name_index.get(filename) {
        if !paths.is_empty() {
            return Some(paths[0].clone());
        }
    }

    None
}

pub async fn rebuild_and_cache(
    vault_root: PathBuf,
    redis_pool: tower_sessions_redis_store::fred::prelude::Pool,
    vault_name: &str,
) {
    use tower_sessions_redis_store::fred::interfaces::KeysInterface;

    let graph = tokio::task::spawn_blocking(move || build_graph(&vault_root))
        .await
        .unwrap_or_else(|_| GraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
        });

    let json = match serde_json::to_string(&graph) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("failed to serialize graph: {e}");
            return;
        }
    };

    let redis_key = format!("graph:{vault_name}");
    if let Err(e) = redis_pool
        .set::<(), _, _>(redis_key.as_str(), json.as_str(), None, None, false)
        .await
    {
        tracing::error!("failed to cache graph in Redis: {e}");
    } else {
        tracing::info!(
            "graph cached for vault '{}': {} nodes, {} edges",
            vault_name,
            graph.nodes.len(),
            graph.edges.len()
        );
    }
}

pub async fn load_cached(
    redis_pool: &tower_sessions_redis_store::fred::prelude::Pool,
    vault_name: &str,
) -> Option<GraphData> {
    use tower_sessions_redis_store::fred::interfaces::KeysInterface;

    let redis_key = format!("graph:{vault_name}");
    let json: Option<String> = redis_pool.get(redis_key.as_str()).await.ok()?;
    let json = json?;
    serde_json::from_str(&json).ok()
}
