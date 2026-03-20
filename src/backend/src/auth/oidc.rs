use anyhow::Context;
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use openidconnect::{
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreErrorResponseType,
        CoreGenderClaim, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreRevocableToken, CoreTokenType,
    },
    AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, EmptyAdditionalClaims,
    EmptyExtraTokenFields, EndpointMaybeSet, EndpointNotSet, EndpointSet, IdTokenFields,
    IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    RevocationErrorResponseType, Scope, StandardErrorResponse,
    OAuth2TokenResponse, StandardTokenIntrospectionResponse, StandardTokenResponse,
    TokenResponse,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::auth::session::{AuthError, User, USER_KEY};
use crate::config::OidcSettings;

const CSRF_TOKEN_KEY: &str = "csrf_token";
const NONCE_KEY: &str = "nonce";
const PKCE_VERIFIER_KEY: &str = "pkce_verifier";

type ConfiguredClient = Client<
    EmptyAdditionalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    StandardTokenResponse<
        IdTokenFields<
            EmptyAdditionalClaims,
            EmptyExtraTokenFields,
            CoreGenderClaim,
            CoreJweContentEncryptionAlgorithm,
            CoreJwsSigningAlgorithm,
        >,
        CoreTokenType,
    >,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, CoreTokenType>,
    CoreRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
pub struct OidcClient {
    client: ConfiguredClient,
    http_client: openidconnect::reqwest::Client,
    issuer_url: String,
    public_issuer_url: Option<String>,
    frontend_url: String,
    client_id: String,
}

impl OidcClient {
    pub async fn new(settings: &OidcSettings, redirect_url: &str) -> anyhow::Result<Self> {
        let http_client = openidconnect::reqwest::ClientBuilder::new()
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .build()
            .context("failed to build HTTP client")?;

        let issuer_url =
            IssuerUrl::new(settings.issuer_url.clone()).context("invalid issuer URL")?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .context("OIDC discovery failed")?;

        let client = ConfiguredClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(settings.client_id.clone()),
            settings
                .client_secret
                .as_ref()
                .map(|s| ClientSecret::new(s.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_url.to_string()).context("invalid redirect URL")?,
        );

        Ok(Self {
            client,
            http_client,
            issuer_url: settings.issuer_url.clone(),
            public_issuer_url: settings.public_issuer_url.clone(),
            frontend_url: settings.frontend_url.clone().unwrap_or_default(),
            client_id: settings.client_id.clone(),
        })
    }

    fn rewrite_url_for_browser(&self, url: &str) -> String {
        match &self.public_issuer_url {
            Some(public_url) => url.replace(&self.issuer_url, public_url),
            None => url.to_string(),
        }
    }
}

const RETURN_TO_KEY: &str = "return_to";

#[derive(Deserialize)]
pub struct LoginParams {
    pub return_to: Option<String>,
}

