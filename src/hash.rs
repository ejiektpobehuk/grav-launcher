use color_eyre::Result;
use color_eyre::eyre::eyre;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

pub fn get_remote_hash(base_url: &str) -> Result<String> {
    let sha_url = format!("{base_url}.sha256");
    let current_version_hash_body = reqwest::blocking::get(sha_url)?.text()?;
    Ok(current_version_hash_body.trim().to_string())
}

pub fn get_local_hash() -> Result<Option<(String, PathBuf)>> {
    // Specify the file path
    let xdg_dirs = xdg::BaseDirectories::with_prefix("GRAV")
        .map_err(|e| eyre!("Failed to get xdg directories: {}", e))?;

    if let Some(game_binary_path) = xdg_dirs.find_data_file("GRAV.x86_64") {
        // Open the file in read-only mode
        let file = File::open(&game_binary_path).map_err(|e| {
            eyre!(
                "Failed to open game binary at {:?}: {}",
                game_binary_path,
                e
            )
        })?;
        let mut reader = BufReader::new(file);

        // Create a Sha256 object
        let mut hasher = Sha256::new();

        // Read the file in chunks
        let mut buffer = [0; 1024];
        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| eyre!("Failed to read from file: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            // Feed the contents of the buffer into the hasher
            hasher.update(&buffer[..bytes_read]);
        }

        // Retrieve the final hash
        let result = hasher.finalize();
        Ok(Some((format!("{result:x}"), game_binary_path)))
    } else {
        Ok(None)
    }
}
