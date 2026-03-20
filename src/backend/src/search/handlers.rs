use axum::{
    extract::Query,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::User;
use crate::config::AuthMode;
use crate::docs::vault_config::{self, VaultConfigCache};
use crate::search::index::SearchIndex;

type VaultConfigsMap = Arc<RwLock<HashMap<String, VaultConfigCache>>>;

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_vault")]
    pub vault: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_vault() -> String {
    "docs".to_string()
}

fn default_limit() -> usize {
    20
}

#[derive(Serialize)]
pub struct SearchResultResponse {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

pub async fn search(
    user: User,
    Query(params): Query<SearchParams>,
    search_index: Arc<RwLock<Option<SearchIndex>>>,
    _vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    vault_configs: VaultConfigsMap,
    vault_name: &str,
) -> Response {
    if params.q.is_empty() {
        return Json(Vec::<SearchResultResponse>::new()).into_response();
    }

    let limit = params.limit.min(100);

    let index_guard = search_index.read().await;
    let index = match index_guard.as_ref() {
        Some(idx) => idx,
        None => return Json(Vec::<SearchResultResponse>::new()).into_response(),
    };

    // Fetch extra results to account for post-filter role removal
    let raw_results = index.search(&params.q, limit * 2);
    drop(index_guard);

    if auth_mode == AuthMode::None {
        let results: Vec<SearchResultResponse> = raw_results
            .into_iter()
            .take(limit)
            .map(|r| SearchResultResponse {
                path: r.path,
                title: r.title,
                snippet: r.snippet,
                score: r.score,
            })
            .collect();
        return Json(results).into_response();
    }

    let configs = {
        let guard = vault_configs.read().await;
        guard.get(vault_name).cloned().unwrap_or_default()
    };

    let filtered = tokio::task::spawn_blocking(move || {
        raw_results
            .into_iter()
            .filter(|r| {
                let roles = vault_config::resolve_roles(&configs, &r.path, &default_roles);
                vault_config::check_access(&user.roles, &roles)
            })
            .take(limit)
            .map(|r| SearchResultResponse {
                path: r.path,
                title: r.title,
                snippet: r.snippet,
                score: r.score,
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    Json(filtered).into_response()
}
