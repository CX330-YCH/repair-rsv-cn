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
pub(crate) fn helper_path(cache: &Path) -> Result<PathBuf, String> {
    use std::sync::OnceLock;
    static HELPER: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    HELPER
        .get_or_init(|| {
            prepare_runtime(cache, embedded::ID, embedded::FILES, embedded::ARCHIVE)
                .map(|directory| directory.join("untrunc-rsv.exe"))
                .map_err(|error| format!("Could not prepare the embedded repair engine: {error}"))
        })
        .clone()
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
    if valid_runtime(&directory, files) {
        return Ok(directory);
    }
    fs::create_dir_all(&parent)?;
    let staging = parent.join(format!("{id}-staging-{}", std::process::id()));
    fs::create_dir_all(&staging)?;
    let result = (|| {
        let mut decoder = GzDecoder::new(archive);
        for entry in files {
            let mut output = File::create(staging.join(entry.name))?;
            let copied = io::copy(&mut decoder.by_ref().take(entry.size), &mut output)?;
            output.flush()?;
            if copied != entry.size {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Incomplete embedded engine",
                ));
            }
        }
        // Read to EOF to verify the gzip trailer/checksum as well.
        let mut remainder = [0u8; 1];
        if decoder.read(&mut remainder)? != 0 || !valid_runtime(&staging, files) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Embedded engine checksum mismatch",
            ));
        }
        if directory.exists() {
            if valid_runtime(&directory, files) {
                return Ok(directory.clone()); // Another app instance finished extracting it.
            }
            fs::remove_dir_all(&directory)?;
        }
        match fs::rename(&staging, &directory) {
            Ok(()) => Ok(directory.clone()),
            Err(_) if valid_runtime(&directory, files) => Ok(directory.clone()),
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
        prepare_runtime(&root, "fixture", &files, &archive).unwrap();
        assert!(valid_runtime(&directory, &files));
        assert!(
            prepare_runtime(&root, "truncated", &files, &archive[..archive.len() / 2]).is_err()
        );
        assert!(!root.join("engine/truncated").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
