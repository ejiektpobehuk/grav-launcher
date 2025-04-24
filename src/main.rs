use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use gilrs::{EventType, Gilrs};

use crossterm::event as terminal_event;
use crossterm::event::Event as CrosstermEvent;
use crossterm::execute;

mod event;
use crate::event::Event;

mod app;
mod hash;
mod launcher;
mod ui;

static BASE_URL: &str = "https://grav.arigven.games/builds/GRAV.x86_64";

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

    // Initialize controller input handler
    controller_input_handling(tx.clone());

    // Initialize keyboard input handler
    input_handling(tx.clone());

    let thread_join_handle = thread::spawn(move || launcher::launcher_logic(tx));

    let app_result = app::run(&mut terminal, &rx);

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

        loop {
            // Process controller events
            while let Some(gilrs_event) = gilrs.next_event() {
                if let EventType::ButtonPressed(button, _) = gilrs_event.event {
                    if tx.send(Event::ControllerInput(button)).is_err() {
                        eprintln!(
                            "Controller event receiver disconnected, shutting down controller thread"
                        );
                        return;
                    }
                }
            }

            // Sleep to prevent high CPU usage
            thread::sleep(Duration::from_millis(10));
        }
    });
}
