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

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    #[default]
    Content,
    Files,
    Tags,
    Headings,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    #[serde(default = "default_vault")]
    pub vault: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub r#type: SearchType,
    pub tag: Option<String>,
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
    pub result_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_level: Option<u8>,
}

enum RawSearchData {
    Content(Vec<crate::search::index::SearchResult>),
    Files(Vec<crate::search::index::SearchResult>),
    Tags(Vec<crate::search::index::TagSearchResult>),
    Headings(Vec<crate::search::index::HeadingSearchResult>),
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
    let q = params.q.clone().unwrap_or_default();
    let tag = params.tag.clone();
    let search_type = params.r#type.clone();
    let limit = params.limit.min(100);

    let is_tags_type = matches!(search_type, SearchType::Tags);
    if q.is_empty() && tag.is_none() && !is_tags_type {
        return Json(Vec::<SearchResultResponse>::new()).into_response();
    }

    let raw_data = {
        let index_guard = search_index.read().await;
        let index = match index_guard.as_ref() {
            Some(idx) => idx,
            None => return Json(Vec::<SearchResultResponse>::new()).into_response(),
        };

        match search_type {
            SearchType::Content => {
                if q.is_empty() {
                    return Json(Vec::<SearchResultResponse>::new()).into_response();
                }
                RawSearchData::Content(index.search(&q, limit * 2))
            }
            SearchType::Files => {
                if q.is_empty() {
                    return Json(Vec::<SearchResultResponse>::new()).into_response();
                }
                RawSearchData::Files(index.search_files(&q, limit * 2))
            }
            SearchType::Tags => {
                if let Some(ref exact_tag) = tag {
                    RawSearchData::Tags(index.search_by_tag(exact_tag, limit * 2))
                } else if q.is_empty() {
                    RawSearchData::Tags(index.all_tags())
                } else {
                    RawSearchData::Tags(index.search_tags(&q, limit * 2))
                }
            }
            SearchType::Headings => {
                if q.is_empty() {
                    return Json(Vec::<SearchResultResponse>::new()).into_response();
                }
                RawSearchData::Headings(index.search_headings(&q, limit * 2))
            }
        }
    };

    if auth_mode == AuthMode::None {
        let results = tokio::task::spawn_blocking(move || match raw_data {
            RawSearchData::Content(raw) => filter_and_map_content(raw, limit),
            RawSearchData::Files(raw) => filter_and_map_files(raw, limit),
            RawSearchData::Tags(raw) => aggregate_tags(raw, &q, limit),
            RawSearchData::Headings(raw) => expand_headings(raw, &q, limit),
        })
        .await
        .unwrap_or_default();
        return Json(results).into_response();
    }

    let configs = {
        let guard = vault_configs.read().await;
        guard.get(vault_name).cloned().unwrap_or_default()
    };

    let results = tokio::task::spawn_blocking(move || match raw_data {
        RawSearchData::Content(raw) => {
            let filtered = filter_by_access(raw, &user, &configs, &default_roles);
            filter_and_map_content(filtered, limit)
        }
        RawSearchData::Files(raw) => {
            let filtered = filter_by_access(raw, &user, &configs, &default_roles);
            filter_and_map_files(filtered, limit)
        }
        RawSearchData::Tags(raw) => {
            let filtered = filter_tags_by_access(raw, &user, &configs, &default_roles);
            aggregate_tags(filtered, &q, limit)
        }
        RawSearchData::Headings(raw) => {
            let filtered = filter_headings_by_access(raw, &user, &configs, &default_roles);
            expand_headings(filtered, &q, limit)
        }
    })
    .await
    .unwrap_or_default();

    Json(results).into_response()
}

fn check_user_access(
    path: &str,
    user: &User,
    configs: &VaultConfigCache,
    default_roles: &[String],
) -> bool {
    let roles = vault_config::resolve_roles(configs, path, default_roles);
    vault_config::check_access(&user.roles, &roles)
}

fn filter_by_access(
    raw: Vec<crate::search::index::SearchResult>,
    user: &User,
    configs: &VaultConfigCache,
    default_roles: &[String],
) -> Vec<crate::search::index::SearchResult> {
    raw.into_iter()
        .filter(|r| check_user_access(&r.path, user, configs, default_roles))
        .collect()
}

fn filter_tags_by_access(
    raw: Vec<crate::search::index::TagSearchResult>,
    user: &User,
    configs: &VaultConfigCache,
    default_roles: &[String],
) -> Vec<crate::search::index::TagSearchResult> {
    raw.into_iter()
        .filter(|r| check_user_access(&r.path, user, configs, default_roles))
        .collect()
}

fn filter_headings_by_access(
    raw: Vec<crate::search::index::HeadingSearchResult>,
    user: &User,
    configs: &VaultConfigCache,
    default_roles: &[String],
) -> Vec<crate::search::index::HeadingSearchResult> {
    raw.into_iter()
        .filter(|r| check_user_access(&r.path, user, configs, default_roles))
        .collect()
}

fn filter_and_map_content(
    raw: Vec<crate::search::index::SearchResult>,
    limit: usize,
) -> Vec<SearchResultResponse> {
    raw.into_iter()
        .take(limit)
        .map(|r| SearchResultResponse {
            path: r.path,
            title: r.title,
            snippet: r.snippet,
            score: r.score,
            result_type: "content".to_string(),
            anchor: None,
            tag: None,
            doc_count: None,
            heading_level: None,
        })
        .collect()
}

fn filter_and_map_files(
    raw: Vec<crate::search::index::SearchResult>,
    limit: usize,
) -> Vec<SearchResultResponse> {
    raw.into_iter()
        .take(limit)
        .map(|r| SearchResultResponse {
            path: r.path,
            title: r.title,
            snippet: String::new(),
            score: r.score,
            result_type: "file".to_string(),
            anchor: None,
            tag: None,
            doc_count: None,
            heading_level: None,
        })
        .collect()
}

fn aggregate_tags(
    raw: Vec<crate::search::index::TagSearchResult>,
    query: &str,
    limit: usize,
) -> Vec<SearchResultResponse> {
    let mut tag_counts: HashMap<String, u32> = HashMap::new();

    for result in &raw {
        for tag in result.tags.split_whitespace() {
            if query.is_empty() || tag.starts_with(&query.to_lowercase()) {
                *tag_counts.entry(tag.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut tags: Vec<(String, u32)> = tag_counts.into_iter().collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1));

    tags.into_iter()
        .take(limit)
        .map(|(tag, count)| SearchResultResponse {
            path: String::new(),
            title: tag.clone(),
            snippet: String::new(),
            score: count as f32,
            result_type: "tag".to_string(),
            anchor: None,
            tag: Some(tag),
            doc_count: Some(count),
            heading_level: None,
        })
        .collect()
}

fn expand_headings(
    raw: Vec<crate::search::index::HeadingSearchResult>,
    query: &str,
    limit: usize,
) -> Vec<SearchResultResponse> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for result in raw {
        let headings: Vec<crate::docs::markdown::HeadingInfo> =
            serde_json::from_str(&result.headings_data).unwrap_or_default();

        for heading in headings {
            if heading.text.to_lowercase().contains(&query_lower) {
                results.push(SearchResultResponse {
                    path: result.path.clone(),
                    title: heading.text,
                    snippet: result.title.clone(),
                    score: result.score,
                    result_type: "heading".to_string(),
                    anchor: Some(heading.anchor),
                    tag: None,
                    doc_count: None,
                    heading_level: Some(heading.level),
                });
            }
        }
    }

    results.truncate(limit);
    results
}
