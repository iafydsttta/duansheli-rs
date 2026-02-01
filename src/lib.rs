use std::path::Path;
use std::error::Error;
use std::time::SystemTime;

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
        // println!("File: {:?}, Metadata: {:?}, \n", entry.path(), metadata);
        println!("File: {:?}, Duration since modified: {:?}, \n", entry.path(), SystemTime::now().duration_since(metadata.modified()?).unwrap());
    }
    
    Ok(())
}

pub fn move_to_archive() {
    unimplemented!();
}
