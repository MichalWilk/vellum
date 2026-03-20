use axum::{
    extract::Path as AxumPath,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::User;
use crate::config::AuthMode;
use crate::docs::vault_config::{self, VaultConfigCache};

type VaultConfigsMap = Arc<RwLock<HashMap<String, VaultConfigCache>>>;

#[derive(Serialize)]
pub enum NodeType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "dir")]
    Dir,
}

#[derive(Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
}

#[derive(Serialize)]
pub struct DocResponse {
    pub content: String,
    pub frontmatter: std::collections::HashMap<String, serde_json::Value>,
    pub path: String,
    pub last_modified: Option<String>,
    pub last_modified_by: Option<String>,
}

pub async fn tree(
    user: User,
    vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    vault_configs: VaultConfigsMap,
    vault_name: &str,
) -> Json<Vec<FileNode>> {
    let configs = get_cached_configs(&vault_configs, vault_name).await;
    let nodes = tokio::task::spawn_blocking(move || {
        build_tree(&vault_root, &vault_root, &configs, &user, &auth_mode, &default_roles)
    })
    .await
    .unwrap_or_default();

    Json(nodes)
}

async fn get_cached_configs(vault_configs: &VaultConfigsMap, vault_name: &str) -> VaultConfigCache {
    let guard = vault_configs.read().await;
    guard.get(vault_name).cloned().unwrap_or_default()
}

fn build_tree(
    dir: &Path,
    vault_root: &Path,
    configs: &vault_config::VaultConfigCache,
    user: &User,
    auth_mode: &AuthMode,
    default_roles: &[String],
) -> Vec<FileNode> {
    let mut entries = match std::fs::read_dir(dir) {
        Ok(e) => e.flatten().collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };

    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        let full_path = entry.path();
        let rel_path = full_path
            .strip_prefix(vault_root)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .to_string();

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_dir() {
            let dir_rel = format!("{}/", rel_path);

            if *auth_mode != AuthMode::None {
                let roles = vault_config::resolve_roles(configs, &dir_rel, default_roles);
                if !vault_config::check_access(&user.roles, &roles) {
                    continue;
                }
            }

            let children = build_tree(
                &full_path, vault_root, configs, user, auth_mode, default_roles,
            );

            // Skip empty directories (e.g. images-only dirs)
            if !children.is_empty() {
                dirs.push(FileNode {
                    name,
                    path: rel_path,
                    node_type: NodeType::Dir,
                    children: Some(children),
                });
            }
        } else if ft.is_file() && name.ends_with(".md") {
            if *auth_mode != AuthMode::None {
                let roles = vault_config::resolve_roles(configs, &rel_path, default_roles);
                if !vault_config::check_access(&user.roles, &roles) {
                    continue;
                }
            }

            files.push(FileNode {
                name,
                path: rel_path,
                node_type: NodeType::File,
                children: None,
            });
        }
    }

    dirs.extend(files);
    dirs
}

