use chrono::Utc;
use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::fs::{create_dir_all, remove_dir_all, remove_file, rename};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Deserialize, Debug)]
pub struct DirConfig {
    pub path: PathBuf,
    pub time_to_archive_hours: u64,
    pub time_to_deletion_hours: u64,
}

#[derive(Debug, PartialEq)]
pub enum FileAction {
    MoveFile { from: PathBuf, to: PathBuf },
    MoveDir { from: PathBuf, to: PathBuf },
    DeleteFile { path: PathBuf },
    DeleteDir { path: PathBuf },
}

impl fmt::Display for FileAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileAction::MoveFile { from, to } => {
                write!(f, "move file {} -> {}", from.display(), to.display())
            }
            FileAction::MoveDir { from, to } => {
                write!(f, "move dir {} -> {}", from.display(), to.display())
            }
            FileAction::DeleteFile { path } => write!(f, "delete file {}", path.display()),
            FileAction::DeleteDir { path } => write!(f, "delete dir {}", path.display()),
        }
    }
}

pub struct DirEntryWithAge {
    pub path: String,
    pub seconds_since_modification: u64,
    pub is_dir: bool,
}

fn plan_archive_actions(
    archive_path: &Path,
    entries: Vec<DirEntryWithAge>,
    cutoff_secs: u64,
) -> Vec<FileAction> {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");

    entries
        .into_iter()
        .filter(|e| e.seconds_since_modification >= cutoff_secs)
        .map(|entry| {
            let source = PathBuf::from(&entry.path);
            let filename = source
                .file_name()
                .expect("entry should have a filename")
                .to_string_lossy();
            let new_name = format!("{}.{}.bak", filename, timestamp);
            let target = archive_path.join(&new_name);

            if entry.is_dir {
                FileAction::MoveDir {
                    from: source,
                    to: target,
                }
            } else {
                FileAction::MoveFile {
                    from: source,
                    to: target,
                }
            }
        })
        .collect()
}

fn plan_delete_actions(entries: Vec<DirEntryWithAge>, cutoff_secs: u64) -> Vec<FileAction> {
    entries
        .into_iter()
        .filter(|e| e.seconds_since_modification >= cutoff_secs)
        .map(|entry| {
            let path = PathBuf::from(&entry.path);
            if entry.is_dir {
                FileAction::DeleteDir { path }
            } else {
                FileAction::DeleteFile { path }
            }
        })
        .collect()
}

pub fn plan_declutter(cfg: &DirConfig) -> Result<Vec<FileAction>, Box<dyn Error>> {
    let archive_name = ".duansheli-archive";
    let archive_path = cfg.path.join(archive_name);
    let archive_cutoff = cfg.time_to_archive_hours * 3600;
    let delete_cutoff = cfg.time_to_deletion_hours * 3600;

    let root_entries = list_dir_with_meta(&cfg.path, Some(archive_name))?;

    let (to_delete, to_archive): (Vec<_>, Vec<_>) = root_entries
        .into_iter()
        .filter(|e| e.seconds_since_modification >= archive_cutoff)
        .partition(|e| e.seconds_since_modification >= delete_cutoff);

    let mut actions = plan_delete_actions(to_delete, delete_cutoff);
    actions.extend(plan_archive_actions(&archive_path, to_archive, archive_cutoff));

    // Delete existing archive entries that exceed deletion cutoff
    let archive_entries = list_dir_with_meta(&archive_path, None)?;
    actions.extend(plan_delete_actions(archive_entries, delete_cutoff));

    Ok(actions)
}

pub fn execute_actions(actions: &[FileAction]) -> Result<(), Box<dyn Error>> {
    for action in actions {
        match action {
            FileAction::MoveFile { from, to } | FileAction::MoveDir { from, to } => {
                log::info!("Moving {} -> {}", from.display(), to.display());
                rename(from, to)?;
            }
            FileAction::DeleteFile { path } => {
                log::info!("Removing file {}", path.display());
                remove_file(path)?;
            }
            FileAction::DeleteDir { path } => {
                log::info!("Removing dir {} and all its contents", path.display());
                remove_dir_all(path)?;
            }
        }
    }
    Ok(())
}

pub fn declutter_directory(cfg: DirConfig, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let archive_path = cfg.path.join(".duansheli-archive");
    create_dir_all(&archive_path)?;

    let actions = plan_declutter(&cfg)?;

    if dry_run {
        for action in &actions {
            log::info!("[dry-run] {}", action);
        }
    } else {
        execute_actions(&actions)?;
    }

    Ok(())
}

