use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PageStatus {
    Queued,
    Processing,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRecord {
    pub id: String,
    pub status: PageStatus,
    pub created_at: String,
    pub raw_path: Option<String>,
    pub processed_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Library {
    pub root: PathBuf,
}

impl Library {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("inbox")).context("create inbox")?;
        std::fs::create_dir_all(root.join("processed")).context("create processed")?;
        std::fs::create_dir_all(root.join("trash")).context("create trash")?;
        Ok(Self { root })
    }

    pub fn inbox_dir(&self) -> PathBuf {
        self.root.join("inbox")
    }

    pub fn processed_dir(&self) -> PathBuf {
        self.root.join("processed")
    }

    pub fn new_page_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn raw_path_for(&self, id: &str, ext: &str) -> PathBuf {
        self.inbox_dir().join(format!("{id}.{ext}"))
    }

    pub fn processed_path_for(&self, id: &str) -> PathBuf {
        self.processed_dir().join(format!("{id}.jpg"))
    }

    pub fn list_processed_sorted(&self) -> Result<Vec<PathBuf>> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.processed_dir())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("png") || e.eq_ignore_ascii_case("jpg"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort_by(|a, b| {
            let ma = a.metadata().and_then(|m| m.modified()).ok();
            let mb = b.metadata().and_then(|m| m.modified()).ok();
            ma.cmp(&mb)
        });
        Ok(files)
    }

    pub fn make_record(id: &str) -> PageRecord {
        PageRecord {
            id: id.to_string(),
            status: PageStatus::Queued,
            created_at: Utc::now().to_rfc3339(),
            raw_path: None,
            processed_path: None,
            error: None,
        }
    }
}
