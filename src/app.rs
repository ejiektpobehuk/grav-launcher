use crate::event::Event;
use crate::ui::AppState;
use crate::ui::draw;
use color_eyre::Result;
use crossterm::event as terminal_event;
use crossterm::event::KeyCode;
use gilrs::Button;
use ratatui::prelude::*;
use std::sync::mpsc;

pub fn run(terminal: &mut Terminal<impl Backend>, rx: &mpsc::Receiver<Event>) -> Result<()> {
    let mut app_state = AppState::init();
    loop {
        terminal.draw(|frame| draw(frame, &mut app_state))?;
        match rx.recv()? {
            Event::Input(event) => {
                if app_state.show_exit_popup {
                    // Handle input while exit popup is active
                    match event.code {
                        // Confirm exit
                        KeyCode::Enter | KeyCode::Char('y') => {
                            break;
                        }
                        // Cancel exit
                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                            app_state.hide_exit_popup();
                        }
                        _ => {}
                    }
                } else if app_state.fullscreen_mode {
                    // In fullscreen mode, Escape/h/q return to normal view
                    match event.code {
                        KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('q') => {
                            app_state.exit_fullscreen();
                        }
                        _ => {}
                    }
                } else {
                    // In normal mode
                    match event.code {
                        // Show exit confirmation popup
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app_state.show_exit_popup();
                        }
                        // Enter fullscreen with Enter/l
                        KeyCode::Enter | KeyCode::Char('l') => {
                            app_state.enter_fullscreen();
                        }
                        // Navigation with arrow keys and j/k
                        KeyCode::Right | KeyCode::Down | KeyCode::Char('j') => {
                            app_state.next_log();
                        }
                        KeyCode::Left | KeyCode::Up | KeyCode::Char('k') => {
                            app_state.prev_log();
                        }
                        _ => {}
                    }
                }
            }
            Event::ControllerInput(button) => {
                if app_state.show_exit_popup {
                    // Handle controller input while exit popup is active
                    match button {
                        // Confirm exit with A button
                        Button::South => {
                            break;
                        }
                        // Cancel exit with B button
                        Button::East => {
                            app_state.hide_exit_popup();
                        }
                        _ => {}
                    }
                } else if app_state.fullscreen_mode {
                    // In fullscreen mode, East (B) returns to normal view
                    if button == Button::East {
                        app_state.exit_fullscreen();
                    }
                } else {
                    // In normal mode
                    match button {
                        // Show exit confirmation with East (B) button
                        Button::East => {
                            app_state.show_exit_popup();
                        }
                        // Enter fullscreen with South (A) button
                        Button::South => {
                            app_state.enter_fullscreen();
                        }
                        // D-pad navigation
                        _ if button == Button::DPadRight || button == Button::DPadDown => {
                            app_state.next_log();
                        }
                        _ if button == Button::DPadLeft || button == Button::DPadUp => {
                            app_state.prev_log();
                        }
                        _ => {}
                    }
                }
            }
            Event::NextLog => {
                app_state.next_log();
            }
            Event::PrevLog => {
                app_state.prev_log();
            }
            Event::EnterFullscreen => {
                app_state.enter_fullscreen();
            }
            Event::ExitFullscreen => {
                app_state.exit_fullscreen();
            }
            Event::Resize => {
                terminal.autoresize()?;
            }
            Event::Tick => {}
            Event::AccessingOnlineHash => {
                app_state.log.remote_hash_msg = Some("accessing".into());
            }
            Event::OfflineError(err) => {
                app_state.log.remote_hash_msg =
                    Some(format!("unavailable. No internet connection: {err}"));
            }
            Event::RemoteHash(hash_value) => {
                app_state.log.remote_hash_msg = Some(hash_value);
            }
            Event::ComputingLocalHash => {
                app_state.log.local_hash_msg = Some("Computing".into());
            }
            Event::LocalHash(hash_value) => {
                app_state.log.local_hash_msg = Some(hash_value);
            }
            Event::HashAreEqual(eq) => {
                if eq {
                    app_state.log.push(
                        "Hashes are the same: You have the latest verstion of the game. ".into(),
                    );
                } else {
                    app_state
                        .log
                        .push("Hashes are different: There is a newer version.".into());
                }
            }
            Event::StartDownloadingBinary(total_download_size) => {
                app_state.log.start_download(total_download_size);
            }
            Event::DownloadProgress(downloaded) => {
                app_state.log.set_download_progress(downloaded);
            }
            Event::RemoteBinaryDownloaded => {
                app_state.log.mark_download_complete();
            }
            Event::BinaryDownloadError(err) => {
                app_state.log.set_download_error(err);
            }
            Event::NoLocalBinaryFound => {
                app_state
                    .log
                    .push("Local game binary not found".to_string());
            }
            Event::GameBinaryUpdated => {}
            Event::Launching => {
                app_state.log.push("Launcning the game. . .".to_string());
            }
            Event::GameExecutionError(err) => {
                app_state.log.push(format!("Game execution error: {err}"));
            }
            Event::GameOutput(stdout) => {
                app_state.game_stdout.push(stdout);
            }
            Event::GameErrorOutput(stderr) => {
                app_state.game_stderr.push(stderr);
            }
            Event::LauncherError(err) => {
                app_state.log.push(format!("Error: {err}"));
            }
        }
    }
    Ok(())
}
