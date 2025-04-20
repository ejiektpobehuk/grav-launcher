use crossterm::event as terminal_event;

type FileSize = u64;
// type Percentage = f64;
pub enum Event {
    Input(terminal_event::KeyEvent),
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
}
