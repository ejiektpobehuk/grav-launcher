pub struct Log {
    pub local_hash_msg: Option<String>,
    pub remote_hash_msg: Option<String>,
    pub launcher_status_msg: Option<String>,
    pub game_download: Option<Download>,
    pub launcher_update: Option<Download>,
    pub extra_log: Vec<String>,
}

impl Log {
    pub const fn new() -> Self {
        Self {
            local_hash_msg: None,
            remote_hash_msg: None,
            launcher_status_msg: None,
            game_download: None,
            launcher_update: None,
            extra_log: Vec::new(),
        }
    }
    pub fn push(&mut self, string: String) {
        self.extra_log.push(string);
    }
    pub fn entries(&self) -> Vec<Entry> {
        let mut accumulator: Vec<Entry> = Vec::new();

        // Add launcher status message if present
        if let Some(status) = &self.launcher_status_msg {
            accumulator.push(status.clone().into());
        }

        // Add hash information
        if let Some(remote_hash) = &self.remote_hash_msg {
            accumulator.push(format!("Remote hash: {remote_hash}").into());
        }
        if let Some(local_hash) = &self.local_hash_msg {
            accumulator.push(format!("Local hash:  {local_hash}").into());
        }

        // Add launcher update download status if present
        if let Some(launcher_update) = &self.launcher_update {
            // Create a special LauncherUpdate entry for formatting
            accumulator.push(Entry::LauncherUpdate(launcher_update.clone()));
        }

        // Add game download status if present
        if let Some(game_download) = &self.game_download {
            // Create a special GameDownload entry for formatting
            accumulator.push(Entry::GameDownload(game_download.clone()));
        }

        // Add all other log entries
        let extra_log_clone = self.extra_log.clone();
        accumulator.append(
            &mut extra_log_clone
                .iter()
                .map(std::convert::Into::into)
                .collect(),
        );
        accumulator
    }
    pub fn start_download(&mut self, total: Option<u64>) {
        self.game_download = Some(Download::new(total));
    }
    pub const fn set_download_progress(&mut self, downloaded: u64) {
        if let Some(download) = &mut self.game_download {
            download.set_progress(downloaded);
        }
    }
    pub fn mark_download_complete(&mut self) {
        if let Some(download) = &mut self.game_download {
            download.mark_complete();
        }
    }
    pub fn set_download_error(&mut self, error: String) {
        if let Some(download) = &mut self.game_download {
            download.set_error(error);
        }
    }
}

pub enum Entry {
    Text(String),
    Downloand(Download),
    LauncherUpdate(Download),
    GameDownload(Download),
}

impl From<String> for Entry {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}
impl From<&String> for Entry {
    fn from(text: &String) -> Self {
        Self::Text(text.clone())
    }
}
impl From<Download> for Entry {
    fn from(download: Download) -> Self {
        Self::Downloand(download)
    }
}

#[derive(Clone)]
pub struct Download {
    pub total: Option<u64>,
    pub current: u64,
    pub status: DownloadStatus,
}

#[derive(Clone)]
pub enum DownloadStatus {
    InProgress,
    Comple,
    Errored(String),
}

impl Download {
    // Create a new Download with the given total size
    pub const fn new(total: Option<u64>) -> Self {
        Self {
            total,
            current: 0,
            status: DownloadStatus::InProgress,
        }
    }

    pub const fn current(&self) -> u64 {
        self.current
    }
    pub const fn status(&self) -> &DownloadStatus {
        &self.status
    }
    pub const fn total(&self) -> &Option<u64> {
        &self.total
    }

    pub const fn set_progress(&mut self, current: u64) {
        self.current = current;
    }

    pub const fn set_total(&mut self, total: Option<u64>) {
        self.total = total;
    }

    pub fn mark_complete(&mut self) {
        self.status = DownloadStatus::Comple;
    }

    pub fn set_error(&mut self, error: String) {
        self.status = DownloadStatus::Errored(error);
    }
}
