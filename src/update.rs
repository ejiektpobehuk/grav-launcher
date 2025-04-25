use color_eyre::{Result, eyre::eyre};
use eyre::WrapErr;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc;

use crate::REPOSITORY;
use crate::event::Event;

/// The GitHub API endpoint for retrieving the latest release
fn github_api_releases_url() -> String {
    // Extract the repository owner and name from the full repository URL
    // Expected format: "https://github.com/owner/repo"
    let path = REPOSITORY.trim_start_matches("https://github.com/");
    format!("https://api.github.com/repos/{path}/releases/latest")
}

/// Struct representing a GitHub release
#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

/// Struct representing a GitHub release asset
#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check if a newer version of the launcher is available
/// Returns Ok(Some(version)) if an update is available, Ok(None) if not
pub fn check_for_update(current_version: &str) -> Result<Option<String>> {
    // Remove 'v' prefix if present for comparison
    let current_version = current_version.trim_start_matches('v');

    // Fetch the latest release from GitHub
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(github_api_releases_url())
        .header("User-Agent", "grav-launcher")
        .send()
        .wrap_err("Failed to connect to GitHub API")?;

    if !response.status().is_success() {
        return Err(eyre!("GitHub API returned error: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .wrap_err("Failed to parse GitHub API response")?;

    // Extract the version number from the tag (remove 'v' prefix)
    let latest_version = release.tag_name.trim_start_matches('v');

    // Compare versions
    if is_newer_version(current_version, latest_version) {
        Ok(Some(release.tag_name))
    } else {
        Ok(None)
    }
}

/// Download and apply the update
pub fn update_launcher(version: &str, tx: &mpsc::Sender<Event>) -> Result<()> {
    // Find the correct asset to download
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(github_api_releases_url())
        .header("User-Agent", "grav-launcher")
        .send()
        .wrap_err("Failed to connect to GitHub API")?;

    let release: GitHubRelease = response
        .json()
        .wrap_err("Failed to parse GitHub API response")?;

    // Find the grav-launcher asset
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "grav-launcher")
        .ok_or_else(|| eyre!("Could not find launcher binary in release assets"))?;

    // Notify UI that download is starting
    if tx.send(Event::StartDownloadingLauncherUpdate).is_err() {
        return Err(eyre!(
            "Channel disconnected when starting launcher download"
        ));
    }

    // Download the new version
    let binary_response = reqwest::blocking::get(&asset.browser_download_url)
        .wrap_err("Failed to download launcher update")?;

    let total_size = binary_response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok()?.parse::<u64>().ok());

    // Get the current executable path
    let current_exe = env::current_exe().wrap_err("Failed to get current executable path")?;

    // Create a temporary file for the download
    let temp_path = current_exe.with_file_name(format!("grav-launcher.{version}.new"));
    let mut file = File::create(&temp_path)
        .wrap_err_with(|| format!("Failed to create temporary file at {temp_path:?}"))?;

    // Stream the download
    let mut downloaded: u64 = 0;
    let mut resp = binary_response;
    let mut buffer = [0u8; 8 * 1024];

    // Initial progress update with total size
    if tx
        .send(Event::LauncherDownloadProgress(0, total_size))
        .is_err()
    {
        return Err(eyre!("Channel disconnected during launcher download"));
    }

    loop {
        let bytes_read = resp
            .read(&mut buffer)
            .wrap_err("Failed to read from HTTP stream")?;

        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .wrap_err("Failed to write binary file to disk")?;

        downloaded += bytes_read as u64;

        // Update UI with progress
        if tx
            .send(Event::LauncherDownloadProgress(downloaded, total_size))
            .is_err()
        {
            return Err(eyre!("Channel disconnected during launcher download"));
        }
    }

    // Make the file executable
    let mut perms = fs::metadata(&temp_path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x permissions
    fs::set_permissions(&temp_path, perms)?;

    // Notify UI that download is complete
    if tx.send(Event::LauncherUpdateDownloaded).is_err() {
        return Err(eyre!(
            "Channel disconnected after launcher download completed"
        ));
    }

    // Notify UI that the update is ready to apply
    if tx.send(Event::LauncherUpdateReady).is_err() {
        return Err(eyre!("Channel disconnected after launcher update prepared"));
    }

    Ok(())
}

/// Apply the update by replacing the current executable
pub fn apply_update(tx: &mpsc::Sender<Event>) -> Result<()> {
    // Check if there's a pending update
    let current_exe = env::current_exe().wrap_err("Failed to get current executable path")?;

    // Look for temporary update files that match our pattern: grav-launcher.v*.new
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| eyre!("Couldn't get parent directory of executable"))?;
    let entries = fs::read_dir(exe_dir).wrap_err("Failed to read executable directory")?;

    // Find update files
    let mut update_file = None;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Check if this is an update file
        if file_name_str.starts_with("grav-launcher.") && file_name_str.ends_with(".new") {
            update_file = Some(entry.path());
            break;
        }
    }

    // If no update file is found, exit
    let Some(update_path) = update_file else {
        return Ok(());
    };

    // Notify UI that update is being applied
    if tx.send(Event::LauncherApplyingUpdate).is_err() {
        return Err(eyre!("Channel disconnected when applying launcher update"));
    }

    // Replace the executable - on Unix systems, we can do this while the program is running
    fs::rename(&update_path, &current_exe).wrap_err_with(|| {
        format!(
            "Failed to replace executable: {} -> {}",
            update_path.display(),
            current_exe.display()
        )
    })?;

    // Notify the user that they need to restart the application
    if tx.send(Event::LauncherUpdateApplied).is_err() {
        return Err(eyre!(
            "Channel disconnected when notifying about successful update"
        ));
    }

    Ok(())
}

/// Compare version strings to determine if the target version is newer
fn is_newer_version(current: &str, target: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    };

    let current_parts = parse_version(current);
    let target_parts = parse_version(target);

    for (i, target_part) in target_parts.iter().enumerate() {
        let current_part = current_parts.get(i).copied().unwrap_or(0);

        match target_part.cmp(&current_part) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }

    // If we've compared all parts and they're equal, check if target has more parts
    target_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_higher_version_returns_true() {
        assert!(is_newer_version("1.0.0", "1.0.1"), "Patch version bump");
        assert!(is_newer_version("1.0.0", "1.1.0"), "Minor version bump");
        assert!(is_newer_version("1.0.0", "2.0.0"), "Major version bump");
    }

    #[test]
    fn test_lower_version_returns_false() {
        assert!(!is_newer_version("1.0.1", "1.0.0"), "Lower patch version");
        assert!(!is_newer_version("1.1.0", "1.0.0"), "Lower minor version");
        assert!(!is_newer_version("2.0.0", "1.0.0"), "Lower major version");
    }

    #[test]
    fn test_equal_version_returns_false() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_different_length_versions() {
        assert!(
            is_newer_version("1.0", "1.0.1"),
            "Target has extra component"
        );
        assert!(
            !is_newer_version("1.0.1", "1.0"),
            "Current has extra component"
        );
        assert!(
            is_newer_version("1.0.0", "1.0.0.1"),
            "Target has additional component"
        );
    }

    #[test]
    fn test_version_with_large_numbers() {
        assert!(
            is_newer_version("1.9.0", "1.10.0"),
            "Properly compare 10 > 9"
        );
        assert!(
            is_newer_version("2.0.9", "2.0.10"),
            "Properly compare 10 > 9 in patch"
        );
    }
}
