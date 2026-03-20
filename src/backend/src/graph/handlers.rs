use axum::{
    response::{IntoResponse, Response},
    Json,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::User;
use crate::config::AuthMode;
use crate::docs::vault_config::{self, VaultConfigCache};
use crate::graph::builder::{self, GraphData};

type VaultConfigsMap = Arc<RwLock<HashMap<String, VaultConfigCache>>>;

pub async fn graph(
    user: User,
    vault_root: PathBuf,
    auth_mode: AuthMode,
    default_roles: Vec<String>,
    redis_pool: tower_sessions_redis_store::fred::prelude::Pool,
    vault_name: &str,
    vault_configs: VaultConfigsMap,
) -> Response {
    let graph_data = match builder::load_cached(&redis_pool, vault_name).await {
        Some(data) => data,
        None => {
            builder::rebuild_and_cache(vault_root.clone(), redis_pool.clone(), vault_name).await;
            match builder::load_cached(&redis_pool, vault_name).await {
                Some(data) => data,
                None => {
                    return Json(GraphData {
                        nodes: Vec::new(),
                        edges: Vec::new(),
                    })
                    .into_response()
                }
            }
        }
    };

    if auth_mode == AuthMode::None {
        return Json(graph_data).into_response();
    }

    let configs = {
        let guard = vault_configs.read().await;
        guard.get(vault_name).cloned().unwrap_or_default()
    };

    let filtered = tokio::task::spawn_blocking(move || {
        let accessible_nodes: Vec<_> = graph_data
            .nodes
            .into_iter()
            .filter(|node| {
                let roles = vault_config::resolve_roles(&configs, &node.id, &default_roles);
                vault_config::check_access(&user.roles, &roles)
            })
            .collect();

        let accessible_ids: HashSet<&str> =
            accessible_nodes.iter().map(|n| n.id.as_str()).collect();

        let accessible_edges: Vec<_> = graph_data
            .edges
            .into_iter()
            .filter(|e| {
                accessible_ids.contains(e.source.as_str())
                    && accessible_ids.contains(e.target.as_str())
            })
            .collect();

        GraphData {
            nodes: accessible_nodes,
            edges: accessible_edges,
        }
    })
    .await
    .unwrap_or_else(|_| GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
    });

    Json(filtered).into_response()
}
