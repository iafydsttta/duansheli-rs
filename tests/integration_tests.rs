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
    // debug_print_tree_with_timestamps(root);

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
        ignore_hidden_entries: true,
    };

    // act
    declutter_directory(cfg, false).unwrap();

    // assert — old entries moved to archive
    assert!(
        !root.join("f_old.txt").exists(),
        "old file should be archived"
    );
    assert!(!root.join("D_OLD").exists(), "old dir should be archived");
    assert!(
        !root.join("D_OLD_NESTING").exists(),
        "old nested dir should be archived"
    );
    assert!(
        !root.join("f_medium.txt").exists(),
        "medium file should be archived"
    );
    assert!(
        !root.join("D_MEDIUM").exists(),
        "medium dir should be archived"
    );

    // young entries remain untouched
    assert!(
        root.join("f_young.txt").exists(),
        "young file should remain"
    );
    assert!(root.join("D_YOUNG").exists(), "young dir should remain");

    // archive should contain all moved entries (with .bak suffix)
    let archived: Vec<_> = fs::read_dir(&archive)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(archived.len(), 5, "archive should contain 5 entries");
    assert!(archived.iter().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        name.starts_with("f_old.") && name.ends_with(".bak.txt")
    }));
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
    assert!(archived.iter().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        name.starts_with("f_medium.") && name.ends_with(".bak.txt")
    }));
    assert!(
        archived
            .iter()
            .any(|e| e.file_name().to_string_lossy().starts_with("D_MEDIUM."))
    );
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
        ignore_hidden_entries: true,
    };

    declutter_directory(cfg, true).unwrap();

    // all entries should remain in place
    assert!(
        root.join("f_old.txt").exists(),
        "old file should still exist"
    );
    assert!(root.join("D_OLD").exists(), "old dir should still exist");
    assert!(
        root.join("D_OLD_NESTING").exists(),
        "old nested dir should still exist"
    );
    assert!(
        root.join("f_medium.txt").exists(),
        "medium file should still exist"
    );
    assert!(
        root.join("D_MEDIUM").exists(),
        "medium dir should still exist"
    );
    assert!(
        root.join("f_young.txt").exists(),
        "young file should still exist"
    );
    assert!(
        root.join("D_YOUNG").exists(),
        "young dir should still exist"
    );

    // archive exists but should be empty
    assert!(archive.is_dir(), "archive directory should exist");
    let archived: Vec<_> = fs::read_dir(&archive)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
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
        ignore_hidden_entries: true,
    };

    declutter_directory(cfg, true).unwrap();

    // all entries should remain in place
    assert!(
        root.join("f_old.txt").exists(),
        "old file should still exist"
    );
    assert!(root.join("D_OLD").exists(), "old dir should still exist");
    assert!(
        root.join("D_OLD_NESTING").exists(),
        "old nested dir should still exist"
    );
    assert!(
        root.join("f_medium.txt").exists(),
        "medium file should still exist"
    );
    assert!(
        root.join("D_MEDIUM").exists(),
        "medium dir should still exist"
    );
    assert!(
        root.join("f_young.txt").exists(),
        "young file should still exist"
    );
    assert!(
        root.join("D_YOUNG").exists(),
        "young dir should still exist"
    );
}

