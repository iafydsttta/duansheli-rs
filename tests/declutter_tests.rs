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
        fs::write(path.join("child.txt"), "x").unwrap();
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
/// ├── old_file.txt          file   old
/// ├── young_file.txt        file   young
/// ├── old_dir/              dir    old
/// │   └── child.txt
/// ├── young_dir/            dir    young
/// │   └── child.txt
/// └── old_nested/           dir    old
///     ├── child.txt
///     ├── inner_file.txt    file   old
///     └── inner_dir/        dir    old
///         └── child.txt
/// ```
///
/// `old_secs`   — mtime age for entries that should exceed the archive threshold
/// `young_secs` — mtime age for entries that should stay below it
fn create_test_directory(old_secs: u64, young_secs: u64) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Top-level entries
    create_file_fixture(root, "old_file.txt", old_secs);
    create_file_fixture(root, "young_file.txt", young_secs);
    create_dir_fixture(root, "old_dir", old_secs);
    create_dir_fixture(root, "young_dir", young_secs);

    // old_nested/ — a dir with extra nested content
    create_dir_fixture(root, "old_nested", old_secs);
    let nested = root.join("old_nested");
    create_file_fixture(&nested, "inner_file.txt", old_secs);
    create_dir_fixture(&nested, "inner_dir", old_secs);

    tmp
}


#[test]
fn test_old_file_is_archived() {
   let tmp = dbg!(create_test_directory(3600, 60));
}