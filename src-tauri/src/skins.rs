use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    os::windows::fs::MetadataExt,
    path::Path,
};
const MAX_FILE: u64 = 8 * 1024 * 1024;
const MAX_TOTAL: u64 = 32 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    pub file: String,
    pub frames: u32,
    pub fps: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub license: String,
    pub asset_scale: f64,
    pub animations: BTreeMap<String, Animation>,
    pub preview: String,
    pub trail: String,
}
#[derive(Serialize)]
pub struct Bundle {
    pub manifest: Manifest,
    pub assets: BTreeMap<String, String>,
}
fn safe_name(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 100
        || name.contains(['/', '\\', ':'])
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return false;
    }
    let base = name.split('.').next().unwrap_or("").to_ascii_lowercase();
    ![
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ]
    .contains(&base.as_str())
}
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "builtin"
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && safe_name(id)
}
fn read_bounded(mut source: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut b = Vec::new();
    source
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut b)
        .map_err(|_| "skin_read")?;
    if b.len() as u64 > limit {
        return Err("skin_size_limit".into());
    }
    Ok(b)
}
fn directory(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    let mut total = 0;
    let root = std::fs::symlink_metadata(path).map_err(|_| "skin_directory")?;
    if root.file_attributes() & 0x400 != 0 {
        return Err("skin_reparse".into());
    }
    for entry in std::fs::read_dir(path).map_err(|_| "skin_directory")? {
        let entry = entry.map_err(|_| "skin_entry")?;
        let m = std::fs::symlink_metadata(entry.path()).map_err(|_| "skin_metadata")?;
        if !m.is_file() || m.file_attributes() & 0x400 != 0 {
            return Err("skin_regular_files_only".into());
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or("skin_unicode_name")?
            .to_owned();
        if !safe_name(&name) || files.len() >= 32 {
            return Err("skin_entry_limit".into());
        }
        let b = read_bounded(
            std::fs::File::open(entry.path()).map_err(|_| "skin_read")?,
            MAX_FILE,
        )?;
        total += b.len() as u64;
        if total > MAX_TOTAL {
            return Err("skin_total_limit".into());
        }
        if files.insert(name, b).is_some() {
            return Err("skin_duplicate".into());
        }
    }
    Ok(files)
}
fn archive(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let file = std::fs::File::open(path).map_err(|_| "skin_open")?;
    if file.metadata().map_err(|_| "skin_metadata")?.len() > 16 * 1024 * 1024 {
        return Err("skin_archive_limit".into());
    }
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "skin_zip")?;
    if zip.len() > 32 {
        return Err("skin_entry_limit".into());
    }
    let mut files = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut total = 0;
    for i in 0..zip.len() {
        let item = zip.by_index(i).map_err(|_| "skin_zip_entry")?;
        let name = item.name().to_owned();
        if !safe_name(&name)
            || item.is_dir()
            || item
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
            || !names.insert(name.to_ascii_lowercase())
        {
            return Err("skin_unsafe_entry".into());
        }
        if item.size() > MAX_FILE {
            return Err("skin_file_limit".into());
        }
        let bytes = read_bounded(item, MAX_FILE)?;
        total += bytes.len() as u64;
        if total > MAX_TOTAL {
            return Err("skin_total_limit".into());
        }
        files.insert(name, bytes);
    }
    Ok(files)
}
fn validate(files: &BTreeMap<String, Vec<u8>>) -> Result<Manifest, String> {
    let raw = files.get("manifest.json").ok_or("skin_manifest_missing")?;
    if raw.len() > 32 * 1024 {
        return Err("skin_manifest_limit".into());
    }
    let m: Manifest = serde_json::from_slice(raw).map_err(|_| "skin_manifest_invalid")?;
    if m.schema_version != 1
        || !safe_id(&m.id)
        || m.name.is_empty()
        || m.name.len() > 120
        || m.author.len() > 120
        || m.license.is_empty()
        || m.license.len() > 120
        || m.version.len() > 30
        || m.version.split('.').count() != 3
        || !m.asset_scale.is_finite()
        || !(0.25..=4.).contains(&m.asset_scale)
    {
        return Err("skin_manifest_values".into());
    }
    if m.animations.len() != 4
        || !["idle", "locked", "target", "hit"]
            .iter()
            .all(|s| m.animations.contains_key(*s))
    {
        return Err("skin_animation_states".into());
    }
    let mut allowed = BTreeSet::from([
        "manifest.json".to_owned(),
        m.preview.clone(),
        m.trail.clone(),
    ]);
    for a in m.animations.values() {
        allowed.insert(a.file.clone());
    }
    if files.keys().any(|n| !allowed.contains(n)) || allowed.len() != files.len() {
        return Err("skin_unexpected_file".into());
    }
    let mut dimensions = BTreeMap::new();
    let mut decoded_total = 0u64;
    for (name, bytes) in files {
        if name == "manifest.json" {
            continue;
        }
        if !safe_name(name) || !name.ends_with(".png") || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("skin_png_only".into());
        }
        reject_animated_png(bytes)?;
        let reader =
            image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
        let (w, h) = reader.into_dimensions().map_err(|_| "skin_image_invalid")?;
        if w == 0 || h == 0 || w > 4096 || h > 4096 {
            return Err("skin_dimensions".into());
        }
        decoded_total += w as u64 * h as u64 * 4;
        if decoded_total > 64 * 1024 * 1024 {
            return Err("skin_decoded_limit".into());
        }
        // Decode before installation, never let a disguised non-image reach the WebView.
        image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|_| "skin_decode")?;
        dimensions.insert(name.clone(), (w, h));
    }
    for a in m.animations.values() {
        let (w, _) = dimensions.get(&a.file).ok_or("skin_animation_file")?;
        if a.frames == 0 || a.frames > 120 || a.fps == 0 || a.fps > 60 || w % a.frames != 0 {
            return Err("skin_animation_dimensions".into());
        }
    }
    Ok(m)
}
pub fn import(source: &Path, root: &Path) -> Result<Manifest, String> {
    let files = if source.is_dir() {
        directory(source)?
    } else {
        archive(source)?
    };
    let manifest = validate(&files)?;
    std::fs::create_dir_all(root).map_err(|_| "skin_store")?;
    let destination = root.join(&manifest.id);
    if destination.exists() {
        return Err("skin_already_installed".into());
    }
    let staging = root.join(format!(".import-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging).map_err(|_| "skin_staging")?;
    let result = (|| {
        for (n, b) in files {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(staging.join(n))
                .map_err(|_| "skin_write")?;
            f.write_all(&b).map_err(|_| "skin_write")?;
        }
        std::fs::rename(&staging, &destination).map_err(|_| "skin_commit")?;
        Ok(manifest)
    })();
    if result.is_err() {
        // Only the freshly-created UUID staging directory owned by this operation is removed.
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}
pub fn list(root: &Path) -> Vec<Manifest> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return vec![];
    };
    entries
        .flatten()
        .filter_map(|e| {
            let files = directory(&e.path()).ok()?;
            validate(&files).ok()
        })
        .collect()
}
pub fn load(root: &Path, id: &str) -> Result<Bundle, String> {
    if !safe_id(id) {
        return Err("skin_id".into());
    }
    let files = directory(&root.join(id))?;
    let manifest = validate(&files)?;
    if manifest.id != id {
        return Err("skin_identity".into());
    }
    let assets = files
        .into_iter()
        .filter(|(n, _)| n != "manifest.json")
        .map(|(n, b)| {
            (
                n,
                format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(b)
                ),
            )
        })
        .collect();
    Ok(Bundle { manifest, assets })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_paths() {
        for n in [
            "../a.png",
            "a\\b.png",
            "C:a.png",
            "/x.png",
            "con.png",
            "evil.png.",
            "a:b",
            "foo/bar",
        ] {
            assert!(!safe_name(n), "{n}")
        }
    }
    #[test]
    fn id_is_confined() {
        for id in ["..", "a/b", "builtin", "CON", ""] {
            assert!(!safe_id(id))
        }
        assert!(safe_id("pulsar-demo"))
    }
    #[test]
    fn bounded_reader() {
        assert!(read_bounded(&b"12345"[..], 4).is_err())
    }
    #[test]
    fn no_manifest() {
        assert!(validate(&BTreeMap::new()).is_err())
    }
}