fn validate_path_common(path: &str) -> Result<(), StatusCode> {
    if path.contains("..") {
        return Err(StatusCode::BAD_REQUEST);
    }
    if path.split('/').any(|segment| segment.starts_with('.')) {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(())
}

fn validate_doc_path(path: &str) -> Result<(), StatusCode> {
    validate_path_common(path)?;
    if !path.ends_with(".md") {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(())
}

fn validate_asset_path(path: &str) -> Result<(), StatusCode> {
    validate_path_common(path)?;
    if path.ends_with(".md") {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(())
}

fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

pub async fn doc(
    user: User,
    AxumPath(path): AxumPath<String>,
    vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    vault_name: &str,
    vault_configs: VaultConfigsMap,
) -> Response {
    if let Err(status) = validate_doc_path(&path) {
        return status.into_response();
    }

    let full_path = vault_root.join(&path);

    let canonical = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let canonical_root = match vault_root.canonicalize() {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if !canonical.starts_with(&canonical_root) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let configs = get_cached_configs(&vault_configs, vault_name).await;

    if auth_mode != AuthMode::None {
        let check_path = path.clone();
        let check_roles = default_roles.clone();
        let check_user_roles = user.roles.clone();
        let check_configs = configs.clone();

        let has_access = tokio::task::spawn_blocking(move || {
            let roles = vault_config::resolve_roles(&check_configs, &check_path, &check_roles);
            vault_config::check_access(&check_user_roles, &roles)
        })
        .await
        .unwrap_or(false);

        if !has_access {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let doc_path = path.clone();
    let root = vault_root.clone();
    let vname = vault_name.to_string();
    let embed_user_roles = user.roles.clone();
    let embed_default_roles = default_roles.clone();
    let embed_auth_mode = auth_mode.clone();
    let result = tokio::task::spawn_blocking(move || {
        let raw = std::fs::read_to_string(&full_path).ok()?;
        let (frontmatter, html) = crate::docs::markdown::render_document(
            &raw, &doc_path, &vname, &root,
            &configs, &embed_user_roles, &embed_default_roles, &embed_auth_mode,
        );
        let meta = crate::docs::git_meta::get_file_meta(&root, &doc_path);
        Some(DocResponse {
            content: html,
            frontmatter,
            path: doc_path,
            last_modified: meta.last_modified,
            last_modified_by: meta.last_modified_by,
        })
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(doc) => Json(doc).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn asset(
    user: User,
    AxumPath(path): AxumPath<String>,
    vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    vault_configs: VaultConfigsMap,
    vault_name: &str,
) -> Response {
    if let Err(status) = validate_asset_path(&path) {
        return status.into_response();
    }

    let full_path = vault_root.join(&path);

    let canonical = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let canonical_root = match vault_root.canonicalize() {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if !canonical.starts_with(&canonical_root) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Role check based on parent directory
    if auth_mode != AuthMode::None {
        let parent_dir = std::path::Path::new(&path)
            .parent()
            .map(|p| format!("{}/", p.display()))
            .unwrap_or_default();
        let check_roles = default_roles.clone();
        let check_user_roles = user.roles.clone();
        let configs = get_cached_configs(&vault_configs, vault_name).await;

        let has_access = tokio::task::spawn_blocking(move || {
            let roles = vault_config::resolve_roles(&configs, &parent_dir, &check_roles);
            vault_config::check_access(&check_user_roles, &roles)
        })
        .await
        .unwrap_or(false);

        if !has_access {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    match tokio::fs::read(&full_path).await {
        Ok(bytes) => {
            let content_type = guess_content_type(&path);
            (
                [
                    (axum::http::header::CONTENT_TYPE, content_type),
                    (axum::http::header::CONTENT_SECURITY_POLICY, "script-src 'none'"),
                    (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Serialize)]
pub struct TagInfo {
    pub name: String,
    pub count: usize,
}

pub async fn tags(
    user: User,
    vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    vault_configs: VaultConfigsMap,
    vault_name: &str,
) -> Json<Vec<TagInfo>> {
    let configs = get_cached_configs(&vault_configs, vault_name).await;
    let result = tokio::task::spawn_blocking(move || {
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        collect_tags(
            &vault_root,
            &vault_root,
            &configs,
            &user,
            &auth_mode,
            &default_roles,
            &mut tag_counts,
        );
        let mut tags: Vec<TagInfo> = tag_counts
            .into_iter()
            .map(|(name, count)| TagInfo { name, count })
            .collect();
        tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        tags
    })
    .await
    .unwrap_or_default();

    Json(result)
}

fn collect_tags(
    dir: &Path,
    vault_root: &Path,
    configs: &vault_config::VaultConfigCache,
    user: &User,
    auth_mode: &AuthMode,
    default_roles: &[String],
    tag_counts: &mut HashMap<String, usize>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e.flatten().collect::<Vec<_>>(),
        Err(_) => return,
    };

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let full_path = entry.path();
        let rel_path = full_path
            .strip_prefix(vault_root)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .to_string();

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_dir() {
            let dir_rel = format!("{}/", rel_path);
            if *auth_mode != AuthMode::None {
                let roles = vault_config::resolve_roles(configs, &dir_rel, default_roles);
                if !vault_config::check_access(&user.roles, &roles) {
                    continue;
                }
            }
            collect_tags(&full_path, vault_root, configs, user, auth_mode, default_roles, tag_counts);
        } else if ft.is_file() && name.ends_with(".md") {
            if *auth_mode != AuthMode::None {
                let roles = vault_config::resolve_roles(configs, &rel_path, default_roles);
                if !vault_config::check_access(&user.roles, &roles) {
                    continue;
                }
            }
            if let Ok(raw) = std::fs::read_to_string(&full_path) {
                for tag in crate::docs::markdown::extract_tags(&raw) {
                    *tag_counts.entry(tag).or_insert(0) += 1;
                }
            }
        }
    }
}
