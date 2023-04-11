use std::fs::Metadata;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub metadata: Option<Metadata>,
}

impl File {
    pub fn new(file_path: PathBuf, with_metadata: bool) -> Self {
        let metadata = if with_metadata {
            file_path.metadata().ok()
        } else {
            None
        };

        Self {
            path: file_path,
            metadata,
        }
    }
}
