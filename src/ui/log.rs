pub struct Log {
    pub local_hash_msg: Option<String>,
    pub remote_hash_msg: Option<String>,
    download: Download,
    pub extra_log: Vec<String>,
}

impl Log {
    pub const fn new() -> Self {
        Self {
            local_hash_msg: None,
            remote_hash_msg: None,
            download: Download::new(),
            extra_log: Vec::new(),
        }
    }
    pub fn push(&mut self, string: String) {
        self.extra_log.push(string);
    }
    pub fn entries(&self) -> Vec<Entry> {
        let mut accumulator: Vec<Entry> = Vec::new();
        if let Some(remote_hash) = &self.remote_hash_msg {
            accumulator.push(format!("Remote hash: {remote_hash}").into());
        }
        if let Some(local_hash) = &self.local_hash_msg {
            accumulator.push(format!("Local hash:  {local_hash}").into());
        }
        match &self.download.status {
            DownloadStatus::NotStarted => {}
            _ => {
                accumulator.push(self.download.clone().into());
            }
        }
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
        self.download.status = DownloadStatus::InProgress;
        self.download.total = total;
    }
    pub const fn set_download_progress(&mut self, downloaded: u64) {
        self.download.current = downloaded;
    }
    pub fn mark_download_complete(&mut self) {
        self.download.status = DownloadStatus::Comple;
    }
    pub fn set_download_error(&mut self, error: String) {
        self.download.status = DownloadStatus::Errored(error);
    }
}

pub enum Entry {
    Text(String),
    Downloand(Download),
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
    total: Option<u64>,
    current: u64,
    status: DownloadStatus,
}

#[derive(Clone)]
pub enum DownloadStatus {
    NotStarted,
    InProgress,
    Comple,
    Errored(String),
}

impl Download {
    const fn new() -> Self {
        Self {
            total: None,
            current: 0,
            status: DownloadStatus::NotStarted,
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
}
