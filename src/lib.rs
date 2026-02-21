use chrono::Utc;
use serde::Deserialize;
use std::error::Error;
use std::fs::{create_dir_all, remove_dir, remove_file, rename};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Deserialize, Debug)]
pub struct DirConfig {
    pub path: PathBuf,
    pub time_to_archive_hours: u64,
    pub time_to_delete_from_archive_hours: u64,
}

pub fn declutter_directory(cfg: DirConfig) -> Result<(), Box<dyn Error>> {
    // ensure archive exists
    let archive_name = ".duansheli-archive";
    let archive_path = Path::new(&cfg.path).join(archive_name);
    create_dir_all(&archive_path)?;

    // clean up dir
    for entry in list_dir_with_meta(&cfg.path, Some(archive_name))? {
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

    // clean up archive
    
    // TODO: This will not work yet as moving to archive does not affect mtime.
    for entry in list_dir_with_meta(&archive_path, None)? {
        println!("{}, {}", &entry.path, &entry.seconds_since_modification);

        if entry.seconds_since_modification
            >= (cfg.time_to_delete_from_archive_hours * 3600).try_into().unwrap()
        {
            if entry.is_dir {
                remove_dir(entry.path)?;
                
            } else {
                remove_file(entry.path)?;
            }
        }
    }

    Ok(())
}

pub struct DirEntryWithAge {
    pub path: String,
    pub seconds_since_modification: u64,
    pub is_dir: bool,
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
                .inspect_err(|e| eprintln!("Error reading entry: {}", e))
                .ok()?;

            if exclude_recursive.is_some_and(|x| x == entry.file_name()){
                println!("Excluding: {:?}", entry.file_name());
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
                is_dir: meta.is_dir()
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
