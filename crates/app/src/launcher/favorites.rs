//! Pure launcher favorites / recents / folders / "new" logic (T265-C).
//!
//! No GPUI types — every function is a deterministic function of its inputs so
//! the model is unit-testable without a window. The view is a thin shell over
//! these.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chronos_services::applications::frecency::{score, FrecencyData};
use chronos_services::AppEntry;

use crate::launcher::launcher_config::Folder;

/// "New" badge threshold: a `.desktop` file younger than this is new (days).
pub const NEW_DAYS: i64 = 7;

/// Move an element from `from` to `to` (indices in the original slice), the
/// standard remove-then-insert reorder used by DnD. A no-op when either index
/// is out of range or they are equal.
pub fn move_item<T: Clone>(items: &[T], from: usize, to: usize) -> Vec<T> {
    if from >= items.len() || to >= items.len() || from == to {
        return items.to_vec();
    }
    let mut out = items.to_vec();
    let item = out.remove(from);
    out.insert(to, item);
    out
}

/// Resolve favorite ids to entries. Unknown ids (app uninstalled / renamed) are
/// silently skipped (T265-C), and `sort_alpha` re-sorts by display name.
pub fn resolve_favorites(
    order: &[String],
    by_id: &HashMap<String, AppEntry>,
    sort_alpha: bool,
) -> Vec<AppEntry> {
    let mut out: Vec<AppEntry> = order.iter().filter_map(|id| by_id.get(id).cloned()).collect();
    if sort_alpha {
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }
    out
}

/// Top-N recently/frequently launched apps (recents section). Apps never
/// launched (weight 0) are excluded, so the section shows real history, not the
/// whole installed set. Ties break by name for determinism.
pub fn top_recents(
    entries: &[AppEntry],
    data: &FrecencyData,
    now: i64,
    limit: usize,
) -> Vec<AppEntry> {
    if limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(f64, &AppEntry)> = entries
        .iter()
        .filter_map(|e| {
            let weight = data.entries.get(&e.id).map(|fe| score(fe, now)).unwrap_or(0.0);
            (weight > 0.0).then(|| (weight, e))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(_, e)| e.clone())
        .collect()
}

/// Whether a `.desktop` mtime (unix seconds) is "new" relative to `now`.
pub fn is_new(mtime: Option<i64>, now: i64, threshold_days: i64) -> bool {
    match mtime {
        Some(m) => now >= m && (now - m) < threshold_days * 86_400,
        None => false,
    }
}

/// Desktop entry directories, mirroring the applications service scan order
/// (system first, then user — the service scans system then user so user
/// overrides; here the first hit wins, same effective result for existence).
pub fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    let user_data = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|home| PathBuf::from(home).join(".local/share"))
                .unwrap_or_default()
        });
    dirs.push(user_data.join("applications"));
    dirs
}

/// mtime (unix seconds) of `<dir>/<id>.desktop`; first existing dir wins.
pub fn desktop_mtime(id: &str, dirs: &[PathBuf]) -> Option<i64> {
    for dir in dirs {
        let path = dir.join(format!("{id}.desktop"));
        if let Ok(meta) = std::fs::metadata(&path)
            && let Ok(modified) = meta.modified()
            && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
        {
            return Some(dur.as_secs() as i64);
        }
    }
    None
}

/// Smallest unused `folder-<n>` id (1-based), deterministic given the folders.
pub fn next_folder_id(folders: &[Folder]) -> String {
    let mut n = 1usize;
    loop {
        let id = format!("folder-{n}");
        if !folders.iter().any(|f| f.id == id) {
            return id;
        }
        n += 1;
    }
}

/// Add an app id to a folder, idempotently. Returns true if it was added.
pub fn folder_add_app(folder: &mut Folder, app_id: &str) -> bool {
    if folder.apps.iter().any(|a| a == app_id) {
        return false;
    }
    folder.apps.push(app_id.to_string());
    true
}

/// Resolve a folder's app ids to entries, skipping unknown ids.
pub fn resolve_folder_apps(folder: &Folder, by_id: &HashMap<String, AppEntry>) -> Vec<AppEntry> {
    folder.apps.iter().filter_map(|id| by_id.get(id).cloned()).collect()
}

