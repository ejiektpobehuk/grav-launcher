use eyre::WrapErr;
use sha2::{Digest, Sha256};
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::io::{BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use color_eyre::Result;

use crossterm::event;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

static BASE_URL: &'static str = "https://grav.arigven.games/builds/GRAV.x86_64";

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let (tx, rx) = mpsc::channel();
    input_handling(tx.clone());

    let thread_join_handle = thread::spawn(move || launcher_logic(tx.clone()));

    let app_result = run(&mut terminal, rx);

    ratatui::restore();

    let res = thread_join_handle.join();
    app_result
}

fn input_handling(tx: mpsc::Sender<Event>) {
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout).unwrap() {
                match event::read().unwrap() {
                    event::Event::Key(key) => tx.send(Event::Input(key)).unwrap(),
                    event::Event::Resize(_, _) => tx.send(Event::Resize).unwrap(),
                    _ => {}
                };
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });
}

type FileSize = f64;
// type Percentage = f64;
enum Event {
    Input(event::KeyEvent),
    Tick,
    Resize,
    AccessingOnlineHash,
    OfflineError(String),
    RemoteHash(String),
    LocalHash(String),
    // Progress(Percentage),
    ComputinLocalHash,
    HashAreEqual(bool),
    DownloadingBinary(FileSize),
    BinaryDownloadError(String),
    RemoteBinaryDownloaded,
    NoLocalBinaryFound,
    GameExecutionError(String),
    GameBinaryUpdated,
    Launching,
    GameOutput(String),
    GameErrorOutput(String),
}

struct Log {
    local_hash_msg: Option<String>,
    remote_hash_msg: Option<String>,
    download_msg: Option<String>,
    extra_log: Vec<String>,
}

impl Log {
    fn new() -> Self {
        Self {
            local_hash_msg: None,
            remote_hash_msg: None,
            download_msg: None,
            extra_log: Vec::new(),
        }
    }
    fn push(&mut self, string: String) {
        self.extra_log.push(string);
    }
    fn values(&self) -> Vec<String> {
        let mut accumulator: Vec<String> = Vec::new();
        if let Some(remote_hash) = &self.remote_hash_msg {
            accumulator.push(format!("Remote hash: {remote_hash}"));
        }
        if let Some(local_hash) = &self.local_hash_msg {
            accumulator.push(format!("Local hash:  {local_hash}"));
        }
        if let Some(download) = &self.download_msg {
            accumulator.push(download.to_string());
        }
        let mut extra_log_clone = self.extra_log.clone();
        accumulator.append(&mut extra_log_clone);
        accumulator
    }
}

struct AppState {
    log: Log,
    game_stdout: Vec<String>,
    game_stderr: Vec<String>,
    list_state: ListState,
    stdout_state: ListState,
    stderr_state: ListState,
}

