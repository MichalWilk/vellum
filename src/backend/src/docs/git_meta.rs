use std::path::Path;

pub struct FileMeta {
    pub last_modified: Option<String>,
    pub last_modified_by: Option<String>,
}

/// Try git history first, fall back to filesystem metadata.
pub fn get_file_meta(vault_root: &Path, file_path: &str) -> FileMeta {
    if let Some(meta) = get_git_meta(vault_root, file_path) {
        return meta;
    }
    get_fs_meta(vault_root, file_path)
}

fn get_git_meta(vault_root: &Path, file_path: &str) -> Option<FileMeta> {
    let repo = git2::Repository::discover(vault_root).ok()?;
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.push_head().ok()?;
    revwalk.set_sorting(git2::Sort::TIME).ok()?;

    let rel_path = Path::new(file_path);

    for oid in revwalk {
        let oid = oid.ok()?;
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;

        let current_blob_id = tree.get_path(rel_path).ok().map(|entry| entry.id());

        // File doesn't exist at this commit - skip.
        let Some(blob_id) = current_blob_id else {
            continue;
        };

        if commit.parent_count() == 0 {
            // Initial commit introduced the file.
            return Some(commit_to_meta(&commit));
        }

        let parent = commit.parent(0).ok()?;
        let parent_tree = parent.tree().ok()?;
        let parent_blob_id = parent_tree.get_path(rel_path).ok().map(|e| e.id());

        if Some(blob_id) != parent_blob_id {
            return Some(commit_to_meta(&commit));
        }
    }

    // File exists at HEAD but was never "modified" per our diff logic.
    // This shouldn't happen if HEAD has at least one commit, but handle gracefully.
    None
}

fn commit_to_meta(commit: &git2::Commit<'_>) -> FileMeta {
    let git_time = commit.time();
    let seconds = git_time.seconds();
    let offset_minutes = git_time.offset_minutes();

    let last_modified = unix_to_rfc3339(seconds, offset_minutes);
    let last_modified_by = commit.author().name().map(String::from);

    FileMeta {
        last_modified: Some(last_modified),
        last_modified_by,
    }
}

fn get_fs_meta(vault_root: &Path, file_path: &str) -> FileMeta {
    let full_path = vault_root.join(file_path);
    let last_modified = std::fs::metadata(&full_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
            Some(unix_to_rfc3339(duration.as_secs() as i64, 0))
        });

    FileMeta {
        last_modified,
        last_modified_by: None,
    }
}

fn unix_to_rfc3339(seconds: i64, offset_minutes: i32) -> String {
    // Apply offset to get local time for display, but format with the offset.
    let utc_secs = seconds;
    let total_secs = utc_secs + (offset_minutes as i64) * 60;

    // Break down into date/time components.
    let days_from_epoch = total_secs.div_euclid(86400);
    let time_of_day = total_secs.rem_euclid(86400);

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let secs = time_of_day % 60;

    // Convert days from 1970-01-01 to y/m/d using a civil calendar algorithm.
    let (year, month, day) = days_to_ymd(days_from_epoch);

    let offset_sign = if offset_minutes >= 0 { '+' } else { '-' };
    let abs_offset = offset_minutes.unsigned_abs();
    let off_h = abs_offset / 60;
    let off_m = abs_offset % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
        year, month, day, hours, minutes, secs, offset_sign, off_h, off_m
    )
}

// Civil calendar conversion from days since 1970-01-01.
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}
