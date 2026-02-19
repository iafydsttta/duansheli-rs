use chrono::Utc;
use serde::Deserialize;
use std::error::Error;
use std::fs::{create_dir_all, remove_file, rename};
use std::path::Path;
use std::time::SystemTime;

#[derive(Deserialize, Debug)]
pub struct DirConfig {
    pub path: String,
    pub time_to_archive_hours: i32,
    pub time_to_delete_from_archive_hours: i32,
}

pub fn declutter_directory(cfg: DirConfig) -> Result<(), Box<dyn Error>> {
    // ensure archive exists
    let archive_name = ".duansheli-archive";
    create_dir_all(Path::new(&cfg.path).join(archive_name))?;

    // clean up dir
    for entry in list_dir_with_meta(&cfg.path, archive_name)? {
        println!("{}, {}", &entry.path, &entry.seconds_since_modification);

        if entry.seconds_since_modification
            >= (cfg.time_to_archive_hours * 3600).try_into().unwrap()
        {
            println!(
                "Old entry detected: {}, {}",
                &entry.path, entry.seconds_since_modification
            );

            // move to archive
            let source = Path::new(&entry.path);
            let filename = source
                .file_name()
                .ok_or("Invalid filename")?
                .to_string_lossy();
            let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
            let new_name = format!("{}.{}.bak", filename, timestamp);

            let target_path = Path::new(&cfg.path).join(archive_name).join(&new_name);

            rename(source, &target_path)?;
        }
    }

    // clean up corresponding archive
    Ok(())
}

pub struct DirEntryWithAge {
    pub path: String,
    pub seconds_since_modification: u64,
}

pub fn list_dir_with_meta(
    dir: &str,
    exclude_recursive: &str,
) -> Result<Vec<DirEntryWithAge>, Box<dyn Error>> {
    let dir = Path::new(dir);

    if !dir.is_dir() {
        let err = Err("Directory does not exist".into());
        return err;
    }

    // let raw_out: Vec<Result<std::fs::DirEntry, std::io::Error>> = dir.read_dir()?.collect();

    let entries: Vec<DirEntryWithAge> = dir
        .read_dir()?
        .filter_map(|entry_result| {
            let entry = entry_result
                .inspect_err(|e| eprintln!("Error reading entry: {}", e))
                .ok()?;

            if entry.file_name() == exclude_recursive {
                println!("Excluding: {}", exclude_recursive);
                return None;
            }

            let meta = entry
                .metadata()
                .inspect_err(|e| eprintln!("Error reading metadata: {}", e))
                .ok()?;

            let modified = meta
                .modified()
                .inspect_err(|e| eprintln!("Error getting modification date: {}", e))
                .ok()?;

            let seconds_since_modification = SystemTime::now()
                .duration_since(modified)
                .inspect_err(|e| eprintln!("Error getting time since modification: {}", e))
                .ok()?
                .as_secs();

            Some(DirEntryWithAge {
                path: entry.path().to_string_lossy().into_owned(),
                seconds_since_modification,
            })
        })
        .collect();

    Ok(entries)
}

pub fn move_to_archive() {
    unimplemented!();
}

pub fn delete_file() {
    unimplemented!();
}
