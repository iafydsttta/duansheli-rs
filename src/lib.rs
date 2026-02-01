use std::path::Path;
use std::error::Error;
use std::time::SystemTime;
use std::fs::remove_file;


pub fn list_dir_with_meta(dir: &str) -> Result<(), Box<dyn Error>> {
    
    let dir = Path::new(dir);
    if !dir.is_dir() {
        // TODO: useing anyhow crate: return Err(anyhow!("Directory does not exist")); 
        let err = Err("Directory does not exist".into());
        return err;
    }
    for entry in dir.read_dir().unwrap() {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let time_since_modified_seconds = SystemTime::now().duration_since(metadata.modified()?).unwrap().as_secs();
        println!("File: {:?}, Metadata: {:?}, \n", entry.path(), metadata);
        println!("File: {:?}, Duration since modified: {:?}, \n", entry.path(), time_since_modified_seconds);
        
        if time_since_modified_seconds > 24*60*60 {
            remove_file(&entry.path())?;
        }
    }
    
    Ok(())
}

pub fn move_to_archive() {
    unimplemented!();
}