pub fn list_dir_with_meta(
    dir: &Path,
    exclude_recursive: Option<&str>,
) -> Result<Vec<DirEntryWithAge>, Box<dyn Error>> {
    if !dir.is_dir() {
        let err = Err("Directory does not exist".into());
        return err;
    }

    let entries: Vec<DirEntryWithAge> = dir
        .read_dir()?
        .filter_map(|entry_result| {
            let entry = entry_result
                .inspect_err(|e| log::warn!("Error reading entry: {}", e))
                .ok()?;

            if exclude_recursive.is_some_and(|x| x == entry.file_name()) {
                log::debug!("Excluding: {:?}", entry.path());
                return None;
            }

            let meta = entry
                .metadata()
                .inspect_err(|e| log::warn!("Error reading metadata: {}", e))
                .ok()?;

            let modified = meta
                .modified()
                .inspect_err(|e| log::warn!("Error getting modification date: {}", e))
                .ok()?;

            let seconds_since_modification = SystemTime::now()
                .duration_since(modified)
                .inspect_err(|e| log::warn!("Error getting time since modification: {}", e))
                .ok()?
                .as_secs();

            Some(DirEntryWithAge {
                path: entry.path().to_string_lossy().into_owned(),
                seconds_since_modification,
                is_dir: meta.is_dir(),
            })
        })
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(path: &str, age_secs: u64, is_dir: bool) -> DirEntryWithAge {
        DirEntryWithAge {
            path: path.to_string(),
            seconds_since_modification: age_secs,
            is_dir,
        }
    }

    #[test]
    fn test_plan_archive_actions_maps_old_entries() {
        let archive = PathBuf::from("/tmp/archive");
        let cutoff = 3600;
        let entries = vec![
            make_entry("/tmp/root/old_file.txt", 7200, false),
            make_entry("/tmp/root/old_dir", 7200, true),
        ];

        let actions = plan_archive_actions(&archive, entries, cutoff);

        assert_eq!(actions.len(), 2);
        match &actions[0] {
            FileAction::MoveFile { from, to } => {
                assert_eq!(from, &PathBuf::from("/tmp/root/old_file.txt"));
                assert!(to.starts_with("/tmp/archive/"));
                assert!(to.to_string_lossy().ends_with(".bak"));
                assert!(to.to_string_lossy().contains("old_file.txt."));
            }
            other => panic!("expected MoveFile, got {:?}", other),
        }
        match &actions[1] {
            FileAction::MoveDir { from, to } => {
                assert_eq!(from, &PathBuf::from("/tmp/root/old_dir"));
                assert!(to.starts_with("/tmp/archive/"));
                assert!(to.to_string_lossy().ends_with(".bak"));
                assert!(to.to_string_lossy().contains("old_dir."));
            }
            other => panic!("expected MoveDir, got {:?}", other),
        }
    }

    #[test]
    fn test_plan_archive_actions_skips_young_entries() {
        let archive = PathBuf::from("/tmp/archive");
        let cutoff = 3600;
        let entries = vec![
            make_entry("/tmp/root/young_file.txt", 100, false),
            make_entry("/tmp/root/young_dir", 500, true),
        ];

        let actions = plan_archive_actions(&archive, entries, cutoff);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_plan_delete_actions_maps_old_entries() {
        let cutoff = 3600;
        let entries = vec![
            make_entry("/tmp/archive/old_file.bak", 7200, false),
            make_entry("/tmp/archive/old_dir.bak", 7200, true),
        ];

        let actions = plan_delete_actions(entries, cutoff);

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0],
            FileAction::DeleteFile {
                path: PathBuf::from("/tmp/archive/old_file.bak")
            }
        );
        assert_eq!(
            actions[1],
            FileAction::DeleteDir {
                path: PathBuf::from("/tmp/archive/old_dir.bak")
            }
        );
    }

    #[test]
    fn test_plan_delete_actions_skips_young_entries() {
        let cutoff = 3600;
        let entries = vec![
            make_entry("/tmp/archive/young_file.bak", 100, false),
            make_entry("/tmp/archive/young_dir.bak", 500, true),
        ];

        let actions = plan_delete_actions(entries, cutoff);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_display_file_action() {
        let action = FileAction::MoveFile {
            from: PathBuf::from("/a/b.txt"),
            to: PathBuf::from("/c/d.txt"),
        };
        assert_eq!(format!("{}", action), "move file /a/b.txt -> /c/d.txt");

        let action = FileAction::DeleteDir {
            path: PathBuf::from("/x/y"),
        };
        assert_eq!(format!("{}", action), "delete dir /x/y");
    }
}
