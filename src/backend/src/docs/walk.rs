use std::path::Path;

/// Walk all `.md` files in the vault, skipping dotfiles.
/// Calls `visitor(relative_path, absolute_path)` for each `.md` file found.
pub fn walk_vault_files(vault_root: &Path, mut visitor: impl FnMut(&str, &Path)) {
    walk_recursive(vault_root, vault_root, &mut visitor);
}

fn walk_recursive(dir: &Path, vault_root: &Path, visitor: &mut impl FnMut(&str, &Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_dir() {
            walk_recursive(&path, vault_root, visitor);
        } else if ft.is_file() && name.ends_with(".md") {
            let rel = path
                .strip_prefix(vault_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            visitor(&rel, &path);
        }
    }
}
