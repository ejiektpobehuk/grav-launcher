use crate::event::Event;
use crate::ui::draw;
use crate::ui::{AppState, DisplayMode, ExitPopupState, TerminalFocus, UpdateStatus};
use color_eyre::Result;
use crossterm::event::KeyCode;
use gilrs::{Axis, Button};
use ratatui::prelude::*;
use std::sync::mpsc;
use std::thread;

pub fn run(
    terminal: &mut Terminal<impl Backend>,
    rx: &mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
) -> Result<()> {
    let mut app_state = AppState::init();

    loop {
        terminal.draw(|frame| draw(frame, &mut app_state))?;
        match rx.recv()? {
            Event::Input(event) => {
                app_state.keyboard_input_used();
                if handle_keyboard_input(&mut app_state, &tx, event.code) {
                    break;
                }
            }
            Event::ControllerInput(button) => {
                app_state.controller_input_used();
                if app_state.terminal_focus == TerminalFocus::Focused
                    && handle_controller_input(&mut app_state, &tx, button)
                {
                    break;
                }
            }
            Event::ControllerAxisMoved(axis, value) => {
                app_state.controller_input_used();
                if app_state.terminal_focus == TerminalFocus::Focused {
                    handle_controller_axis(&mut app_state, axis, value);
                }
            }
            Event::TerminalFocusChanged(focused) => {
                app_state.set_terminal_focus(focused);
            }
            Event::Resize => {
                terminal.autoresize()?;
            }
            Event::Tick => {}
            event => handle_system_event(&mut app_state, &tx, event),
        }
    }
    Ok(())
}

/// Handle keyboard input based on current app state
/// Returns true if the application should exit
fn handle_keyboard_input(app_state: &mut AppState, tx: &mpsc::Sender<Event>, key: KeyCode) -> bool {
    if app_state.exit_popup == ExitPopupState::Visible {
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
    } else if let DisplayMode::Fullscreen(_) = app_state.display_mode {
        // In fullscreen mode, Escape/h/q return to normal view
        match key {
            KeyCode::Esc | KeyCode::Char('h' | 'q') => {
                app_state.exit_fullscreen();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app_state.scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app_state.scroll_down();
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
                app_state.enter_fullscreen(20); // Default visible height, will be updated in draw
            }
            // Navigation with arrow keys and j/k
            KeyCode::Right | KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                app_state.next_log();
            }
            KeyCode::Left | KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                app_state.prev_log();
            }
            // Request launcher update
            KeyCode::Char('u') => {
                // Only send the event if an update is available and not already in progress
                if app_state.launcher_update_available.is_some()
                    && app_state.update_status == UpdateStatus::NotRequested
                {
                    let _ = tx.send(Event::RequestLauncherUpdate);
                }
            }
            _ => {}
        }
    }
    false
}

/// Handle controller input based on current app state
/// Returns true if the application should exit
fn handle_controller_input(
    app_state: &mut AppState,
    tx: &mpsc::Sender<Event>,
    button: Button,
) -> bool {
    if app_state.exit_popup == ExitPopupState::Visible {
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
    } else if let DisplayMode::Fullscreen(_) = app_state.display_mode {
        // In fullscreen mode, East (B) returns to normal view
        match button {
            Button::East => {
                app_state.exit_fullscreen();
            }
            // Scrolling only in fullscreen mode
            Button::DPadUp => {
                app_state.scroll_up();
            }
            Button::DPadDown => {
                app_state.scroll_down();
            }
            // Shoulder buttons for log navigation in fullscreen mode
            Button::LeftTrigger => {
                app_state.prev_log();
            }
            Button::RightTrigger => {
                app_state.next_log();
            }
            // Triggers for scrolling to the max
            Button::LeftTrigger2 => {
                app_state.scroll_to_top();
            }
            Button::RightTrigger2 => {
                app_state.scroll_to_bottom();
            }
            _ => {}
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
                app_state.enter_fullscreen(20); // Default visible height, will be updated in draw
            }
            // Request launcher update with North (Y) button
            Button::North => {
                // Only send the event if an update is available and not already in progress
                if app_state.launcher_update_available.is_some()
                    && app_state.update_status == UpdateStatus::NotRequested
                {
                    let _ = tx.send(Event::RequestLauncherUpdate);
                }
            }
            // D-pad navigation
            Button::DPadRight | Button::DPadDown | Button::RightTrigger => {
                app_state.next_log();
            }
            Button::DPadLeft | Button::DPadUp | Button::LeftTrigger => {
                app_state.prev_log();
            }
            _ => {}
        }
    }
    false
}

