//! Scans of transcript files, kept on disk so that the status line re-reads only the files
//! that changed. A day of work is about 100 MB of transcripts: about 1 s to parse, against
//! a few milliseconds to load the scans.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::transcript::{Scan, scan};

/// Bumped when `Scan` changes shape, so that old entries are ignored.
const VERSION: &str = "v1";
/// Entries untouched for this long belong to transcripts no status line reads any more.
const KEEP: Duration = Duration::from_secs(3 * 24 * 3600);

#[derive(Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
    len: u64,
    modified_ns: u128,
    scan: Scan,
}

fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("claude-nature").join(VERSION))
}

/// The scan of `file`, from the cache when its length and modification time match.
/// Any cache failure falls back to a fresh scan: the cache only saves time.
pub fn scan_cached(file: &Path) -> Scan {
    let Ok(meta) = fs::metadata(file) else {
        return Scan::default();
    };
    let len = meta.len();
    let modified_ns = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let Some(dir) = dir() else {
        return scan(file);
    };
    let mut hasher = DefaultHasher::new();
    file.hash(&mut hasher);
    let entry_path = dir.join(format!("{:016x}.json", hasher.finish()));
    if let Ok(bytes) = fs::read(&entry_path)
        && let Ok(entry) = serde_json::from_slice::<Entry>(&bytes)
        && entry.path == file
        && entry.len == len
        && entry.modified_ns == modified_ns
    {
        return entry.scan;
    }
    let fresh = scan(file);
    let entry = Entry {
        path: file.to_path_buf(),
        len,
        modified_ns,
        scan: fresh,
    };
    if fs::create_dir_all(&dir).is_ok()
        && let Ok(bytes) = serde_json::to_vec(&entry)
    {
        let tmp = entry_path.with_extension(format!("{}.tmp", std::process::id()));
        if fs::write(&tmp, bytes).is_ok() {
            let _ = fs::rename(&tmp, &entry_path);
        }
    }
    entry.scan
}

/// Removes the entries nobody read for `KEEP`.
pub fn prune() {
    let Some(dir) = dir() else { return };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > KEEP);
        if old {
            let _ = fs::remove_file(entry.path());
        }
    }
}
