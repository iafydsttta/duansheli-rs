use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

use duansheli::*;

fn create_file_fixture(dir: &std::path::Path, name: &str, age_secs: u64) {
    create_fixture(dir, name, age_secs, false);
}

fn create_dir_fixture(dir: &std::path::Path, name: &str, age_secs: u64) {
    create_fixture(dir, name, age_secs, true);
}
/// Create a file or directory at `dir/{name}` and backdate its mtime by `age_secs`.
fn create_fixture(dir: &std::path::Path, name: &str, age_secs: u64, is_dir: bool) {
    let path = dir.join(name);
    if is_dir {
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("f_child.txt"), "x").unwrap();
    } else {
        fs::write(&path, "content").unwrap();
    }
    let new_mtime = SystemTime::now() - Duration::from_secs(age_secs);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(new_mtime)).unwrap();
}

/// Build a temp directory with a mix of old/medium/young files and directories.
///
/// ```text
/// {root}/
/// ├── f_old.txt               (exceeds_deletion_secs)
/// ├── f_medium.txt            (exceeds_archive_secs)
/// ├── f_young.txt             (below_thresholds_secs)
/// ├── D_OLD/                  (exceeds_deletion_secs)
/// │   └── f_child.txt
/// ├── D_MEDIUM/               (exceeds_archive_secs)
/// │   └── f_child.txt
/// ├── D_YOUNG/                (below_thresholds_secs)
/// │   └── f_child.txt
/// └── D_OLD_NESTING/          (exceeds_deletion_secs)
///     ├── f_child.txt
///     ├── f_inner.txt
///     └── D_INNER/
///         └── f_child.txt
/// ```
///
/// `exceeds_deletion_secs` — mtime age for entries that should exceed the deletion threshold
/// `exceeds_archive_secs`  — mtime age for entries that should exceed the archive threshold but not deletion
/// `below_thresholds_secs` — mtime age for entries that should stay below both thresholds
fn create_test_directory(
    exceeds_deletion_secs: u64,
    exceeds_archive_secs: u64,
    below_thresholds_secs: u64,
) -> TempDir {
    let tmp_dir = TempDir::new().unwrap();
    let root = tmp_dir.path();

    // Top-level entries
    create_file_fixture(root, "f_old.txt", exceeds_deletion_secs);
    create_file_fixture(root, "f_medium.txt", exceeds_archive_secs);
    create_file_fixture(root, "f_young.txt", below_thresholds_secs);
    create_dir_fixture(root, "D_OLD", exceeds_deletion_secs);
    create_dir_fixture(root, "D_MEDIUM", exceeds_archive_secs);
    create_dir_fixture(root, "D_YOUNG", below_thresholds_secs);

    // D_OLD_NESTING/ — a dir with extra nested content
    create_dir_fixture(root, "D_OLD_NESTING", exceeds_deletion_secs);
    let deep_dir = root.join("D_OLD_NESTING");
    create_file_fixture(&deep_dir, "f_inner.txt", exceeds_deletion_secs);
    create_dir_fixture(&deep_dir, "D_INNER", exceeds_deletion_secs);
    // re-backdate after adding children (they update the dir's mtime)
    let old_mtime = SystemTime::now() - Duration::from_secs(exceeds_deletion_secs);
    filetime::set_file_mtime(&deep_dir, filetime::FileTime::from_system_time(old_mtime)).unwrap();

    tmp_dir
}

#[test]
fn test_directory_archival() {
    // arrange
    let time_to_archive_hours: u64 = 1;
    let time_to_deletion_hours: u64 = 999;

    let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;
    let tmp_dir = create_test_directory(exceeds_archive_secs, exceeds_archive_secs, 0);
    let root = tmp_dir.path();
    let archive = root.join(".duansheli-archive");

    // debug
    debug_print_tree_with_timestamps(root);

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
    };

    // act
    declutter_directory(cfg, false).unwrap();

    // assert — old entries moved to archive
    assert!(!root.join("f_old.txt").exists(), "old file should be archived");
    assert!(!root.join("D_OLD").exists(), "old dir should be archived");
    assert!(!root.join("D_OLD_NESTING").exists(), "old nested dir should be archived");
    assert!(!root.join("f_medium.txt").exists(), "medium file should be archived");
    assert!(!root.join("D_MEDIUM").exists(), "medium dir should be archived");

    // young entries remain untouched
    assert!(root.join("f_young.txt").exists(), "young file should remain");
    assert!(root.join("D_YOUNG").exists(), "young dir should remain");

    // archive should contain all moved entries (with .bak suffix)
    let archived: Vec<_> = fs::read_dir(&archive)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(archived.len(), 5, "archive should contain 5 entries");
    assert!(archived.iter().any(|e| e.file_name().to_string_lossy().starts_with("f_old.txt.")));
    assert!(archived.iter().any(|e| e.file_name().to_string_lossy().starts_with("D_OLD.")));
    assert!(archived.iter().any(|e| e.file_name().to_string_lossy().starts_with("D_OLD_NESTING.")));
    assert!(archived.iter().any(|e| e.file_name().to_string_lossy().starts_with("f_medium.txt.")));
    assert!(archived.iter().any(|e| e.file_name().to_string_lossy().starts_with("D_MEDIUM.")));
}

