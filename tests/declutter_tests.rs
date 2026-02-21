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

/// Build a temp directory with a mix of old/young files and directories.
///
/// ```text
/// {root}/
/// ├── f_old.txt
/// ├── f_young.txt
/// ├── D_OLD/
/// │   └── f_child.txt
/// ├── D_YOUNG/
/// │   └── f_child.txt
/// └── D_OLD_NESTING/
///     ├── f_child.txt
///     ├── f_inner.txt
///     └── D_INNER/
///         └── f_child.txt
/// ```
///
/// `old_secs`   — mtime age for entries that should exceed the archive threshold
/// `young_secs` — mtime age for entries that should stay below it
fn create_test_directory(old_secs: u64, young_secs: u64) -> TempDir {
    let tmp_dir = TempDir::new().unwrap();
    let root = tmp_dir.path();

    // Top-level entries
    create_file_fixture(root, "f_old.txt", old_secs);
    create_file_fixture(root, "f_young.txt", young_secs);
    create_dir_fixture(root, "D_OLD", old_secs);
    create_dir_fixture(root, "D_YOUNG", young_secs);

    // D_OLD_NESTING/ — a dir with extra NESTING content
    create_dir_fixture(root, "D_OLD_NESTING", old_secs);
    let deep_dir = root.join("D_OLD_NESTING");
    create_file_fixture(&deep_dir, "f_inner.txt", old_secs);
    create_dir_fixture(&deep_dir, "D_INNER", old_secs);
    // re-backdate after adding children (they update the dir's mtime)
    let old_mtime = SystemTime::now() - Duration::from_secs(old_secs);
    filetime::set_file_mtime(&deep_dir, filetime::FileTime::from_system_time(old_mtime)).unwrap();

    tmp_dir
}

#[test]
fn test_directory_archival() {
    // arrange
    let time_to_archive_hours: u64 = 1;
    let time_to_delete_from_archive_hours: u64 = 999;

    let tmp_dir = dbg!(create_test_directory((time_to_archive_hours * 3600) + 1, 0));
    print!("test dir path{}", tmp_dir.path().to_string_lossy());
    let tree_output = std::process::Command::new("tree")
        .arg("-D")
        .arg(tmp_dir.path())
        .output()
        .unwrap();
    println!("Tree: \n{}", String::from_utf8(tree_output.stdout).unwrap());

    let cfg = DirConfig {
        path: tmp_dir.path().to_path_buf(),
        time_to_archive_hours,
        time_to_delete_from_archive_hours,
    };

    // act
    declutter_directory(cfg).unwrap();

    // assert
    // old top-level entries moved to archive
    let root = tmp_dir.path();
    let archive = root.join(".duansheli-archive");

    assert!(
        !root.join("f_old.txt").exists(),
        "old file should be archived"
    );
    assert!(!root.join("D_OLD").exists(), "old dir should be archived");
    assert!(
        !root.join("D_OLD_NESTING").exists(),
        "old NESTING dir should be archived"
    );

    // young entries remain untouched
    assert!(
        root.join("f_young.txt").exists(),
        "young file should remain"
    );
    assert!(root.join("D_YOUNG").exists(), "young dir should remain");

    // archive should contain the moved entries (with .bak suffix)
    let archived: Vec<_> = fs::read_dir(&archive)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(archived.len(), 3, "archive should contain 3 entries");
    assert!(
        archived
            .iter()
            .any(|e| e.file_name().to_string_lossy().starts_with("f_old.txt."))
    );
    assert!(
        archived
            .iter()
            .any(|e| e.file_name().to_string_lossy().starts_with("D_OLD."))
    );
    assert!(archived.iter().any(|e| {
        e.file_name()
            .to_string_lossy()
            .starts_with("D_OLD_NESTING.")
    }));
}
