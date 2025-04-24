use crate::event::Event;
use crate::ui::AppState;
use crate::ui::draw;
use color_eyre::Result;
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
                app_state.keyboard_input_used();
                if handle_keyboard_input(&mut app_state, event.code) {
                    break;
                }
            }
            Event::ControllerInput(button) => {
                app_state.controller_input_used();
                if app_state.terminal_focused && handle_controller_input(&mut app_state, button) {
                    break;
                }
            }
            Event::TerminalFocusChanged(focused) => {
                app_state.set_terminal_focus(focused);
            }
            Event::Resize => {
                terminal.autoresize()?;
            }
            Event::Tick => {}
            event => handle_system_event(&mut app_state, event),
        }
    }
    Ok(())
}

/// Handle keyboard input based on current app state
/// Returns true if the application should exit
const fn handle_keyboard_input(app_state: &mut AppState, key: KeyCode) -> bool {
    if app_state.show_exit_popup {
        match key {
            // Confirm exit
            KeyCode::Enter | KeyCode::Char('y') => {
                return true;
            }
            // Cancel exit
            KeyCode::Esc | KeyCode::Char('n' | 'q') => {
                app_state.hide_exit_popup();
            }
            _ => {}
        }
    } else if app_state.fullscreen_mode {
        // In fullscreen mode, Escape/h/q return to normal view
        match key {
            KeyCode::Esc | KeyCode::Char('h' | 'q') => {
                app_state.exit_fullscreen();
            }
            _ => {}
        }
    } else {
        // In normal mode
        match key {
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
    false
}

/// Handle controller input based on current app state
/// Returns true if the application should exit
fn handle_controller_input(app_state: &mut AppState, button: Button) -> bool {
    if app_state.show_exit_popup {
        // Handle controller input while exit popup is active
        match button {
            // Confirm exit with A button
            Button::South => {
                return true;
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
    false
}

/// Handle system events like hashing, downloads, and game execution
fn handle_system_event(app_state: &mut AppState, event: Event) {
    match event {
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
                app_state
                    .log
                    .push("Hashes are the same: You have the latest verstion of the game. ".into());
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
        _ => {}
    }
}
