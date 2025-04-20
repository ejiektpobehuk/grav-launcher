use color_eyre::Result;
use eyre::WrapErr;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::{BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crate::BASE_URL;
use crate::event::Event;
use crate::hash;

pub fn launcher_logic(tx: mpsc::Sender<Event>) {
    tx.send(Event::AccessingOnlineHash).unwrap();
    match hash::get_remote_hash(&BASE_URL) {
        Ok(remote_version_hash) => {
            tx.send(Event::RemoteHash(remote_version_hash.clone()))
                .unwrap();
            tx.send(Event::ComputingLocalHash).unwrap();
            match hash::get_local_hash() {
                Ok(Some((local_version_hash, game_path))) => {
                    tx.send(Event::LocalHash(local_version_hash.clone()))
                        .unwrap();
                    if local_version_hash.eq(&remote_version_hash) {
                        tx.send(Event::HashAreEqual(true)).unwrap();
                        let _ = check_exec_permissions(&game_path);
                        run_the_game(game_path, tx.clone());
                    } else {
                        tx.send(Event::HashAreEqual(false)).unwrap();
                        match download_game_binary() {
                            Ok(game_path) => {
                                tx.send(Event::RemoteBinaryDownloaded).unwrap();
                                run_the_game(game_path, tx.clone());
                            }
                            Err(e) => {
                                tx.send(Event::BinaryDownloadError(format!("{e}"))).unwrap();
                                run_the_game(game_path, tx.clone());
                            }
                        };
                    };
                }
                Ok(None) => {
                    match download_game_binary() {
                        Ok(game_path) => {
                            tx.send(Event::RemoteBinaryDownloaded).unwrap();
                            run_the_game(game_path, tx.clone());
                        }
                        Err(e) => {
                            tx.send(Event::BinaryDownloadError(format!("{e}"))).unwrap();
                        }
                    };
                }
                Err(e) => {
                    tx.send(Event::BinaryDownloadError(format!(
                        "Failed to calculate local hash: {e}"
                    )))
                    .ok();
                }
            };
        }
        Err(e) => {
            tx.send(Event::OfflineError(format!("{e}"))).unwrap();
            let xdg_dirs = xdg::BaseDirectories::with_prefix("GRAV").unwrap();
            if let Some(game_binary_path) = xdg_dirs.find_data_file("GRAV.x86_64") {
                run_the_game(game_binary_path, tx.clone());
            } else {
                tx.send(Event::NoLocalBinaryFound).unwrap();
                println!("Game binary not found");
            }
        }
    }
}

fn download_game_binary() -> Result<PathBuf> {
    match reqwest::blocking::get(BASE_URL) {
        Ok(response) => {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("GRAV").unwrap();
            let file_path = xdg_dirs.place_data_file("GRAV.x86_64").unwrap();
            let mut file = File::create(&file_path).unwrap();
            let response_bytes = response.bytes().unwrap();
            let _ = file.write_all(&response_bytes);
            // let mut permissions = file.metadata().unwrap().permissions();
            // permissions.set_mode(0o744);
            // let _ = fs::set_permissions(&file_path, permissions);
            Ok(file_path)
        }
        Err(e) => Err(e).wrap_err("Download Error"),
    }
}

fn run_the_game(game_path: PathBuf, tx: mpsc::Sender<Event>) {
    tx.send(Event::Launching).unwrap();
    let child = Command::new(game_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(e) => {
            tx.send(Event::GameExecutionError(format!("{e}"))).unwrap();
            return;
        }
    };

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let tx_clone = tx.clone();
    // Create threads for reading stdout and stderr
    let stdout_thread = thread::spawn(move || {
        let stdout_reader = BufReader::new(stdout);
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                tx_clone.send(Event::GameOutput(line.to_string())).unwrap();
            }
        }
    });

    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let tx_clone = tx.clone();
    let stderr_thread = thread::spawn(move || {
        let stderr_reader = BufReader::new(stderr);
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                tx.send(Event::GameErrorOutput(line.to_string())).unwrap();
            }
        }
    });
}

fn check_exec_permissions(binary_path: &PathBuf) -> Result<()> {
    let permissions = fs::Permissions::from_mode(0o744);
    let _ = fs::set_permissions(binary_path, permissions);
    Ok(())
}
