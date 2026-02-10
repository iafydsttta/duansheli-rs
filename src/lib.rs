use std::path::Path;
use std::error::Error;
use std::time::SystemTime;
use std::fs::remove_file;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DirConfig {
    pub path: String,
    pub time_to_archive_hours: i32,
    pub time_to_delete_from_archive_hours: i32
}

pub fn declutter_directory(dir: &str, cfg: DirConfig) -> Result<(), Box<dyn Error>> {

    // TODO: move `dir` into DirConfig
    for entry in list_dir_with_meta(dir)? {
        println!("{}, {}", &entry.path, &entry.seconds_since_modification);
        if entry.seconds_since_modification > (cfg.time_to_archive_hours * 3600).try_into().unwrap() {
            println!("TTA: {}", cfg.time_to_archive_hours);
            println!("Old entry detected.");
        }
    }

   // iter dir
   // for item in dir:
   // archive?
   
   // iter archive
   // for item in archive:
   // delete?
   Ok(())
}

pub struct DirEntryWithAge {
    path: String,
    seconds_since_modification: u64
}

pub fn list_dir_with_meta(dir: &str) -> Result<Vec<DirEntryWithAge>, Box<dyn Error>> {
    
    let dir = Path::new(dir);

    if !dir.is_dir() {
        let err = Err("Directory does not exist".into());
        return err;
    }
    
    let entries: Vec<DirEntryWithAge> = dir.read_dir()?
        .filter_map(|entry_result| {

            let entry = entry_result
                .inspect_err(| e | eprintln!("Error reading entry: {}", e))
                .ok()?;
            
            let meta = entry.metadata()
                .inspect_err(| e | eprintln!("Error reading metadata: {}", e))
                .ok()?;
            
            let modified = meta.modified()
                .inspect_err(|e| eprintln!("Error getting modification date: {}", e))
                .ok()?;
            
            let seconds_since_modification = SystemTime::now().duration_since(modified)
                .inspect_err(|e| eprintln!("Error getting time since modification: {}", e))
                .ok()?.as_secs();

            Some(DirEntryWithAge{
                path: entry.path().to_string_lossy().into_owned(),
                seconds_since_modification
            })
    }).collect();

    Ok(entries)
}

pub fn move_to_archive() {
    unimplemented!();
}

pub fn delete_file() {
   unimplemented!(); 
}