/// Handle controller analog stick movement
fn handle_controller_axis(app_state: &mut AppState, axis: gilrs::Axis, value: f32) {
    // Only handle axis events when not in exit popup and based on app display mode
    if app_state.exit_popup == ExitPopupState::Visible {
        return;
    }

    match axis {
        Axis::LeftStickX => {
            if app_state.display_mode == DisplayMode::Normal {
                if value > 0.0 {
                    // Right movement
                    app_state.next_log();
                } else {
                    // Left movement
                    app_state.prev_log();
                }
            }
        }
        Axis::LeftStickY => {
            // Only handle vertical scrolling in fullscreen mode
            if let DisplayMode::Fullscreen(_) = app_state.display_mode {
                if value > 0.0 {
                    // Up movement
                    app_state.scroll_up();
                } else {
                    // Down movement
                    app_state.scroll_down();
                }
            } else if value > 0.0 {
                // Up movement
                app_state.next_log();
            } else {
                // Down movement
                app_state.prev_log();
            }
        }
        _ => {}
    }
}

/// Handle system events like hashing, downloads, and game execution
fn handle_system_event(app_state: &mut AppState, tx: &mpsc::Sender<Event>, event: Event) {
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
                app_state.log.add_titled(
                    "Hashes are the same",
                    "You have the latest version of the game.",
                );
            } else {
                app_state
                    .log
                    .add_titled("Hashes are different", "There is a newer version.");
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
            app_state.log.add_text("Local game binary not found");
        }
        Event::GameBinaryUpdated => {}
        Event::Launching => {
            app_state.log.add_text("Launching the game...");
        }
        Event::GameExecutionError(err) => {
            app_state.log.add_titled("Execution error", err);
        }
        Event::GameOutput(stdout) => {
            app_state.game_stdout.push(stdout);
        }
        Event::GameErrorOutput(stderr) => {
            app_state.game_stderr.push(stderr);
        }
        Event::LauncherError(err) => {
            app_state.log.add_titled("Error", err);
        }
        // Launcher update events
        Event::CheckingForLauncherUpdate => {
            app_state.log.launcher_status_msg = Some("checking for a newer version".into());
        }
        Event::LauncherUpdateAvailable(version) => {
            // Get the current version from our crate
            let current_version = crate::VERSION;
            app_state.log.launcher_status_msg = Some(format!(
                "an update is available {current_version} -> {version}"
            ));
            app_state.launcher_update_available = Some(version);
        }
        Event::LauncherNoUpdateAvailable => {
            // Include the current version in the status message
            let current_version = crate::VERSION;
            app_state.log.launcher_status_msg =
                Some(format!("already at the latest version - {current_version}"));
        }
        Event::StartDownloadingLauncherUpdate => {
            // Create a download entry specifically for the launcher update
            app_state.log.launcher_update = Some(crate::ui::log::Download::new(None));
        }
        Event::LauncherDownloadProgress(downloaded, total) => {
            if let Some(download) = &mut app_state.log.launcher_update {
                // Update the download progress
                download.set_progress(downloaded);

                // If we haven't set the total yet and it's now available, set it
                if download.total().is_none() && total.is_some() {
                    download.set_total(total);
                }
            }
        }
        Event::LauncherUpdateDownloaded => {
            if let Some(download) = &mut app_state.log.launcher_update {
                download.mark_complete();
            }
        }
        Event::LauncherApplyingUpdate => {
            app_state.log.launcher_status_msg = Some("applying update...".into());
        }
        Event::LauncherUpdateApplied => {
            app_state.log.launcher_status_msg =
                Some("update applied. Please restart the launcher.".into());
        }
        Event::RequestLauncherUpdate => {
            // Start the update process if an update is available and not already in progress
            if let Some(version) = &app_state.launcher_update_available {
                if app_state.update_status == UpdateStatus::NotRequested {
                    // Mark that an update is in progress
                    app_state.update_status = UpdateStatus::Requested;

                    // Clone the version since we need to move it into the thread
                    let version_clone = version.clone();

                    // Create a new thread to handle the download
                    let tx_clone = tx.clone();
                    thread::spawn(move || {
                        if let Err(e) = crate::update::update_launcher(&version_clone, &tx_clone) {
                            let _ = tx_clone.send(Event::LauncherError(format!(
                                "Failed to update launcher: {e}"
                            )));
                        }
                    });
                }
            }
        }
        _ => {}
    }
}