fn run(terminal: &mut Terminal<impl Backend>, rx: mpsc::Receiver<Event>) -> Result<()> {
    let mut app_state = AppState {
        log: Log::new(),
        game_stdout: Vec::new(),
        game_stderr: Vec::new(),
        list_state: ListState::default(),
        stdout_state: ListState::default(),
        stderr_state: ListState::default(),
    };
    loop {
        terminal.draw(|frame| draw(frame, &mut app_state))?;
        match rx.recv()? {
            Event::Input(event) => {
                if event.code == event::KeyCode::Char('q') {
                    break;
                }
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
            Event::ComputinLocalHash => {
                app_state.log.local_hash_msg = Some("Computing".into());
            }
            Event::LocalHash(hash_value) => {
                app_state.log.local_hash_msg = Some(hash_value);
            }
            Event::HashAreEqual(eq) => match eq {
                true => app_state
                    .log
                    .push("Hashes are the same: You have the latest verstion of the game. ".into()),
                false => app_state
                    .log
                    .push("Hashes are different: There is a newer version.".into()),
            },
            Event::DownloadingBinary(_) => {
                app_state.log.push(format!(
                    "Downloading a new binary: a file size should be here"
                ));
            }
            Event::RemoteBinaryDownloaded => {
                app_state.log.push(format!(
                    "Downloading a new binary: a file size should be here"
                ));
            }
            Event::BinaryDownloadError(err) => {
                app_state
                    .log
                    .push(format!("Unable to download a remote binary: {err}"));
            }
            Event::NoLocalBinaryFound => {
                app_state.log.push(format!("Local game binary not found"));
            }
            Event::GameBinaryUpdated => {}
            Event::Launching => {
                app_state.log.push(format!("Launcning the game. . ."));
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
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app_state: &mut AppState) {
    let area = frame.area();

    let outer_layout = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area);

    let title = Line::from(" GRAV launcher ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);
    frame.render_widget(block, area);

    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(vec![Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer_layout[0]);

    let game_output_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_layout[1]);

    let items: Vec<ListItem> = app_state
        .log
        .values()
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(format!("{i}")));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Launcher log ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let list = List::new(items).block(block);

    frame.render_stateful_widget(list, inner_layout[0], &mut app_state.list_state);

    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(format!("{i}")));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game text output ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let stdout = List::new(stdouts).block(block);

    frame.render_stateful_widget(stdout, game_output_layout[0], &mut app_state.stdout_state);

    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(format!("{i}")));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game errors ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let stderr = List::new(stderrs).block(block);

    frame.render_stateful_widget(stderr, game_output_layout[1], &mut app_state.stderr_state);
}

fn launcher_logic(tx: mpsc::Sender<Event>) {
    tx.send(Event::AccessingOnlineHash).unwrap();
    match get_remote_hash(&BASE_URL) {
        Ok(remote_version_hash) => {
            tx.send(Event::RemoteHash(remote_version_hash.clone()))
                .unwrap();
            tx.send(Event::ComputinLocalHash).unwrap();
            match get_local_hash() {
                Some((local_version_hash, game_path)) => {
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
                None => {
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
            };
        }
        Err(e) => {
            tx.send(Event::OfflineError(format!("{e}"))).unwrap();
            let xdg_dirs = xdg::BaseDirectories::with_prefix("GRAV").unwrap();
            match xdg_dirs.find_data_file("GRAV.x86_64") {
                Some(game_binary_path) => {
                    run_the_game(game_binary_path, tx.clone());
                }
                None => {
                    tx.send(Event::NoLocalBinaryFound).unwrap();
                    println!("Game binary not found");
                }
            }
        }
    }
}

fn get_remote_hash(base_url: &str) -> Result<String> {
    let sha_url = format!("{base_url}.sha256");
    let current_version_hash_body = reqwest::blocking::get(sha_url)?.text()?;
    Ok(current_version_hash_body.trim().to_string())
}

fn get_local_hash() -> Option<(String, PathBuf)> {
    // Specify the file path
    let xdg_dirs = xdg::BaseDirectories::with_prefix("GRAV").unwrap();

    match xdg_dirs.find_data_file("GRAV.x86_64") {
        Some(game_binary_path) => {
            // Open the file in read-only mode
            let file = File::open(&game_binary_path).unwrap();
            let mut reader = BufReader::new(file);

            // Create a Sha256 object
            let mut hasher = Sha256::new();

            // Read the file in chunks
            let mut buffer = [0; 1024];
            loop {
                let bytes_read = reader.read(&mut buffer).unwrap();
                if bytes_read == 0 {
                    break;
                }
                // Feed the contents of the buffer into the hasher
                hasher.update(&buffer[..bytes_read]);
            }

            // Retrieve the final hash
            let result = hasher.finalize();
            Some((format!("{result:x}"), game_binary_path))
        }
        None => {
            println!("Game binary not found");
            None
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

fn run_the_game(game_path: PathBuf, tx: mpsc::Sender<Event>) -> () {
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
                tx_clone.send(Event::GameOutput(format!("{line}"))).unwrap();
            }
        }
    });

    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let tx_clone = tx.clone();
    let stderr_thread = thread::spawn(move || {
        let stderr_reader = BufReader::new(stderr);
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                tx.send(Event::GameErrorOutput(format!("{line}"))).unwrap();
            }
        }
    });
}

fn check_exec_permissions(binary_path: &PathBuf) -> Result<()> {
    let permissions = fs::Permissions::from_mode(0o744);
    let _ = fs::set_permissions(&binary_path, permissions);
    Ok(())
}
