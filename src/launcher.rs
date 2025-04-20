use color_eyre::{Result, eyre::eyre};
use eyre::WrapErr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::BASE_URL;
use crate::event::Event;
use crate::hash;

pub fn launcher_logic(tx: mpsc::Sender<Event>) {
    if let Err(e) = launcher_logic_impl(&tx) {
        let _ = tx.send(Event::LauncherError(format!("Launcher error: {e}")));
    }
}

fn launcher_logic_impl(tx: &mpsc::Sender<Event>) -> Result<()> {
    tx.send(Event::AccessingOnlineHash).ok();

    let remote_version_hash = match hash::get_remote_hash(BASE_URL) {
        Ok(hash) => hash,
        Err(e) => {
            tx.send(Event::OfflineError(format!("{e}"))).ok();

            let xdg_dirs = match xdg::BaseDirectories::with_prefix("GRAV") {
                Ok(d) => d,
                Err(e) => {
                    tx.send(Event::LauncherError(format!(
                        "Failed to find XDG directories: {e}"
                    )))
                    .ok();
                    return Ok(());
                }
            };

            if let Some(game_binary_path) = xdg_dirs.find_data_file("GRAV.x86_64") {
                if let Err(e) = run_the_game(game_binary_path, tx) {
                    tx.send(Event::GameExecutionError(format!("{e}"))).ok();
                }
            } else {
                tx.send(Event::NoLocalBinaryFound).ok();
            }
            return Ok(());
        }
    };

    tx.send(Event::RemoteHash(remote_version_hash.clone())).ok();
    tx.send(Event::ComputingLocalHash).ok();

    match hash::get_local_hash() {
        Ok(Some((local_version_hash, game_path))) => {
            tx.send(Event::LocalHash(local_version_hash.clone())).ok();
            if local_version_hash == remote_version_hash {
                tx.send(Event::HashAreEqual(true)).ok();
                if let Err(e) = check_exec_permissions(&game_path) {
                    tx.send(Event::LauncherError(format!(
                        "Failed to set exec permissions: {e}"
                    )))
                    .ok();
                    // Optionally: still attempt to run anyway.
                }
                if let Err(e) = run_the_game(game_path, tx) {
                    tx.send(Event::GameExecutionError(format!("{e}"))).ok();
                }
            } else {
                tx.send(Event::HashAreEqual(false)).ok();
                match download_game_binary() {
                    Ok(game_path) => {
                        tx.send(Event::RemoteBinaryDownloaded).ok();
                        if let Err(e) = run_the_game(game_path, tx) {
                            tx.send(Event::GameExecutionError(format!("{e}"))).ok();
                        }
                    }
                    Err(e) => {
                        tx.send(Event::BinaryDownloadError(format!("{e}"))).ok();
                    }
                }
            }
        }
        Ok(None) => match download_game_binary() {
            Ok(game_path) => {
                tx.send(Event::RemoteBinaryDownloaded).ok();
                if let Err(e) = run_the_game(game_path, tx) {
                    tx.send(Event::GameExecutionError(format!("{e}"))).ok();
                }
            }
            Err(e) => {
                tx.send(Event::BinaryDownloadError(format!("{e}"))).ok();
            }
        },
        Err(e) => {
            tx.send(Event::LauncherError(format!(
                "Failed to compute local hash: {e}"
            )))
            .ok();
        }
    };
    Ok(())
}

fn download_game_binary() -> Result<PathBuf> {
    let response = reqwest::blocking::get(BASE_URL)
        .wrap_err("Failed to download game binary (network/HTTP error)")?;

    let xdg_dirs =
        xdg::BaseDirectories::with_prefix("GRAV").wrap_err("Failed to get XDG data dir")?;
    let file_path = xdg_dirs
        .place_data_file("GRAV.x86_64")
        .wrap_err("Can't create data file path")?;
    let mut file = File::create(&file_path)
        .wrap_err_with(|| format!("Failed to create file {:?}", file_path))?;
    let response_bytes = response
        .bytes()
        .wrap_err("Failed to read HTTP response body as bytes")?;
    file.write_all(&response_bytes)
        .wrap_err("Failed to write binary file to disk")?;
    Ok(file_path)
}

fn run_the_game(game_path: PathBuf, tx: &mpsc::Sender<Event>) -> Result<()> {
    tx.send(Event::Launching).ok();
    let mut child = Command::new(game_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("Failed to launch game binary")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre!("Failed to capture stdout"))?;
    let tx_stdout = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    tx_stdout.send(Event::GameOutput(l)).ok();
                }
                Err(e) => {
                    tx_stdout
                        .send(Event::GameExecutionError(format!("stdout read: {e}")))
                        .ok();
                }
            }
        }
    });

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre!("Failed to capture stderr"))?;
    let tx_stderr = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    tx_stderr.send(Event::GameErrorOutput(l)).ok();
                }
                Err(e) => {
                    tx_stderr
                        .send(Event::GameExecutionError(format!("stderr read: {e}")))
                        .ok();
                }
            }
        }
    });

    Ok(())
}

fn check_exec_permissions(binary_path: &PathBuf) -> Result<()> {
    let permissions = fs::Permissions::from_mode(0o744);
    fs::set_permissions(binary_path, permissions)
        .wrap_err_with(|| format!("Failed to set execute permissions for {:?}", binary_path))?;
    Ok(())
}