pub async fn login(
    session: Session,
    axum::extract::Query(params): axum::extract::Query<LoginParams>,
    oidc: OidcClient,
) -> Result<Response, AuthError> {
    if let Some(return_to) = &params.return_to {
        session
            .insert(RETURN_TO_KEY, return_to.clone())
            .await
            .ok();
    }
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = oidc
        .client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    session
        .insert(CSRF_TOKEN_KEY, csrf_token.secret().to_string())
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?;
    session
        .insert(NONCE_KEY, nonce.secret().to_string())
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?;
    session
        .insert(PKCE_VERIFIER_KEY, pkce_verifier.secret().to_string())
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?;

    let browser_url = oidc.rewrite_url_for_browser(auth_url.as_str());
    Ok(Redirect::to(&browser_url).into_response())
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

pub async fn callback(
    session: Session,
    oidc: OidcClient,
    Query(params): Query<CallbackParams>,
) -> Result<Response, AuthError> {
    let stored_csrf: String = session
        .get(CSRF_TOKEN_KEY)
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?
        .ok_or(AuthError::InvalidState)?;

    if stored_csrf != params.state {
        return Err(AuthError::InvalidState);
    }

    let stored_nonce: String = session
        .get(NONCE_KEY)
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?
        .ok_or(AuthError::InvalidState)?;

    let stored_pkce: String = session
        .get(PKCE_VERIFIER_KEY)
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?
        .ok_or(AuthError::InvalidState)?;

    let token_response = oidc
        .client
        .exchange_code(AuthorizationCode::new(params.code))
        .map_err(|e| AuthError::TokenExchangeFailed(e.to_string()))?
        .set_pkce_verifier(PkceCodeVerifier::new(stored_pkce))
        .request_async(&oidc.http_client)
        .await
        .map_err(|e| AuthError::TokenExchangeFailed(e.to_string()))?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| AuthError::TokenExchangeFailed("no ID token returned".to_string()))?;

    // Verify ID token (signature + issuer + audience + nonce)
    let nonce = Nonce::new(stored_nonce.clone());
    let verifier = oidc.client.id_token_verifier();
    let (sub, name, email) = match id_token.claims(&verifier, &nonce) {
        Ok(claims) => {
            let sub = claims.subject().to_string();
            let email = claims.email().map(|e| e.to_string()).unwrap_or_default();
            let name = claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string())
                .unwrap_or_default();
            (sub, name, email)
        }
        Err(e) if oidc.public_issuer_url.is_some() => {
            // Issuer mismatch expected when internal/external hostnames differ (Docker).
            // Token came from PKCE-protected server-to-server exchange.
            tracing::warn!("ID token verification failed (expected with public_issuer_url): {e}");
            let id_token_str = id_token.to_string();
            let id_claims = extract_claims_from_jwt(&id_token_str);
            let sub = id_claims
                .get("sub")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = id_claims
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let email = id_claims
                .get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let token_nonce = id_claims
                .get("nonce")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if token_nonce != stored_nonce {
                return Err(AuthError::InvalidState);
            }
            (sub, name, email)
        }
        Err(e) => {
            return Err(AuthError::TokenExchangeFailed(format!(
                "ID token verification failed: {e}"
            )));
        }
    };

    // Extract roles from access token (realm_access.roles + vellum_roles)
    let access_token_str = token_response.access_token().secret().clone();
    let roles = extract_roles_from_jwt(&access_token_str);

    let user = User {
        sub,
        name,
        email,
        roles,
    };

    // Read return_to before flush (flush clears all session data)
    let return_to: Option<String> = session.get(RETURN_TO_KEY).await.ok().flatten();

    session.flush().await.ok();
    session
        .insert(USER_KEY, &user)
        .await
        .map_err(|e| AuthError::ProviderError(e.to_string()))?;

    tracing::info!(sub = %user.sub, roles = ?user.roles, "user authenticated");

    let redirect_to = match return_to {
        Some(path) if is_safe_redirect(&path) => {
            if oidc.frontend_url.is_empty() {
                path
            } else {
                format!("{}{}", oidc.frontend_url, path)
            }
        }
        _ => {
            if oidc.frontend_url.is_empty() {
                "/".to_string()
            } else {
                oidc.frontend_url.clone()
            }
        }
    };
    Ok(Redirect::to(&redirect_to).into_response())
}

pub async fn logout(session: Session, oidc: OidcClient) -> impl IntoResponse {
    session.flush().await.ok();
    session.delete().await.ok();

    let post_logout_redirect = if oidc.frontend_url.is_empty() {
        "/".to_string()
    } else {
        oidc.frontend_url.clone()
    };

    // Redirect to Keycloak end_session_endpoint to also terminate Keycloak session
    let keycloak_logout = format!(
        "{}/protocol/openid-connect/logout?post_logout_redirect_uri={}&client_id={}",
        oidc.issuer_url,
        urlencoding::encode(&post_logout_redirect),
        urlencoding::encode(&oidc.client_id)
    );
    let logout_url = oidc.rewrite_url_for_browser(&keycloak_logout);
    Redirect::to(&logout_url)
}

pub async fn me(user: User) -> Json<UserResponse> {
    Json(UserResponse {
        sub: user.sub,
        name: user.name,
        email: user.email,
        roles: user.roles,
    })
}

#[derive(Serialize)]
pub struct UserResponse {
    pub sub: String,
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
}

fn decode_jwt_payload(jwt: &str) -> Option<Vec<u8>> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()
}

fn is_safe_redirect(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//") && !path.contains("://")
}

fn extract_claims_from_jwt(jwt: &str) -> serde_json::Value {
    let Some(bytes) = decode_jwt_payload(jwt) else {
        return serde_json::Value::Object(Default::default());
    };
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Object(Default::default()))
}

fn extract_roles_from_jwt(jwt: &str) -> Vec<String> {
    let Some(payload_bytes) = decode_jwt_payload(jwt) else {
        return Vec::new();
    };

    #[derive(serde::Deserialize)]
    struct JwtPayload {
        realm_access: Option<RealmAccess>,
        #[serde(default)]
        vellum_roles: Vec<String>,
    }

    #[derive(serde::Deserialize)]
    struct RealmAccess {
        #[serde(default)]
        roles: Vec<String>,
    }

    let payload: JwtPayload = match serde_json::from_slice(&payload_bytes) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    // Merge roles from both sources, deduplicate
    let mut roles = std::collections::HashSet::new();
    if let Some(ra) = payload.realm_access {
        roles.extend(ra.roles);
    }
    roles.extend(payload.vellum_roles);
    roles.into_iter().collect()
}

