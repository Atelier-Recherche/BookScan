//! Page file operations (delete / reorder) inspired by Livescan Preview.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use filetime::{set_file_mtime, FileTime};
use uuid::Uuid;

pub fn move_to_trash(path: &Path, library_root: &Path) -> Result<PathBuf> {
    let trash = library_root.join("trash");
    std::fs::create_dir_all(&trash)?;
    let mut dest = trash.join(
        path.file_name()
            .context("missing filename")?,
    );
    if dest.exists() {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
        let mut i = 1;
        loop {
            let candidate = trash.join(format!("{stem}_{i}.{ext}"));
            if !candidate.exists() {
                dest = candidate;
                break;
            }
            i += 1;
        }
    }
    std::fs::rename(path, &dest).with_context(|| format!("trash {}", path.display()))?;
    Ok(dest)
}

/// Delete a processed page and its matching inbox raw (same stem), if present.
pub fn delete_page(processed: &Path, library_root: &Path) -> Result<()> {
    let stem = processed
        .file_stem()
        .and_then(|s| s.to_str())
        .context("stem")?;
    move_to_trash(processed, library_root)?;
    let inbox = library_root.join("inbox");
    if let Ok(rd) = std::fs::read_dir(inbox) {
        for e in rd.flatten() {
            let p = e.path();
            if p.file_stem().and_then(|s| s.to_str()) == Some(stem) {
                let _ = move_to_trash(&p, library_root);
            }
        }
    }
    Ok(())
}

/// Swap filenames so `pages[index]` moves by `delta` (-1 earlier, +1 later).
///
/// Content is moved between stable filenames, then mtimes are rewritten so
/// `list_processed_sorted` (mtime order) reflects the new sequence.
pub fn move_page(pages: &[PathBuf], index: usize, delta: i32) -> Result<Vec<PathBuf>> {
    if pages.is_empty() {
        bail!("no pages");
    }
    let target = index as i32 + delta;
    if target < 0 || target as usize >= pages.len() {
        bail!("cannot move further");
    }
    let target = target as usize;
    let mut new_order = pages.to_vec();
    new_order.swap(index, target);
    let final_paths = permute_filenames(pages, &new_order)?;
    Ok(final_paths)
}

fn permute_filenames(old_order: &[PathBuf], new_order: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if old_order.len() != new_order.len() {
        bail!("length mismatch");
    }
    if old_order == new_order {
        return Ok(old_order.to_vec());
    }
    let names: Vec<_> = old_order
        .iter()
        .map(|p| p.file_name().map(|n| n.to_owned()).context("name"))
        .collect::<Result<Vec<_>>>()?;
    let parent = old_order[0].parent().context("parent")?.to_path_buf();
    let token = Uuid::new_v4().to_string();
    let mut temps = Vec::new();
    for (i, path) in new_order.iter().enumerate() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
        let temp = parent.join(format!(".tmpreorder_{token}_{i}.{ext}"));
        std::fs::rename(path, &temp).with_context(|| format!("temp {}", path.display()))?;
        temps.push(temp);
    }
    let mut final_paths = Vec::with_capacity(temps.len());
    for (i, temp) in temps.into_iter().enumerate() {
        let final_path = parent.join(&names[i]);
        std::fs::rename(&temp, &final_path)
            .with_context(|| format!("final {}", final_path.display()))?;
        final_paths.push(final_path);
    }

    // Encode order in mtime (list_processed sorts by modified time).
    let now = SystemTime::now();
    let base = now
        .checked_sub(Duration::from_secs(final_paths.len() as u64 + 2))
        .unwrap_or(now);
    for (i, path) in final_paths.iter().enumerate() {
        let t = base + Duration::from_secs(i as u64 + 1);
        set_file_mtime(path, FileTime::from_system_time(t))
            .with_context(|| format!("mtime {}", path.display()))?;
    }
    Ok(final_paths)
}