fn reject_animated_png(bytes: &[u8]) -> Result<(), String> {
    let mut offset = 8usize;
    while offset.checked_add(12).is_some_and(|n| n <= bytes.len()) {
        let length = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| "skin_png_chunk")?,
        ) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        if kind == b"acTL" {
            return Err("skin_use_sprite_sheet".into());
        }
        offset = offset
            .checked_add(length)
            .and_then(|n| n.checked_add(12))
            .ok_or("skin_png_chunk")?;
        if offset > bytes.len() {
            return Err("skin_png_chunk".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn sample_is_valid() {
        let files = directory(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/sample-skin"),
        )
        .unwrap();
        assert_eq!(validate(&files).unwrap().id, "pulsar-original");
    }
    #[test]
    fn unexpected_executable_rejected() {
        let mut files = directory(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/sample-skin"),
        )
        .unwrap();
        files.insert("evil.exe".into(), b"MZ".to_vec());
        assert_eq!(validate(&files).err().unwrap(), "skin_unexpected_file")
    }
    #[test]
    fn reject_apng() {
        let bytes = b"\x89PNG\r\n\x1a\n\0\0\0\0acTL\0\0\0\0";
        assert!(reject_animated_png(bytes).is_err())
    }
}
#[cfg(test)]
mod archive_tests {
    use super::*;
    use std::io::Write;
    fn zip_with(names: &[&str]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"payload").unwrap();
        }
        zip.finish().unwrap();
        file
    }
    #[test]
    fn zip_traversal_rejected_before_extract() {
        let file = zip_with(&["../outside.png"]);
        assert!(archive(file.path()).is_err())
    }
    #[test]
    fn zip_case_collision_rejected() {
        let file = zip_with(&["idle.png", "IDLE.PNG"]);
        assert!(archive(file.path()).is_err())
    }
    #[test]
    fn import_is_transactional() {
        let root = tempfile::tempdir().unwrap();
        let file = zip_with(&["manifest.json", "script.js"]);
        assert!(import(file.path(), root.path()).is_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0)
    }
    #[test]
    fn valid_import_then_duplicate_rejected() {
        let root = tempfile::tempdir().unwrap();
        let source =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/sample-skin");
        assert!(import(&source, root.path()).is_ok());
        assert!(load(root.path(), "pulsar-original").is_ok());
        assert!(import(&source, root.path()).is_err())
    }
}
