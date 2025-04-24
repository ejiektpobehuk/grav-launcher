use color_eyre::{Result, eyre::eyre};
use eyre::WrapErr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
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
    if tx.send(Event::AccessingOnlineHash).is_err() {
        return Err(eyre!("Channel disconnected at start of launcher logic"));
    }

    let remote_version_hash = match hash::get_remote_hash(BASE_URL) {
        Ok(hash) => hash,
        Err(e) => {
            if tx.send(Event::OfflineError(format!("{e}"))).is_err() {
                return Err(eyre!("Channel disconnected when reporting offline error"));
            }

            let xdg_dirs = match xdg::BaseDirectories::with_prefix("GRAV") {
                Ok(d) => d,
                Err(e) => {
                    if tx
                        .send(Event::LauncherError(format!(
                            "Failed to find XDG directories: {e}"
                        )))
                        .is_err()
                    {
                        return Err(eyre!("Channel disconnected when reporting XDG error"));
                    }
                    return Ok(());
                }
            };

            if let Some(game_binary_path) = xdg_dirs.find_data_file("GRAV.x86_64") {
                if let Err(e) = run_the_game(game_binary_path, tx) {
                    if tx.send(Event::GameExecutionError(format!("{e}"))).is_err() {
                        return Err(eyre!(
                            "Channel disconnected when reporting game execution error"
                        ));
                    }
                }
            } else if tx.send(Event::NoLocalBinaryFound).is_err() {
                return Err(eyre!("Channel disconnected when reporting no local binary"));
            }
            return Ok(());
        }
    };

    if tx
        .send(Event::RemoteHash(remote_version_hash.clone()))
        .is_err()
    {
        return Err(eyre!("Channel disconnected when reporting remote hash"));
    }

    if tx.send(Event::ComputingLocalHash).is_err() {
        return Err(eyre!(
            "Channel disconnected when reporting computing local hash"
        ));
    }

    match hash::get_local_hash() {
        Ok(Some((local_version_hash, game_path))) => {
            if tx
                .send(Event::LocalHash(local_version_hash.clone()))
                .is_err()
            {
                return Err(eyre!("Channel disconnected when reporting local hash"));
            }

            if local_version_hash == remote_version_hash {
                if tx.send(Event::HashAreEqual(true)).is_err() {
                    return Err(eyre!("Channel disconnected when reporting hash equality"));
                }

                if let Err(e) = check_exec_permissions(&game_path) {
                    if tx
                        .send(Event::LauncherError(format!(
                            "Failed to set exec permissions: {e}"
                        )))
                        .is_err()
                    {
                        return Err(eyre!(
                            "Channel disconnected when reporting permission error"
                        ));
                    }
                    // Optionally: still attempt to run anyway.
                }

                if let Err(e) = run_the_game(game_path, tx) {
                    if tx.send(Event::GameExecutionError(format!("{e}"))).is_err() {
                        return Err(eyre!(
                            "Channel disconnected when reporting game execution error"
                        ));
                    }
                }
            } else {
                if tx.send(Event::HashAreEqual(false)).is_err() {
                    return Err(eyre!("Channel disconnected when reporting hash inequality"));
                }

                match download_game_binary(remote_version_hash, tx) {
                    Ok(game_path) => {
                        if tx.send(Event::RemoteBinaryDownloaded).is_err() {
                            return Err(eyre!("Channel disconnected after binary download"));
                        }

                        if let Err(e) = run_the_game(game_path, tx) {
                            if tx.send(Event::GameExecutionError(format!("{e}"))).is_err() {
                                return Err(eyre!(
                                    "Channel disconnected when reporting game execution error"
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        if tx.send(Event::BinaryDownloadError(format!("{e}"))).is_err() {
                            return Err(eyre!(
                                "Channel disconnected when reporting binary download error"
                            ));
                        }
                    }
                }
            }
        }
        Ok(None) => match download_game_binary(remote_version_hash, tx) {
            Ok(game_path) => {
                if let Err(e) = run_the_game(game_path, tx) {
                    if tx.send(Event::GameExecutionError(format!("{e}"))).is_err() {
                        return Err(eyre!(
                            "Channel disconnected when reporting game execution error"
                        ));
                    }
                }
            }
            Err(e) => {
                if tx.send(Event::BinaryDownloadError(format!("{e}"))).is_err() {
                    return Err(eyre!(
                        "Channel disconnected when reporting binary download error"
                    ));
                }
            }
        },
        Err(e) => {
            if tx
                .send(Event::LauncherError(format!(
                    "Failed to compute local hash: {e}"
                )))
                .is_err()
            {
                return Err(eyre!(
                    "Channel disconnected when reporting hash computation error"
                ));
            }
        }
    }
    Ok(())
}

fn download_game_binary(current_hash: String, tx: &mpsc::Sender<Event>) -> Result<PathBuf> {
    let response = reqwest::blocking::get(BASE_URL)
        .wrap_err("Failed to download game binary (network/HTTP error)")?;
    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok()?.parse::<u64>().ok());

    let xdg_dirs =
        xdg::BaseDirectories::with_prefix("GRAV").wrap_err("Failed to get XDG data dir")?;
    let tmp_path = xdg_dirs
        .place_data_file(current_hash)
        .wrap_err("Can't create temporary file path")?;
    let mut file =
        File::create(&tmp_path).wrap_err_with(|| format!("Failed to create file {tmp_path:?}"))?;

    if tx.send(Event::StartDownloadingBinary(total_size)).is_err() {
        return Err(eyre!(
            "Launcher channel disconnected during download initialization"
        ));
    }

    let mut downloaded: u64 = 0;
    let mut resp = response;
    let mut buffer = [0u8; 8 * 1024];

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

        if tx.send(Event::DownloadProgress(downloaded)).is_err() {
            return Err(eyre!("Launcher channel disconnected during download"));
        }
    }

    if tx.send(Event::RemoteBinaryDownloaded).is_err() {
        return Err(eyre!(
            "Launcher channel disconnected after download completed"
        ));
    }

    check_exec_permissions(&tmp_path)?;
    let destination_path = xdg_dirs
        .place_data_file("GRAV.x86_64")
        .wrap_err("Can't create data file path")?;
    fs::copy(&tmp_path, &destination_path)?;

    if tx.send(Event::GameBinaryUpdated).is_err() {
        return Err(eyre!("Launcher channel disconnected after binary update"));
    }

    Ok(tmp_path)
}

fn run_the_game(game_path: PathBuf, tx: &mpsc::Sender<Event>) -> Result<()> {
    if tx.send(Event::Launching).is_err() {
        return Err(eyre!("Launcher channel disconnected"));
    }

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
                    if tx_stdout.send(Event::GameOutput(l)).is_err() {
                        eprintln!("Game output channel disconnected, shutting down stdout thread");
                        return;
                    }
                }
                Err(e) => {
                    if tx_stdout
                        .send(Event::GameExecutionError(format!("stdout read: {e}")))
                        .is_err()
                    {
                        eprintln!("Game output channel disconnected, shutting down stdout thread");
                        return;
                    }
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
                    if tx_stderr.send(Event::GameErrorOutput(l)).is_err() {
                        eprintln!(
                            "Game error output channel disconnected, shutting down stderr thread"
                        );
                        return;
                    }
                }
                Err(e) => {
                    if tx_stderr
                        .send(Event::GameExecutionError(format!("stderr read: {e}")))
                        .is_err()
                    {
                        eprintln!(
                            "Game error output channel disconnected, shutting down stderr thread"
                        );
                        return;
                    }
                }
            }
        }
    });

    Ok(())
}

fn check_exec_permissions(binary_path: &PathBuf) -> Result<()> {
    let permissions = fs::Permissions::from_mode(0o744);
    fs::set_permissions(binary_path, permissions)
        .wrap_err_with(|| format!("Failed to set execute permissions for {binary_path:?}"))?;
    Ok(())
}
