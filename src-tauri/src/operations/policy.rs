use serde::{Deserialize, Serialize};
use std::{
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use windows::{
    core::*,
    Win32::{Foundation::*, Storage::FileSystem::*, System::Com::CoTaskMemFree, UI::Shell::*},
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub volume: u32,
    pub high: u32,
    pub low: u32,
    pub created: u64,
}
pub fn known(id: &GUID) -> std::result::Result<PathBuf, String> {
    // SAFETY: known folder API returns COM task-allocated UTF16, freed exactly once.
    unsafe {
        let p = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)
            .map_err(|_| "known_folder_unavailable")?;
        let s = p.to_string();
        CoTaskMemFree(Some(p.0.cast()));
        s.map(PathBuf::from).map_err(|_| "invalid_unicode".into())
    }
}
pub fn inspect(path: &Path) -> std::result::Result<Identity, String> {
    let text = path.to_str().ok_or("invalid_unicode")?;
    if text.starts_with("\\\\") || !path.is_absolute() || text.get(2..).unwrap_or(":").contains(':')
    {
        return Err("nonlocal_path".into());
    }
    let parent = path.parent().ok_or("root_protected")?;
    let desktop = known(&FOLDERID_Desktop)?;
    let public = known(&FOLDERID_PublicDesktop)?;
    if !equal(parent, &desktop) && !equal(parent, &public) {
        return Err("not_desktop_child".into());
    }
    for id in [
        FOLDERID_Windows,
        FOLDERID_System,
        FOLDERID_ProgramFiles,
        FOLDERID_ProgramFilesX86,
    ] {
        let root = known(&id)?;
        if under(path, &root) {
            return Err("system_location".into());
        }
    }
    for id in [FOLDERID_Profile, FOLDERID_Desktop, FOLDERID_PublicDesktop] {
        if equal(path, &known(&id)?) {
            return Err("root_protected".into());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if equal(path, &exe) || under(&exe, path) {
            return Err("running_application".into());
        }
    }
    for ancestor in path.ancestors() {
        let m = std::fs::symlink_metadata(ancestor).map_err(|_| "metadata_denied")?;
        if m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err("reparse_point".into());
        }
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "target_missing")?;
    if metadata.file_attributes() & (FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_READONLY.0) != 0 {
        return Err("protected_attributes".into());
    }
    // Request DELETE access without elevation. Share flags permit the later Shell recycle operation.
    let file = std::fs::OpenOptions::new()
        .access_mode(DELETE.0 | FILE_READ_ATTRIBUTES.0)
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
        .map_err(|_| "delete_access_denied")?;
    use std::os::windows::io::AsRawHandle;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: borrowed handle remains open, info is properly sized writable storage.
    unsafe {
        GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info)
            .map_err(|_| "identity_unavailable")?;
    }
    Ok(Identity {
        volume: info.dwVolumeSerialNumber,
        high: info.nFileIndexHigh,
        low: info.nFileIndexLow,
        created: ((info.ftCreationTime.dwHighDateTime as u64) << 32)
            | info.ftCreationTime.dwLowDateTime as u64,
    })
}
fn equal(a: &Path, b: &Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}
fn under(a: &Path, b: &Path) -> bool {
    let aa = a.to_string_lossy().to_lowercase();
    let bb = b.to_string_lossy().to_lowercase();
    aa == bb || aa.starts_with(&(bb.trim_end_matches('\\').to_owned() + "\\"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_boundary() {
        assert!(under(Path::new(r"C:\Windows\a"), Path::new(r"c:\windows")));
        assert!(!under(
            Path::new(r"C:\Windows-old\a"),
            Path::new(r"c:\windows")
        ));
    }
    #[test]
    fn reject_network() {
        assert_eq!(
            inspect(Path::new(r"\\server\a")).unwrap_err(),
            "nonlocal_path"
        )
    }
    #[test]
    fn reject_drive() {
        assert!(inspect(Path::new(r"C:\")).is_err())
    }
}
