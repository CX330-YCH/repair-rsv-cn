use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::{env, fs, io::Write, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_runtime();
    }
    tauri_build::build()
}

fn embed_windows_runtime() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let bin = root.join("bin");
    println!("cargo:rerun-if-changed={}", bin.display());
    let mut paths: Vec<_> = fs::read_dir(&bin)
        .expect("Prepare the Windows helper and its DLLs in bin/ before building the app")
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.is_file()
                && (path.file_name().unwrap() == "untrunc-rsv.exe"
                    || path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("dll")))
        })
        .collect();
    paths.sort();
    assert!(
        paths.iter().any(|path| path.ends_with("untrunc-rsv.exe")),
        "Missing bin/untrunc-rsv.exe"
    );
    for library in ["avcodec", "avformat", "avutil"] {
        assert!(
            paths.iter().any(|path| path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_lowercase()
                .contains(library)),
            "Missing FFmpeg runtime DLL: {library}"
        );
    }
    paths.extend([
        root.join("LICENSE"),
        root.join("NOTICE.md"),
        root.join("source/untrunc-rsv/COPYING"),
    ]);

    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut archive = GzEncoder::new(
        fs::File::create(out.join("windows-runtime.gz")).unwrap(),
        Compression::default(),
    );
    let mut identity = Sha256::new();
    let mut manifest = String::new();
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path.file_name().unwrap().to_str().unwrap();
        let contents = fs::read(&path).unwrap();
        if path.starts_with(&bin) {
            assert!(
                contents.starts_with(b"MZ"),
                "Not a Windows PE binary: {}",
                path.display()
            );
        }
        let digest: [u8; 32] = Sha256::digest(&contents).into();
        identity.update((name.len() as u64).to_le_bytes());
        identity.update(name.as_bytes());
        identity.update(digest);
        archive.write_all(&contents).unwrap();
        manifest.push_str(&format!(
            "    super::RuntimeFile {{ name: {name:?}, size: {}, digest: {digest:?} }},\n",
            contents.len()
        ));
    }
    archive.finish().unwrap();
    let identity = format!("{:x}", identity.finalize());
    fs::write(out.join("windows-runtime.rs"), format!(
        "pub const ID: &str = {identity:?};\npub static FILES: &[super::RuntimeFile] = &[\n{manifest}];\npub static ARCHIVE: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/windows-runtime.gz\"));\n"
    )).unwrap();
}
