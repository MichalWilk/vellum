mod auth;
mod config;
mod docs;
mod graph;
mod search;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use axum::{
    extract::Path as AxumPath,
    http::HeaderMap,
    middleware as axum_mw,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    fred::prelude::{ClientLike, Config as RedisConfig, Pool as RedisPool},
    RedisStore,
};
use tracing_subscriber::EnvFilter;

use crate::auth::middleware::require_auth;
use crate::auth::oidc::{self, OidcClient};
use crate::auth::User;
use crate::config::{AppConfig, AuthMode};
use crate::docs::vault_config::VaultConfigCache;

type VaultMap = Arc<HashMap<String, PathBuf>>;
type SearchIndexMap = Arc<HashMap<String, Arc<RwLock<Option<search::index::SearchIndex>>>>>;
type VaultConfigsMap = Arc<RwLock<HashMap<String, VaultConfigCache>>>;

#[derive(Serialize, Clone)]
struct VaultInfoResponse {
    name: String,
    description: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.app.log_level)),
        )
        .init();

    let auth_mode = config.auth_mode();
    tracing::info!("auth mode: {:?}", auth_mode);

    let redis_url = config
        .auth
        .oidc
        .as_ref()
        .map(|o| o.session.redis_url.as_str())
        .unwrap_or("redis://localhost:6379");

    let redis_config = RedisConfig::from_url(redis_url)?;
    let redis_pool = RedisPool::new(redis_config, None, None, None, 4)?;
    let _connect_handle = redis_pool.init().await?;

    let redis_pool_shared = redis_pool.clone();
    let session_store = RedisStore::new(redis_pool);
    let cookie_secure = config
        .auth
        .oidc
        .as_ref()
        .map(|o| o.session.cookie_secure)
        .unwrap_or(false);
    let cookie_name = config
        .auth
        .oidc
        .as_ref()
        .map(|o| o.session.cookie_name.clone())
        .unwrap_or_else(|| "vellum_session".to_string());

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cookie_secure)
        .with_name(cookie_name)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(8)));

    // Build vault map
    let resolved_vaults = config.resolved_vaults();
    let mut vault_map_inner = HashMap::new();
    for v in &resolved_vaults {
        vault_map_inner.insert(v.name.clone(), PathBuf::from(&v.path));
    }
    let vault_map: VaultMap = Arc::new(vault_map_inner);

    let default_roles = config.auth.default_roles.clone();
    let webhook_secret = config.auth.webhook_secret.clone().unwrap_or_default();

    // Build vault info for /api/vaults
    let vault_infos: Vec<VaultInfoResponse> = resolved_vaults
        .iter()
        .map(|v| VaultInfoResponse {
            name: v.name.clone(),
            description: v.description.clone(),
        })
        .collect();
    let vault_infos = Arc::new(vault_infos);

    // Preload vault configs at startup (per vault)
    let mut vault_configs_inner = HashMap::new();
    for (name, root) in vault_map.as_ref() {
        let configs = docs::vault_config::preload_vault_configs(root);
        vault_configs_inner.insert(name.clone(), configs);
    }
    let vault_configs: VaultConfigsMap = Arc::new(RwLock::new(vault_configs_inner));

    // Build and cache graph index on startup (per vault)
    for (name, root) in vault_map.as_ref() {
        let pool = redis_pool_shared.clone();
        let root = root.clone();
        let vault_name = name.clone();
        tokio::spawn(async move {
            graph::builder::rebuild_and_cache(root, pool, &vault_name).await;
        });
    }

    // Build search index on startup (per vault)
    let mut search_indexes_inner = HashMap::new();
    for (name, root) in vault_map.as_ref() {
        let idx: Arc<RwLock<Option<search::index::SearchIndex>>> = Arc::new(RwLock::new(None));
        search_indexes_inner.insert(name.clone(), idx.clone());

        let root = root.clone();
        tokio::spawn(async move {
            match tokio::task::spawn_blocking(move || search::index::SearchIndex::build(&root))
                .await
            {
                Ok(Ok(index)) => {
                    *idx.write().await = Some(index);
                    tracing::info!("search index built");
                }
                Ok(Err(e)) => tracing::error!("search index build failed: {e}"),
                Err(e) => tracing::error!("search index task failed: {e}"),
            }
        });
    }
    let search_indexes: SearchIndexMap = Arc::new(search_indexes_inner);

    // Common routes (identical for both auth modes)
    let vault_routes = build_vault_routes(
        vault_map.clone(),
        default_roles.clone(),
        auth_mode.clone(),
        redis_pool_shared.clone(),
        search_indexes.clone(),
        vault_configs.clone(),
    )
    .route("/vaults", get({
        let infos = vault_infos.clone();
        let vmap = vault_map.clone();
        let roles = default_roles.clone();
        let cfgs = vault_configs.clone();
        move |user: User| {
            let infos = infos.clone();
            let vmap = vmap.clone();
            let roles = roles.clone();
            let cfgs = cfgs.clone();
            async move { Json(filter_vaults(&infos, &vmap, &user, &roles, &cfgs).await) }
        }
    }));

    let webhook_route = post({
        let secret = webhook_secret.clone();
        let vmap = vault_map.clone();
        let pool = redis_pool_shared.clone();
        let sidxs = search_indexes.clone();
        let cfgs = vault_configs.clone();
        move |headers: HeaderMap| async move {
            webhook_push_all(headers, secret, vmap, pool, sidxs, cfgs).await
        }
    });

    // Auth-specific routes
    let (auth_routes, app_routes) = match auth_mode {
        AuthMode::Oidc => {
            let oidc_settings = config
                .auth
                .oidc
                .as_ref()
                .expect("auth.oidc config required when mode = oidc");

            let redirect_url = oidc_settings
                .redirect_url
                .clone()
                .unwrap_or_else(|| {
                    format!(
                        "{}://localhost/api/auth/callback",
                        if oidc_settings.session.cookie_secure { "https" } else { "http" }
                    )
                });

            let oidc_client = {
                let mut attempts = 0;
                loop {
                    match OidcClient::new(oidc_settings, &redirect_url).await {
                        Ok(client) => break client,
                        Err(e) => {
                            attempts += 1;
                            if attempts >= 30 {
                                return Err(e);
                            }
                            tracing::warn!("OIDC discovery failed (attempt {attempts}/30), retrying in 2s: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                }
            };
            tracing::info!("OIDC client initialized");

            let auth_routes = Router::new()
                .route(
                    "/login",
                    get({
                        let oidc = oidc_client.clone();
                        move |session, query| oidc::login(session, query, oidc)
                    }),
                )
                .route(
                    "/callback",
                    get({
                        let oidc = oidc_client.clone();
                        move |session, query| oidc::callback(session, oidc, query)
                    }),
                )
                .route("/logout", get({
                    let oidc = oidc_client.clone();
                    move |session| oidc::logout(session, oidc)
                }));

            let protected_routes = Router::new()
                .route("/me", get(oidc::me))
                .layer(axum_mw::from_fn(move |session, req, next| {
                    require_auth(session, AuthMode::Oidc, req, next)
                }));

            let public_routes = vault_routes
                .layer(axum_mw::from_fn(move |session, req, next| {
                    auth::middleware::optional_auth(session, AuthMode::Oidc, req, next)
                }));

            (auth_routes, protected_routes.merge(public_routes))
        }
        AuthMode::None => {
            let auth_routes = Router::new()
                .route(
                    "/login",
                    get(|| async { axum::http::StatusCode::NOT_FOUND }),
                )
                .route(
                    "/callback",
                    get(|| async { axum::http::StatusCode::NOT_FOUND }),
                )
                .route(
                    "/logout",
                    get(|| async { axum::http::StatusCode::NOT_FOUND }),
                );

            let all_routes = Router::new()
                .route("/me", get(oidc::me))
                .merge(vault_routes)
                .layer(axum_mw::from_fn(move |session, req, next| {
                    require_auth(session, AuthMode::None, req, next)
                }));

            (auth_routes, all_routes)
        }
    };

    let app = Router::new()
        .nest("/api/auth", auth_routes)
        .nest("/api", app_routes)
        .route("/api/webhook/push", webhook_route)
        .route("/health", get(|| async { "ok" }))
        .layer(session_layer);

    let addr = format!("{}:{}", config.app.host, config.app.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn filter_vaults(
    infos: &[VaultInfoResponse],
    vault_map: &HashMap<String, PathBuf>,
    user: &User,
    default_roles: &[String],
    vault_configs: &VaultConfigsMap,
) -> Vec<VaultInfoResponse> {
    let configs_guard = vault_configs.read().await;
    infos
        .iter()
        .filter(|info| {
            if vault_map.get(&info.name).is_none() {
                return false;
            }
            let Some(configs) = configs_guard.get(&info.name) else {
                return false;
            };
            let roles = docs::vault_config::resolve_roles(configs, "", default_roles);
            docs::vault_config::check_access(&user.roles, &roles)
        })
        .cloned()
        .collect()
}

fn build_vault_routes(
    vault_map: VaultMap,
    default_roles: Vec<String>,
    auth_mode: AuthMode,
    redis_pool: RedisPool,
    search_indexes: SearchIndexMap,
    vault_configs: VaultConfigsMap,
) -> Router {
    Router::new()
        .route("/tree/{vault}", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let cfgs = vault_configs.clone();
            move |user: User, path: AxumPath<String>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let cfgs = cfgs.clone();
                async move {
                    let vault_name = path.0;
                    match vmap.get(&vault_name) {
                        Some(root) => docs::handlers::tree(user, root.clone(), mode, roles, cfgs, &vault_name).await.into_response(),
                        None => axum::http::StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        }))
        .route("/doc/{vault}/{*path}", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let cfgs = vault_configs.clone();
            move |user: User, path: AxumPath<(String, String)>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let cfgs = cfgs.clone();
                async move {
                    let (vault_name, doc_path) = path.0;
                    match vmap.get(&vault_name) {
                        Some(root) => docs::handlers::doc(user, AxumPath(doc_path), root.clone(), mode, roles, &vault_name, cfgs).await,
                        None => axum::http::StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        }))
        .route("/assets/{vault}/{*path}", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let cfgs = vault_configs.clone();
            move |user: User, path: AxumPath<(String, String)>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let cfgs = cfgs.clone();
                async move {
                    let (vault_name, asset_path) = path.0;
                    match vmap.get(&vault_name) {
                        Some(root) => docs::handlers::asset(user, AxumPath(asset_path), root.clone(), mode, roles, cfgs, &vault_name).await,
                        None => axum::http::StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        }))
        .route("/tags/{vault}", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let cfgs = vault_configs.clone();
            move |user: User, path: AxumPath<String>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let cfgs = cfgs.clone();
                async move {
                    let vault_name = path.0;
                    match vmap.get(&vault_name) {
                        Some(root) => docs::handlers::tags(user, root.clone(), mode, roles, cfgs, &vault_name).await.into_response(),
                        None => axum::http::StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        }))
        .route("/graph/{vault}", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let pool = redis_pool.clone();
            let cfgs = vault_configs.clone();
            move |user: User, path: AxumPath<String>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let pool = pool.clone();
                let cfgs = cfgs.clone();
                async move {
                    let vault_name = path.0;
                    match vmap.get(&vault_name) {
                        Some(root) => graph::handlers::graph(user, root.clone(), mode, roles, pool, &vault_name, cfgs).await,
                        None => axum::http::StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }
        }))
        .route("/search", get({
            let vmap = vault_map.clone();
            let roles = default_roles.clone();
            let mode = auth_mode.clone();
            let sidxs = search_indexes.clone();
            let cfgs = vault_configs.clone();
            move |user: User, query: axum::extract::Query<search::handlers::SearchParams>| {
                let vmap = vmap.clone();
                let roles = roles.clone();
                let mode = mode.clone();
                let sidxs = sidxs.clone();
                let cfgs = cfgs.clone();
                async move {
                    let vault_name = query.vault.clone();
                    match (vmap.get(&vault_name), sidxs.get(&vault_name)) {
                        (Some(root), Some(idx)) => {
                            search::handlers::search(user, query, idx.clone(), root.clone(), mode, roles, cfgs, &vault_name).await
                        }
                        _ => Json(Vec::<search::handlers::SearchResultResponse>::new()).into_response(),
                    }
                }
            }
        }))
}

async fn webhook_push_all(
    headers: HeaderMap,
    webhook_secret: String,
    vault_map: VaultMap,
    redis_pool: RedisPool,
    search_indexes: SearchIndexMap,
    vault_configs: VaultConfigsMap,
) -> Response {
    let provided_secret = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if webhook_secret.is_empty() || !constant_time_eq(provided_secret, &webhook_secret) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }

    // Rebuild vault configs
    let mut new_configs = HashMap::new();
    for (name, root) in vault_map.as_ref() {
        let configs = docs::vault_config::preload_vault_configs(root);
        new_configs.insert(name.clone(), configs);
    }
    *vault_configs.write().await = new_configs;

    for (name, root) in vault_map.as_ref() {
        let pool = redis_pool.clone();
        let graph_root = root.clone();
        let vault_name = name.clone();
        tokio::spawn(async move {
            graph::builder::rebuild_and_cache(graph_root, pool, &vault_name).await;
        });

        if let Some(idx) = search_indexes.get(name) {
            let idx = idx.clone();
            let root = root.clone();
            tokio::spawn(async move {
                match tokio::task::spawn_blocking(move || search::index::SearchIndex::build(&root))
                    .await
                {
                    Ok(Ok(index)) => {
                        *idx.write().await = Some(index);
                        tracing::info!("search index rebuilt");
                    }
                    Ok(Err(e)) => tracing::error!("search index rebuild failed: {e}"),
                    Err(e) => tracing::error!("search index rebuild task failed: {e}"),
                }
            });
        }
    }

    axum::http::StatusCode::OK.into_response()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
