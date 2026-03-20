use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct VaultToml {
    pub access: Option<AccessConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccessConfig {
    pub roles: Option<Vec<String>>,
    pub paths: Option<HashMap<String, Vec<String>>>,
}

pub type VaultConfigCache = HashMap<PathBuf, Option<VaultToml>>;

pub fn load_vault_toml(dir: &Path) -> Option<VaultToml> {
    let path = dir.join(".vault.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&content).ok()
}

pub fn preload_vault_configs(vault_root: &Path) -> VaultConfigCache {
    let mut cache = HashMap::new();
    preload_recursive(vault_root, vault_root, &mut cache);
    cache
}

fn preload_recursive(dir: &Path, vault_root: &Path, cache: &mut VaultConfigCache) {
    let config = load_vault_toml(dir);
    cache.insert(dir.strip_prefix(vault_root).unwrap_or(Path::new("")).to_path_buf(), config);

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with('.') {
                        preload_recursive(&entry.path(), vault_root, cache);
                    }
                }
            }
        }
    }
}

pub fn resolve_roles(
    vault_configs: &VaultConfigCache,
    file_path: &str,
    default_roles: &[String],
) -> Vec<String> {
    let path = Path::new(file_path);
    let mut ancestors: Vec<PathBuf> = Vec::new();

    let mut current = path.parent();
    while let Some(p) = current {
        ancestors.push(p.to_path_buf());
        if p == Path::new("") {
            break;
        }
        current = p.parent();
    }
    if ancestors.is_empty() || ancestors.last() != Some(&PathBuf::from("")) {
        ancestors.push(PathBuf::from(""));
    }

    // Check access.paths from nearest ancestor to root
    for ancestor in &ancestors {
        if let Some(Some(vault_toml)) = vault_configs.get(ancestor) {
            if let Some(access) = &vault_toml.access {
                if let Some(paths) = &access.paths {
                    let rel_path = if ancestor == &PathBuf::from("") {
                        file_path.to_string()
                    } else {
                        let prefix = format!("{}/", ancestor.display());
                        file_path.strip_prefix(&prefix).unwrap_or(file_path).to_string()
                    };

                    // Exact file/dir match
                    if let Some(roles) = paths.get(&rel_path) {
                        return roles.clone();
                    }

                    // Directory prefix matches
                    let rel = Path::new(&rel_path);
                    let mut check = rel.parent();
                    while let Some(p) = check {
                        if p == Path::new("") {
                            break;
                        }
                        let dir_key = format!("{}/", p.display());
                        if let Some(roles) = paths.get(&dir_key) {
                            return roles.clone();
                        }
                        check = p.parent();
                    }
                }
            }
        }
    }

    // Fallback: access.roles from nearest ancestor
    for ancestor in &ancestors {
        if let Some(Some(vault_toml)) = vault_configs.get(ancestor) {
            if let Some(access) = &vault_toml.access {
                if let Some(roles) = &access.roles {
                    return roles.clone();
                }
            }
        }
    }

    default_roles.to_vec()
}

pub fn check_access(user_roles: &[String], required_roles: &[String]) -> bool {
    if required_roles.iter().any(|r| r == "*") {
        return true;
    }
    user_roles.iter().any(|ur| required_roles.contains(ur))
}
