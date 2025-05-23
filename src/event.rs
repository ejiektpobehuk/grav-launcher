use crossterm::event as terminal_event;
use gilrs::{Axis, Button};

type FileSize = u64;
// type Percentage = f64;
pub enum Event {
    Input(terminal_event::KeyEvent),
    ControllerInput(Button),
    ControllerAxisMoved(Axis, f32),
    TerminalFocusChanged(bool),
    Tick,
    Resize,
    AccessingOnlineHash,
    OfflineError(String),
    RemoteHash(String),
    LocalHash(String),
    ComputingLocalHash,
    HashAreEqual(bool),
    StartDownloadingBinary(Option<FileSize>),
    DownloadProgress(FileSize),
    BinaryDownloadError(String),
    RemoteBinaryDownloaded,
    NoLocalBinaryFound,
    GameExecutionError(String),
    GameBinaryUpdated,
    Launching,
    GameOutput(String),
    GameErrorOutput(String),
    LauncherError(String),
    // Launcher update events
    CheckingForLauncherUpdate,
    LauncherUpdateAvailable(String),
    LauncherNoUpdateAvailable,
    StartDownloadingLauncherUpdate,
    LauncherDownloadProgress(FileSize, Option<FileSize>),
    LauncherUpdateDownloaded,
    LauncherApplyingUpdate,
    LauncherUpdateApplied,
    RequestLauncherUpdate,
}
