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
    pub fn entries(&self) -> Vec<String> {
        let mut accumulator: Vec<String> = Vec::new();
        if let Some(remote_hash) = &self.remote_hash_msg {
            accumulator.push(format!("Remote hash: {remote_hash}"));
        }
        if let Some(local_hash) = &self.local_hash_msg {
            accumulator.push(format!("Local hash:  {local_hash}"));
        }
        match &self.download.status {
            DownloadStatus::NotStarted => {}
            DownloadStatus::InProgress => {
                if let Some(total) = self.download.total {
                    accumulator.push(format!("Downloading: {0}/{total}", self.download.current));
                } else {
                    accumulator.push(format!("Downloading: {0}", self.download.current));
                }
            }
            DownloadStatus::Comple => {
                accumulator.push(format!("Download complete: {0}", self.download.current));
            }
            DownloadStatus::Errored(err) => {
                accumulator.push(format!("Download error: {err}"));
            }
        }
        let mut extra_log_clone = self.extra_log.clone();
        accumulator.append(&mut extra_log_clone);
        accumulator
    }
    pub fn start_download(&mut self, total: Option<u64>) {
        self.download.status = DownloadStatus::InProgress;
        self.download.total = total;
    }
    pub fn set_download_progress(&mut self, downloaded: u64) {
        self.download.current = downloaded;
    }
    pub fn mark_download_complete(&mut self) {
        self.download.status = DownloadStatus::Comple;
    }
    pub fn set_download_error(&mut self, error: String) {
        self.download.status = DownloadStatus::Errored(error)
    }
}

struct Download {
    total: Option<u64>,
    current: u64,
    status: DownloadStatus,
}

enum DownloadStatus {
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
}