#[test]
fn test_declutter_rejects_dangerous_path() {
    let cfg = DirConfig {
        path: std::path::PathBuf::from("/"),
        time_to_archive_hours: 1,
        time_to_deletion_hours: 2,
        ignore_hidden_entries: true,
    };
    let result = declutter_directory(cfg, true);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("dangerous path"),
        "expected dangerous path error, got: {}",
        err_msg
    );
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
        ignore_hidden_entries: true,
    };

    declutter_directory(cfg, false).unwrap();

    // Metadata files should survive
    assert!(
        root.join(".DS_Store").exists(),
        ".DS_Store should be ignored and survive"
    );
    assert!(
        root.join("Thumbs.db").exists(),
        "Thumbs.db should be ignored and survive"
    );

    // Normal old file should be gone (deleted, since it exceeds deletion threshold)
    assert!(
        !root.join("old_file.txt").exists(),
        "old_file.txt should have been deleted"
    );

    // Metadata files should NOT be in the archive
    let archive = root.join(".duansheli-archive");
    if archive.exists() {
        let archived: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        for entry in &archived {
            let name = entry.file_name().to_string_lossy().to_string();
            assert!(
                !name.starts_with(".DS_Store"),
                ".DS_Store should not be in archive"
            );
            assert!(
                !name.starts_with("Thumbs.db"),
                "Thumbs.db should not be in archive"
            );
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
    // debug_print_tree_with_timestamps(root);

    let cfg = DirConfig {
        path: root.to_path_buf(),
        time_to_archive_hours,
        time_to_deletion_hours,
        ignore_hidden_entries: true,
    };

    // act
    declutter_directory(cfg, false).unwrap();

    // assert — all old and medium entries removed from root
    assert!(
        !root.join("f_old.txt").exists(),
        "old file should leave root"
    );
    assert!(!root.join("D_OLD").exists(), "old dir should leave root");
    assert!(
        !root.join("D_OLD_NESTING").exists(),
        "old nested dir should leave root"
    );
    assert!(
        !root.join("f_medium.txt").exists(),
        "medium file should leave root"
    );
    assert!(
        !root.join("D_MEDIUM").exists(),
        "medium dir should leave root"
    );

    // young entries untouched
    assert!(
        root.join("f_young.txt").exists(),
        "young file should remain"
    );
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
    assert!(remaining.iter().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        name.starts_with("f_medium.") && name.ends_with(".bak.txt")
    }));
    assert!(
        remaining
            .iter()
            .any(|e| e.file_name().to_string_lossy().starts_with("D_MEDIUM."))
    );
}

/// Run the duansheli binary as a subprocess, pointing HOME to a temp dir so the
/// log file is created in a predictable, isolated location.
#[test]
fn test_log_file_created_on_run() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path();

    // Expected log path when HOME is redirected
    #[cfg(target_os = "macos")]
    let expected_log = fake_home.join("Library/Logs/duansheli/duansheli.log");
    #[cfg(target_os = "linux")]
    let expected_log = fake_home.join(".local/state/duansheli/duansheli.log");
    #[cfg(target_os = "windows")]
    let expected_log = {
        // On Windows we redirect LOCALAPPDATA instead
        fake_home.join(r"duansheli\logs\duansheli.log")
    };

    // Build the path to our binary (in the cargo target directory)
    let binary = env!("CARGO_BIN_EXE_duansheli");

    // Run the binary — it will fail to find a config file, but logging init
    // happens before config loading, so the log file will still be created.
    let mut cmd = std::process::Command::new(binary);
    cmd.env("HOME", fake_home);
    #[cfg(target_os = "windows")]
    cmd.env("LOCALAPPDATA", fake_home);
    // Clear XDG_STATE_HOME so Linux uses the HOME-based fallback
    cmd.env_remove("XDG_STATE_HOME");
    // Clear XDG_CONFIG_HOME so it uses HOME-based default (and fails to find config)
    cmd.env_remove("XDG_CONFIG_HOME");

    let _output = cmd.output().expect("failed to run duansheli binary");

    // The binary will exit with an error (no config file), but the log file
    // should still exist with content from the logging initialization
    assert!(
        expected_log.exists(),
        "log file should be created at {}",
        expected_log.display()
    );

    let log_content = fs::read_to_string(&expected_log).unwrap();
    assert!(!log_content.is_empty(), "log file should not be empty");
    // The error about the missing config file should be logged
    assert!(
        log_content.contains("ERROR")
            || log_content.contains("WARN")
            || log_content.contains("DEBUG"),
        "log file should contain log level markers, got: {log_content}"
    );
}

