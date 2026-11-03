use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/windows-runtime.rs"));
}

struct RuntimeFile {
    name: &'static str,
    size: u64,
    digest: [u8; 32],
}

#[cfg(target_os = "windows")]
static HELPER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
#[cfg(target_os = "windows")]
static PREPARING: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(target_os = "windows")]
pub(crate) fn ready_helper_path() -> Result<PathBuf, String> {
    HELPER
        .get()
        .cloned()
        .ok_or_else(|| "The repair engine is not ready yet.".to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn prepare_helper_path(cache: &Path) -> Result<PathBuf, String> {
    let _guard = PREPARING
        .lock()
        .map_err(|_| "Engine preparation is locked")?;
    if let Some(helper) = HELPER.get() {
        return Ok(helper.clone());
    }
    let helper = prepare_runtime(cache, embedded::ID, embedded::FILES, embedded::ARCHIVE)
        .map(|directory| directory.join("untrunc-rsv.exe"))
        .map_err(|error| format!("Could not prepare the embedded repair engine: {error}"))?;
    let _ = HELPER.set(helper.clone());
    Ok(helper)
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct FileStamp {
    name: String,
    size: u64,
    modified_seconds: u64,
    modified_nanos: u32,
}

fn file_stamps(directory: &Path, files: &[RuntimeFile]) -> io::Result<Vec<FileStamp>> {
    files
        .iter()
        .map(|entry| {
            let info = fs::symlink_metadata(directory.join(entry.name))?;
            if !info.is_file() || info.len() != entry.size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Engine file changed",
                ));
            }
            let modified = info
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?;
            Ok(FileStamp {
                name: entry.name.to_string(),
                size: entry.size,
                modified_seconds: modified.as_secs(),
                modified_nanos: modified.subsec_nanos(),
            })
        })
        .collect()
}

fn fast_valid_runtime(directory: &Path, files: &[RuntimeFile]) -> bool {
    let Ok(marker) = fs::read(directory.join(".ready.json")) else {
        return false;
    };
    let Ok(saved) = serde_json::from_slice::<Vec<FileStamp>>(&marker) else {
        return false;
    };
    file_stamps(directory, files).is_ok_and(|current| current == saved)
}

fn mark_ready(directory: &Path, files: &[RuntimeFile]) -> io::Result<()> {
    let stamps = file_stamps(directory, files)?;
    fs::write(directory.join(".ready.json"), serde_json::to_vec(&stamps)?)
}

fn reuse_runtime(directory: &Path, files: &[RuntimeFile]) -> io::Result<bool> {
    if fast_valid_runtime(directory, files) {
        return Ok(true);
    }
    // Re-hash only when the small metadata record is missing or changed.
    if valid_runtime(directory, files) {
        mark_ready(directory, files)?;
        return Ok(true);
    }
    Ok(false)
}

fn valid_runtime(directory: &Path, files: &[RuntimeFile]) -> bool {
    files.iter().all(|entry| {
        let Ok(mut file) = File::open(directory.join(entry.name)) else {
            return false;
        };
        if file.metadata().map(|info| info.len()).ok() != Some(entry.size) {
            return false;
        }
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => return <[u8; 32]>::from(digest.finalize()) == entry.digest,
                Ok(count) => digest.update(&buffer[..count]),
                Err(_) => return false,
            }
        }
    })
}

fn prepare_runtime(
    cache: &Path,
    id: &str,
    files: &[RuntimeFile],
    archive: &[u8],
) -> io::Result<PathBuf> {
    let parent = cache.join("engine");
    let directory = parent.join(id);
    if reuse_runtime(&directory, files)? {
        return Ok(directory);
    }
    fs::create_dir_all(&parent)?;
    let staging = parent.join(format!("{id}-staging-{}", std::process::id()));
    fs::create_dir_all(&staging)?;
    let result = (|| {
        let mut decoder = GzDecoder::new(archive);
        for entry in files {
            let mut output = File::create(staging.join(entry.name))?;
            let mut digest = Sha256::new();
            let mut copied = 0;
            let mut limited = decoder.by_ref().take(entry.size);
            let mut buffer = [0u8; 65536];
            loop {
                let count = limited.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
                digest.update(&buffer[..count]);
                copied += count as u64;
            }
            output.flush()?;
            if copied != entry.size {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Incomplete embedded engine",
                ));
            }
            if <[u8; 32]>::from(digest.finalize()) != entry.digest {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Embedded engine checksum mismatch",
                ));
            }
        }
        // Read to EOF to verify the gzip trailer/checksum as well.
        let mut remainder = [0u8; 1];
        if decoder.read(&mut remainder)? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Embedded engine checksum mismatch",
            ));
        }
        mark_ready(&staging, files)?;
        if directory.exists() {
            if reuse_runtime(&directory, files)? {
                return Ok(directory.clone()); // Another app instance finished extracting it.
            }
            fs::remove_dir_all(&directory)?;
        }
        match fs::rename(&staging, &directory) {
            Ok(()) => Ok(directory.clone()),
            Err(_) if fast_valid_runtime(&directory, files) => Ok(directory.clone()),
            Err(error) => Err(error),
        }
    })();
    if staging.exists() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn extracts_in_a_unicode_path_reuses_and_repairs_cache() {
        let root = std::env::temp_dir().join(format!(
            "RSV 便携测试-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let files = [
            RuntimeFile {
                name: "untrunc-rsv.exe",
                size: 6,
                digest: Sha256::digest(b"helper").into(),
            },
            RuntimeFile {
                name: "avcodec.dll",
                size: 7,
                digest: Sha256::digest(b"library").into(),
            },
        ];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"helperlibrary").unwrap();
        let archive = encoder.finish().unwrap();
        let directory = prepare_runtime(&root, "fixture", &files, &archive).unwrap();
        assert!(fast_valid_runtime(&directory, &files));
        assert_eq!(
            fs::read(directory.join("untrunc-rsv.exe")).unwrap(),
            b"helper"
        );
        // An already valid cache does not need to decode the payload again.
        assert_eq!(
            prepare_runtime(&root, "fixture", &files, &[]).unwrap(),
            directory
        );
        fs::write(directory.join("avcodec.dll"), b"damaged").unwrap();
        File::options()
            .write(true)
            .open(directory.join("avcodec.dll"))
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1)),
            )
            .unwrap();
        assert!(!fast_valid_runtime(&directory, &files));
        prepare_runtime(&root, "fixture", &files, &archive).unwrap();
        assert!(valid_runtime(&directory, &files));
        // A missing completion marker falls back to a full check of the existing cache.
        fs::remove_file(directory.join(".ready.json")).unwrap();
        prepare_runtime(&root, "fixture", &files, &[]).unwrap();
        assert!(fast_valid_runtime(&directory, &files));
        assert!(
            prepare_runtime(&root, "truncated", &files, &archive[..archive.len() / 2]).is_err()
        );
        assert!(!root.join("engine/truncated").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
