use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;
use gilrs::{Button, Event as GilrsEvent, EventType, Gilrs};

use crossterm::event as terminal_event;

mod event;
use crate::event::Event;

mod app;
mod hash;
mod launcher;
mod ui;

static BASE_URL: &str = "https://grav.arigven.games/builds/GRAV.x86_64";

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, rx) = mpsc::channel();

    // Initialize controller input handler
    controller_input_handling(tx.clone());

    // Initialize keyboard input handler
    input_handling(tx.clone());

    let thread_join_handle = thread::spawn(move || launcher::launcher_logic(tx));

    let app_result = app::run(&mut terminal, &rx);

    ratatui::restore();

    let _res = thread_join_handle.join();
    app_result
}

fn controller_input_handling(tx: mpsc::Sender<Event>) {
    thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(e) => {
                eprintln!("Failed to initialize gilrs: {}", e);
                return;
            }
        };

        loop {
            // Process controller events
            while let Some(gilrs_event) = gilrs.next_event() {
                if let EventType::ButtonPressed(button, _) = gilrs_event.event {
                    tx.send(Event::ControllerInput(button)).unwrap();
                }
            }

            // Sleep to prevent high CPU usage
            thread::sleep(Duration::from_millis(10));
        }
    });
}

fn input_handling(tx: mpsc::Sender<Event>) {
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if terminal_event::poll(timeout).unwrap() {
                match terminal_event::read().unwrap() {
                    terminal_event::Event::Key(key) => tx.send(Event::Input(key)).unwrap(),
                    terminal_event::Event::Resize(_, _) => tx.send(Event::Resize).unwrap(),
                    _ => {}
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });
}
