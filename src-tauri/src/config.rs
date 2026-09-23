use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub speed: f64,
    pub count: u32,
    pub size: f64,
    pub fps: u32,
    pub trail: f64,
    pub sound: bool,
    pub monitor: String,
    pub deletion: bool,
    pub autostart: bool,
    pub fullscreen_pause: bool,
    pub paused: bool,
    pub skin: String,
    pub skins: Vec<String>,
    #[serde(default)]
    pub render_revision: u32,
    pub motion: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            speed: 1.,
            count: 5,
            size: 1.,
            fps: 60,
            trail: 0.7,
            sound: true,
            monitor: "primary".into(),
            deletion: false,
            autostart: false,
            fullscreen_pause: true,
            paused: false,
            skin: "builtin".into(),
            skins: Vec::new(),
            render_revision: 1,
            motion: "cruise".into(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !self.speed.is_finite()
            || !(0.2..=3.).contains(&self.speed)
            || !(1..=10).contains(&self.count)
            || !self.size.is_finite()
            || !(0.5..=2.).contains(&self.size)
            || ![30, 60].contains(&self.fps)
            || !self.trail.is_finite()
            || !(0.0..=1.).contains(&self.trail)
            || self.skin.len() > 80
            || self.skins.len() > 10
            || self.skins.iter().any(|skin| skin.len() > 80)
            || self.monitor.len() > 200
            || self.render_revision > 1
            || !["cruise", "figure8", "swoop", "hover"].contains(&self.motion.as_str())
        {
            return Err("invalid_configuration".into());
        }
        Ok(())
    }
}
pub fn load(dir: &Path) -> Config {
    let mut config = std::fs::read(dir.join("config.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Config>(&b).ok())
        .filter(|c| c.validate().is_ok())
        .unwrap_or_default();
    if config.render_revision == 0 {
        config.fps = 60;
        config.render_revision = 1;
        let _ = save(dir, &config);
    }
    config
}
pub fn save(dir: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    std::fs::create_dir_all(dir).map_err(|_| "config_directory")?;
    let tmp = dir.join("config.next");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(config).map_err(|_| "config_serialize")?,
    )
    .map_err(|_| "config_write")?;
    // SAFETY: null-terminated owned strings remain alive; rename is confined to application data.
    unsafe {
        windows::Win32::Storage::FileSystem::MoveFileExW(
            &windows::core::HSTRING::from(tmp.as_os_str()),
            &windows::core::HSTRING::from(dir.join("config.json").as_os_str()),
            windows::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                | windows::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|_| "config_replace")?;
    }
    Ok(())
}
pub fn autostart(enable: bool) -> Result<(), String> {
    use winreg::{enums::*, RegKey};
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_SET_VALUE,
        )
        .map_err(|_| "autostart_registry")?;
    if enable {
        let exe = std::env::current_exe().map_err(|_| "autostart_executable")?;
        key.set_value(
            "JunoPulsarDesktop",
            &format!("\"{}\" --background", exe.display()),
        )
        .map_err(|_| "autostart_write")?;
    } else {
        match key.delete_value("JunoPulsarDesktop") {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("autostart_remove".into()),
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_defaults() {
        let c = Config::default();
        assert!(!c.deletion);
        assert!(c.sound);
        assert_eq!(c.count, 5);
        assert!(c.validate().is_ok())
    }
    #[test]
    fn reject_bad_fps() {
        let c = Config {
            fps: 144,
            ..Config::default()
        };
        assert!(c.validate().is_err())
    }
    #[test]
    fn accepts_ten_skin_slots_and_rejects_more() {
        let mut c = Config {
            skins: (0..10).map(|index| format!("skin-{index}")).collect(),
            ..Config::default()
        };
        assert!(c.validate().is_ok());
        c.skins.push("one-too-many".into());
        assert!(c.validate().is_err())
    }
    #[test]
    fn old_config_migrates_to_sixty_fps_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut old = Config {
            fps: 30,
            render_revision: 0,
            ..Config::default()
        };
        std::fs::write(
            dir.path().join("config.json"),
            serde_json::to_vec(&old).unwrap(),
        )
        .unwrap();
        old = load(dir.path());
        assert_eq!(old.fps, 60);
        assert_eq!(old.render_revision, 1);
        old.fps = 30;
        save(dir.path(), &old).unwrap();
        assert_eq!(load(dir.path()).fps, 30);
    }
}