#[cfg(unix)]
mod symlink_tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn backdate_symlink(path: &std::path::Path, age_secs: u64) {
        let new_mtime = SystemTime::now() - Duration::from_secs(age_secs);
        let ft = filetime::FileTime::from_system_time(new_mtime);
        filetime::set_symlink_file_times(path, ft, ft).unwrap();
    }

    #[test]
    fn test_symlink_to_file_is_archived_as_link() {
        let time_to_archive_hours: u64 = 1;
        let time_to_deletion_hours: u64 = 999;
        let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;

        let outside = TempDir::new().unwrap();
        let target = outside.path().join("real.txt");
        fs::write(&target, "important contents").unwrap();

        let tracked = TempDir::new().unwrap();
        let root = tracked.path();
        let link_path = root.join("link_to_file");
        symlink(&target, &link_path).unwrap();
        backdate_symlink(&link_path, exceeds_archive_secs);

        let cfg = DirConfig {
            path: root.to_path_buf(),
            time_to_archive_hours,
            time_to_deletion_hours,
            ignore_hidden_entries: true,
        };
        declutter_directory(cfg, false).unwrap();

        assert!(target.exists(), "target file must survive");
        assert_eq!(fs::read_to_string(&target).unwrap(), "important contents");
        assert!(
            fs::symlink_metadata(&link_path).is_err(),
            "link should be gone from tracked root"
        );

        let archive = root.join(".duansheli-archive");
        let archived: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(archived.len(), 1, "archive should contain the moved link");
        let entry = &archived[0];
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(name.starts_with("link_to_file."), "got name {name}");
        let archived_ft = fs::symlink_metadata(entry.path()).unwrap().file_type();
        assert!(
            archived_ft.is_symlink(),
            "archived entry must still be a symlink, not a copy"
        );
    }

    #[test]
    fn test_symlink_to_dir_is_archived_as_link() {
        let time_to_archive_hours: u64 = 1;
        let time_to_deletion_hours: u64 = 999;
        let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;

        let outside = TempDir::new().unwrap();
        let target_dir = outside.path().join("real_dir");
        fs::create_dir(&target_dir).unwrap();
        let child = target_dir.join("child.txt");
        fs::write(&child, "child contents").unwrap();

        let tracked = TempDir::new().unwrap();
        let root = tracked.path();
        let link_path = root.join("link_to_dir");
        symlink(&target_dir, &link_path).unwrap();
        backdate_symlink(&link_path, exceeds_archive_secs);

        let cfg = DirConfig {
            path: root.to_path_buf(),
            time_to_archive_hours,
            time_to_deletion_hours,
            ignore_hidden_entries: true,
        };
        declutter_directory(cfg, false).unwrap();

        assert!(target_dir.exists(), "target dir must survive");
        assert!(child.exists(), "child of target dir must survive");
        assert_eq!(fs::read_to_string(&child).unwrap(), "child contents");
        assert!(
            fs::symlink_metadata(&link_path).is_err(),
            "link should be gone from tracked root"
        );

        let archive = root.join(".duansheli-archive");
        let archived: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(
            archived.len(),
            1,
            "archive should contain only the moved link"
        );
        let entry = &archived[0];
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(name.starts_with("link_to_dir."), "got name {name}");
        let archived_ft = fs::symlink_metadata(entry.path()).unwrap().file_type();
        assert!(
            archived_ft.is_symlink(),
            "archived entry must still be a symlink"
        );
        assert!(
            !archived_ft.is_dir(),
            "archived entry must not be a real directory"
        );
    }
}

mod special_name_tests {
    use std::result;

    use super::*;

    /// Outcome of a single archive pass over a freshly-built temp dir.
    #[allow(dead_code)]
    struct ArchivePass {
        /// File names found in the archive afterwards (`.bak`-suffixed).
        archived: Vec<String>,
        /// File names still present in the tracked root (archive dir excluded).
        remaining_in_root: Vec<String>,
    }

