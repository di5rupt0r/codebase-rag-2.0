use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFileInfo {
    pub relative_path: String,
    pub sha256: String,
    pub mtime_secs: u64,
    pub size_bytes: u64,
    pub chunk_count: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileChanges {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub deleted: Vec<String>,
}

impl FileChanges {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct FileTracker {
    files: HashMap<String, TrackedFileInfo>,
    #[serde(skip)]
    file_path: Option<PathBuf>,
}

impl FileTracker {
    pub fn open_or_create<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("file_tracker.json");

        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            let mut tracker: Self = serde_json::from_reader(reader)?;
            tracker.file_path = Some(path);
            Ok(tracker)
        } else {
            Ok(Self {
                files: HashMap::new(),
                file_path: Some(path),
            })
        }
    }

    pub fn compute_sha256(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    pub fn detect_changes(
        &self,
        project_root: &Path,
        current_files: &[PathBuf],
    ) -> Result<FileChanges> {
        let mut changes = FileChanges::default();
        let mut current_relative_paths = HashSet::new();

        for path in current_files {
            let rel_path = match path.strip_prefix(project_root) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => path.to_string_lossy().to_string(),
            };

            current_relative_paths.insert(rel_path.clone());

            let metadata = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let mtime_secs = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let size = metadata.len();

            if let Some(tracked) = self.files.get(&rel_path) {
                if tracked.mtime_secs != mtime_secs || tracked.size_bytes != size {
                    let content = std::fs::read(path)?;
                    let hash = Self::compute_sha256(&content);
                    if hash != tracked.sha256 {
                        changes.modified.push(path.clone());
                    }
                }
            } else {
                changes.added.push(path.clone());
            }
        }

        for tracked_rel_path in self.files.keys() {
            if !current_relative_paths.contains(tracked_rel_path) {
                changes.deleted.push(tracked_rel_path.clone());
            }
        }

        Ok(changes)
    }

    pub fn record_file(
        &mut self,
        relative_path: &str,
        sha256: String,
        mtime_secs: u64,
        size_bytes: u64,
        chunk_count: usize,
    ) {
        self.files.insert(
            relative_path.to_string(),
            TrackedFileInfo {
                relative_path: relative_path.to_string(),
                sha256,
                mtime_secs,
                size_bytes,
                chunk_count,
            },
        );
    }

    pub fn remove_file(&mut self, relative_path: &str) {
        self.files.remove(relative_path);
    }

    pub fn save(&self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, self)?;
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }

    pub fn total_chunks(&self) -> usize {
        self.files.values().map(|f| f.chunk_count).sum()
    }
}
