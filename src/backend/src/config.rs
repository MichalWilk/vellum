use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub app: AppSettings,
    pub auth: AuthSettings,
    #[serde(default)]
    pub vaults: Vec<VaultEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VaultEntry {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthSettings {
    pub mode: String,
    #[serde(default = "default_roles")]
    pub default_roles: Vec<String>,
    pub webhook_secret: Option<String>,
    #[serde(default)]
    pub oidc: Option<OidcSettings>,
}

fn default_roles() -> Vec<String> {
    vec!["*".to_string()]
}

#[derive(Debug, Deserialize, Clone)]
pub struct OidcSettings {
    pub issuer_url: String,
    pub public_issuer_url: Option<String>,
    pub redirect_url: Option<String>,
    pub frontend_url: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub session: SessionSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionSettings {
    pub redis_url: String,
    pub cookie_name: String,
    pub cookie_secure: bool,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::File::with_name("config").required(true))
            .add_source(config::File::with_name("config.local").required(false))
            .add_source(
                config::Environment::with_prefix("VELLUM")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }

    pub fn resolved_vaults(&self) -> Vec<VaultEntry> {
        if self.vaults.is_empty() {
            vec![VaultEntry {
                name: "docs".to_string(),
                path: "vaults/docs".to_string(),
                description: None,
            }]
        } else {
            self.vaults.clone()
        }
    }

    pub fn auth_mode(&self) -> AuthMode {
        match self.auth.mode.as_str() {
            "none" => AuthMode::None,
            _ => AuthMode::Oidc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMode {
    Oidc,
    None,
}