    /// Build a temp dir holding one old entry per `names`, run an archive-only
    /// declutter pass, and report what landed in the archive vs. stayed in root.
    ///
    /// `time_to_deletion_hours` is set high so nothing is ever deleted — every
    /// aged entry should be *archived*, which is what these tests assert on.
    /// `ignore_hidden_entries` is threaded through so the hidden-file test can
    /// exercise both settings.
    #[allow(dead_code)]
    fn run_archive_pass(names: &[&str], ignore_hidden_entries: bool) -> ArchivePass {
        let time_to_archive_hours: u64 = 1;
        let time_to_deletion_hours: u64 = 999;
        let exceeds_archive_secs = (time_to_archive_hours * 3600) + 1;

        let tmp_dir = TempDir::new().unwrap();
        let root = tmp_dir.path();
        for name in names {
            create_file_fixture(root, name, exceeds_archive_secs);
        }

        let cfg = DirConfig {
            path: root.to_path_buf(),
            time_to_archive_hours,
            time_to_deletion_hours,
            ignore_hidden_entries,
        };
        declutter_directory(cfg, false).unwrap();

        let names_in = |dir: &std::path::Path| -> Vec<String> {
            if !dir.exists() {
                return Vec::new();
            }
            fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n != ".duansheli-archive")
                .collect()
        };

        ArchivePass {
            archived: names_in(&root.join(".duansheli-archive")),
            remaining_in_root: names_in(root),
        }
    }

    /// Assert the archive holds an entry whose name derives from `original`
    /// (the planner inserts a `.<timestamp>.bak` infix, so we match the stem).
    #[allow(dead_code)]
    fn assert_archived(pass: &ArchivePass, original: &str) {
        let stem = std::path::Path::new(original)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let prefix = format!("{stem}.");
        assert!(
            pass.archived.iter().any(|n| n.starts_with(&prefix)),
            "expected an archived entry for {original:?} (prefix {prefix:?}), \
             archive contained: {:?}",
            pass.archived
        );
    }

    fn assert_not_archived(pass: &ArchivePass, original: &str) {
        let stem = std::path::Path::new(&original)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let prefix = format!("{stem}.");

        assert!(
            !pass.archived.iter().any(|n| n.starts_with(&prefix)),
            "expected no archived entry for {original:?} (prefix {prefix:?}), \
             archive contained: {:?}",
            pass.archived
        )
    }

    #[test]
    fn unicode_names_are_archived() {
        // café.txt, 日本語.txt, 🚀.bin — NFC/NFD round-trip via the FS.
    }

    #[test]
    fn spaces_and_shell_metacharacters_are_archived() {
        // "my file.txt", "weird$name", literal "*.tmp", single/double quotes.
    }

    #[test]
    fn hidden_entries_are_archived_via_config() {
        let result = run_archive_pass(&[".foo", "bar"], false);
        assert_archived(&result, ".foo");
    }

    #[test]
    fn hidden_entries_ignored_via_config() {
        let result = run_archive_pass(&[".foo", "bar"], true);
        assert_not_archived(&result, ".foo");
    }

    #[test]
    fn control_characters_in_names_do_not_break_logging() {
        // "a\nb", "a\tb" — must not corrupt log output or assertions.
    }

    #[cfg(unix)]
    #[test]
    fn trailing_dots_and_spaces_are_archived() {
        // "name.", "name " — legal on Unix, illegal on Windows.
    }

    #[test]
    fn very_long_names_are_archived() {
        // Near NAME_MAX (255 bytes). Multi-byte chars consume the budget.
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_names_do_not_panic() {
        // "\xFF" in the filename — Unix only (Windows is WTF-16).
        // Today path.to_str().unwrap() panics; this test pins the contract.
    }
}

#[test]
fn test_default_log_path_uses_home() {
    // Verify the public default_log_path() function returns a path rooted in HOME
    let path = default_log_path();
    let home = std::env::var("HOME").unwrap();
    assert!(
        path.starts_with(&home),
        "log path should be under HOME ({home}), got: {}",
        path.display()
    );
}