#[test]
fn test_directory_archival_dry_run() {
    let time_to_archive_hours: u64 = 1;
    let time_to_deletion_hours: u64 = 999;

    let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;
    let tmp_dir = create_test_directory(exceeds_archive_secs, exceeds_archive_secs, 0);
    let root = tmp_dir.path();
    let archive = root.join(".duansheli-archive");

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
    };

    declutter_directory(cfg, true).unwrap();

    // all entries should remain in place
    assert!(root.join("f_old.txt").exists(), "old file should still exist");
    assert!(root.join("D_OLD").exists(), "old dir should still exist");
    assert!(root.join("D_OLD_NESTING").exists(), "old nested dir should still exist");
    assert!(root.join("f_medium.txt").exists(), "medium file should still exist");
    assert!(root.join("D_MEDIUM").exists(), "medium dir should still exist");
    assert!(root.join("f_young.txt").exists(), "young file should still exist");
    assert!(root.join("D_YOUNG").exists(), "young dir should still exist");

    // archive exists but should be empty
    assert!(archive.is_dir(), "archive directory should exist");
    let archived: Vec<_> = fs::read_dir(&archive).unwrap().filter_map(|e| e.ok()).collect();
    assert_eq!(archived.len(), 0, "archive should be empty in dry-run");
}

#[test]
fn test_permanent_deletion_dry_run() {
    let time_to_archive_hours: u64 = 1;
    let time_to_deletion_hours: u64 = 2;
    let exceeds_deletion_secs = (time_to_deletion_hours * 3600) + 1;
    let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;

    let tmp_dir = create_test_directory(exceeds_deletion_secs, exceeds_archive_secs, 0);
    let root = tmp_dir.path();

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
    };

    declutter_directory(cfg, true).unwrap();

    // all entries should remain in place
    assert!(root.join("f_old.txt").exists(), "old file should still exist");
    assert!(root.join("D_OLD").exists(), "old dir should still exist");
    assert!(root.join("D_OLD_NESTING").exists(), "old nested dir should still exist");
    assert!(root.join("f_medium.txt").exists(), "medium file should still exist");
    assert!(root.join("D_MEDIUM").exists(), "medium dir should still exist");
    assert!(root.join("f_young.txt").exists(), "young file should still exist");
    assert!(root.join("D_YOUNG").exists(), "young dir should still exist");
}

#[test]
fn test_declutter_rejects_dangerous_path() {
    let cfg = DirConfig {
        path: std::path::PathBuf::from("/"),
        time_to_archive_hours: 1,
        time_to_deletion_hours: 2,
    };
    let result = declutter_directory(cfg, true);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("dangerous path"), "expected dangerous path error, got: {}", err_msg);
}

#[test]
fn test_ignored_files_survive_declutter() {
    let time_to_archive_hours: u64 = 1;
    let time_to_deletion_hours: u64 = 2;
    let exceeds_deletion_secs = (time_to_deletion_hours * 3600) + 1;

    let tmp_dir = TempDir::new().unwrap();
    let root = tmp_dir.path();

    // Create metadata files with old mtimes
    create_file_fixture(root, ".DS_Store", exceeds_deletion_secs);
    create_file_fixture(root, "Thumbs.db", exceeds_deletion_secs);
    // Also create a normal old file to confirm it does get processed
    create_file_fixture(root, "old_file.txt", exceeds_deletion_secs);

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
    };

    declutter_directory(cfg, false).unwrap();

    // Metadata files should survive
    assert!(root.join(".DS_Store").exists(), ".DS_Store should be ignored and survive");
    assert!(root.join("Thumbs.db").exists(), "Thumbs.db should be ignored and survive");

    // Normal old file should be gone (deleted, since it exceeds deletion threshold)
    assert!(!root.join("old_file.txt").exists(), "old_file.txt should have been deleted");

    // Metadata files should NOT be in the archive
    let archive = root.join(".duansheli-archive");
    if archive.exists() {
        let archived: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        for entry in &archived {
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(!name.starts_with(".DS_Store"), ".DS_Store should not be in archive");
            assert!(!name.starts_with("Thumbs.db"), "Thumbs.db should not be in archive");
        }
    }
}

fn debug_print_tree_with_timestamps(root: &std::path::Path) {
    let tree_output = std::process::Command::new("tree")
        .arg("-D")
        .arg(root)
        .output()
        .unwrap();
    println!("Tree:\n{}", String::from_utf8(tree_output.stdout).unwrap());
}

#[test]
fn test_permanent_deletion() {
    // arrange
    let time_to_archive_hours: u64 = 1;
    let time_to_deletion_hours: u64 = 2;
    let exceeds_deletion_secs = (time_to_deletion_hours * 3600) + 1;
    let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;

    let tmp_dir = create_test_directory(exceeds_deletion_secs, exceeds_archive_secs, 0);
    let root = tmp_dir.path();
    let archive = root.join(".duansheli-archive");

    // debug output
    debug_print_tree_with_timestamps(root);

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
    };

    // act
    declutter_directory(cfg, false).unwrap();

    // assert — all old and medium entries removed from root
    assert!(!root.join("f_old.txt").exists(), "old file should leave root");
    assert!(!root.join("D_OLD").exists(), "old dir should leave root");
    assert!(!root.join("D_OLD_NESTING").exists(), "old nested dir should leave root");
    assert!(!root.join("f_medium.txt").exists(), "medium file should leave root");
    assert!(!root.join("D_MEDIUM").exists(), "medium dir should leave root");

    // young entries untouched
    assert!(root.join("f_young.txt").exists(), "young file should remain");
    assert!(root.join("D_YOUNG").exists(), "young dir should remain");

    // archive: medium entries survive, old entries permanently deleted
    assert!(archive.is_dir(), "archive directory should exist");
    let remaining: Vec<_> = fs::read_dir(&archive)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        remaining.len(),
        2,
        "only medium entries should survive in archive, but found: {:?}",
        remaining.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
    assert!(remaining.iter().any(|e| e.file_name().to_string_lossy().starts_with("f_medium.txt.")));
    assert!(remaining.iter().any(|e| e.file_name().to_string_lossy().starts_with("D_MEDIUM.")));
}