/// Convenience for tests: build an id→entry map from a slice.
pub fn index_by_id(entries: &[AppEntry]) -> HashMap<String, AppEntry> {
    entries.iter().map(|e| (e.id.clone(), e.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_services::applications::frecency::{FrecencyData, FrecencyEntry};

    fn e(id: &str, name: &str) -> AppEntry {
        AppEntry::fixture(id, name)
    }

    #[test]
    fn move_item_reorders() {
        let v = vec!["a", "b", "c", "d"];
        assert_eq!(move_item(&v, 0, 2), vec!["b", "c", "a", "d"]);
        assert_eq!(move_item(&v, 3, 0), vec!["d", "a", "b", "c"]);
        assert_eq!(move_item(&v, 1, 1), v, "same index is a no-op");
        assert_eq!(move_item(&v, 9, 0), v, "out-of-range is a no-op");
    }

    #[test]
    fn resolve_favorites_skips_unknown_ids() {
        let by_id = index_by_id(&[e("firefox", "Firefox"), e("kitty", "Kitty")]);
        let order = vec!["firefox".to_string(), "ghost".to_string(), "kitty".to_string()];
        let resolved = resolve_favorites(&order, &by_id, false);
        assert_eq!(
            resolved.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["firefox", "kitty"],
            "unknown id 'ghost' must be silently skipped"
        );
    }

    #[test]
    fn resolve_favorites_sorts_alphabetically_when_requested() {
        let by_id = index_by_id(&[e("z", "Zebra"), e("a", "Alpha"), e("m", "Mike")]);
        let order = vec!["z".to_string(), "m".to_string(), "a".to_string()];
        let resolved = resolve_favorites(&order, &by_id, true);
        assert_eq!(
            resolved.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["a", "m", "z"]
        );
    }

    #[test]
    fn top_recents_is_top_n_by_frecency() {
        let day = 86_400i64;
        let now = 1_700_000_000i64;
        let entries = vec![e("old", "Old"), e("new", "New"), e("never", "Never")];
        let mut data = FrecencyData::default();
        data.entries.insert(
            "old".into(),
            FrecencyEntry {
                launch_count: 10,
                last_launched_at: now - 30 * day,
            },
        );
        data.entries.insert(
            "new".into(),
            FrecencyEntry {
                launch_count: 2,
                last_launched_at: now - day,
            },
        );

        let top = top_recents(&entries, &data, now, 8);
        assert_eq!(
            top.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["new", "old"],
            "recency must dominate; never-launched must be excluded"
        );

        let limited = top_recents(&entries, &data, now, 1);
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, "new");
    }

    #[test]
    fn top_recents_respects_zero_limit() {
        let entries = vec![e("a", "A")];
        assert!(top_recents(&entries, &FrecencyData::default(), 0, 0).is_empty());
    }

    #[test]
    fn is_new_within_threshold() {
        let now = 1_700_000_000i64;
        let day = 86_400i64;
        assert!(is_new(Some(now - day), now, 7));
        assert!(is_new(Some(now), now, 7));
        assert!(!is_new(Some(now - 8 * day), now, 7));
        assert!(!is_new(Some(now + day), now, 7), "future mtime is not new");
        assert!(!is_new(None, now, 7));
    }

    #[test]
    fn desktop_mtime_reads_file_metadata() {
        let dir = std::env::temp_dir().join("launcher-mtime-test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("firefox.desktop");
        std::fs::write(&file, "[Desktop Entry]\nType=Application\nName=Firefox\nExec=x\n").unwrap();

        let mtime = desktop_mtime("firefox", &[dir.clone()]).expect("file must be found");
        let now = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(mtime <= now && now - mtime < 10, "mtime must be ~now");
        assert!(desktop_mtime("missing", &[dir.clone()]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_folder_id_skips_existing() {
        let folders = vec![
            Folder {
                id: "folder-1".into(),
                name: "A".into(),
                apps: vec![],
            },
            Folder {
                id: "folder-3".into(),
                name: "C".into(),
                apps: vec![],
            },
        ];
        assert_eq!(next_folder_id(&folders), "folder-2");
        assert_eq!(next_folder_id(&[]), "folder-1");
    }

    #[test]
    fn folder_add_app_is_idempotent() {
        let mut folder = Folder {
            id: "folder-1".into(),
            name: "Work".into(),
            apps: vec!["code".into()],
        };
        assert!(folder_add_app(&mut folder, "slack"));
        assert!(!folder_add_app(&mut folder, "slack"), "duplicate add must be false");
        assert_eq!(folder.apps, vec!["code", "slack"]);
    }

    #[test]
    fn resolve_folder_apps_skips_unknown() {
        let by_id = index_by_id(&[e("code", "Code")]);
        let folder = Folder {
            id: "f".into(),
            name: "Work".into(),
            apps: vec!["code".into(), "gone".into()],
        };
        let apps = resolve_folder_apps(&folder, &by_id);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].id, "code");
    }
}
