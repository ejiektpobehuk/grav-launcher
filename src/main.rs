use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use gilrs::{Axis, EventType, Gilrs};

use crossterm::event as terminal_event;
use crossterm::event::Event as CrosstermEvent;
use crossterm::execute;

mod event;
use crate::event::Event;

mod app;
mod hash;
mod launcher;
mod ui;
mod update;

static BASE_URL: &str = "https://grav.arigven.games/builds/GRAV.x86_64";
static VERSION: &str = env!("CARGO_PKG_VERSION");
static REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

fn enable_focus_reporting() -> Result<()> {
    // Enable focus event reporting in terminal
    execute!(io::stdout(), terminal_event::EnableFocusChange)?;
    Ok(())
}

fn disable_focus_reporting() -> Result<()> {
    // Disable focus event reporting when exiting
    execute!(io::stdout(), terminal_event::DisableFocusChange)?;
    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, rx) = mpsc::channel();

    // Enable terminal focus event reporting
    enable_focus_reporting()?;

    // Initialize controller input handling
    controller_input_handling(tx.clone());

    // Initialize keyboard input handler
    input_handling(tx.clone());

    // Check for launcher update
    let update_tx = tx.clone();
    thread::spawn(move || {
        // Check for new updates
        let _ = update_tx.send(Event::CheckingForLauncherUpdate);
        match update::check_for_update(VERSION) {
            Ok(Some(version)) => {
                let _ = update_tx.send(Event::LauncherUpdateAvailable(version));
            }
            Ok(None) => {
                let _ = update_tx.send(Event::LauncherNoUpdateAvailable);
            }
            Err(e) => {
                let _ = update_tx.send(Event::LauncherError(format!(
                    "Failed to check for launcher updates: {e}"
                )));
            }
        }
    });

    let launcher_tx = tx.clone();
    let thread_join_handle = thread::spawn(move || launcher::launcher_logic(launcher_tx));

    let app_result = app::run(&mut terminal, &rx, tx);

    // Cleanup
    disable_focus_reporting()?;
    ratatui::restore();

    let _res = thread_join_handle.join();
    app_result
}

fn input_handling(tx: mpsc::Sender<Event>) {
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if let Ok(poll_ready) = terminal_event::poll(timeout) {
                if poll_ready {
                    match terminal_event::read() {
                        Ok(event) => {
                            let send_result = match event {
                                CrosstermEvent::Key(key) => tx.send(Event::Input(key)),
                                CrosstermEvent::Resize(_, _) => tx.send(Event::Resize),
                                CrosstermEvent::FocusGained => {
                                    tx.send(Event::TerminalFocusChanged(true))
                                }
                                CrosstermEvent::FocusLost => {
                                    tx.send(Event::TerminalFocusChanged(false))
                                }
                                _ => Ok(()),
                            };

                            if send_result.is_err() {
                                eprintln!(
                                    "Terminal event receiver disconnected, shutting down input thread"
                                );
                                return;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading terminal event: {e}");
                        }
                    }
                }
            } else {
                eprintln!("Error polling terminal events");
            }

            if last_tick.elapsed() >= tick_rate {
                if tx.send(Event::Tick).is_err() {
                    eprintln!("Tick event receiver disconnected, shutting down input thread");
                    return;
                }
                last_tick = Instant::now();
            }
        }
    });
}

fn controller_input_handling(tx: mpsc::Sender<Event>) {
    thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(e) => {
                eprintln!("Failed to initialize gilrs: {e}");
                return;
            }
        };

        // Define threshold values for the stick movement hysteresis
        const HIGH_THRESHOLD: f32 = 0.5; // Consider triggered when exceeding this value
        const LOW_THRESHOLD: f32 = 0.2; // Must return below this value to reset

        // Track the "triggered" state of each direction
        let mut left_triggered = false;
        let mut right_triggered = false;
        let mut up_triggered = false;
        let mut down_triggered = false;

        loop {
            // Process controller events
            while let Some(gilrs_event) = gilrs.next_event() {
                match gilrs_event.event {
                    EventType::ButtonPressed(button, _) => {
                        if tx.send(Event::ControllerInput(button)).is_err() {
                            eprintln!(
                                "Controller event receiver disconnected, shutting down controller thread"
                            );
                            return;
                        }
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        match axis {
                            Axis::LeftStickX => {
                                // Handle horizontal stick movement
                                if value > HIGH_THRESHOLD && !right_triggered {
                                    // Right movement crossing high threshold
                                    right_triggered = true;
                                    if tx.send(Event::ControllerAxisMoved(axis, value)).is_err() {
                                        return;
                                    }
                                } else if value < -HIGH_THRESHOLD && !left_triggered {
                                    // Left movement crossing high threshold
                                    left_triggered = true;
                                    if tx.send(Event::ControllerAxisMoved(axis, value)).is_err() {
                                        return;
                                    }
                                } else if value.abs() < LOW_THRESHOLD {
                                    // Reset triggered state when returning to neutral
                                    left_triggered = false;
                                    right_triggered = false;
                                }
                            }
                            Axis::LeftStickY => {
                                // Handle vertical stick movement
                                if value > HIGH_THRESHOLD && !down_triggered {
                                    // Down movement crossing high threshold
                                    down_triggered = true;
                                    if tx.send(Event::ControllerAxisMoved(axis, value)).is_err() {
                                        return;
                                    }
                                } else if value < -HIGH_THRESHOLD && !up_triggered {
                                    // Up movement crossing high threshold
                                    up_triggered = true;
                                    if tx.send(Event::ControllerAxisMoved(axis, value)).is_err() {
                                        return;
                                    }
                                } else if value.abs() < LOW_THRESHOLD {
                                    // Reset triggered state when returning to neutral
                                    up_triggered = false;
                                    down_triggered = false;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            // Sleep to prevent high CPU usage
            thread::sleep(Duration::from_millis(10));
        }
    });
}
