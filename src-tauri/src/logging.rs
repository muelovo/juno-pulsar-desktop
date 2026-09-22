use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};
struct Logger(Mutex<File>);
impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) && r.target().starts_with("juno_pulsar_desktop") {
            if let Ok(mut f) = self.0.lock() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = writeln!(f, "{} {} {}", now, r.level(), r.args());
            }
        }
    }
    fn flush(&self) {
        if let Ok(mut f) = self.0.lock() {
            let _ = f.flush();
        }
    }
}
pub fn init(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("diagnostics.log");
    if path.metadata().is_ok_and(|m| m.len() > 1024 * 1024) {
        std::fs::write(&path, [])?;
    }
    let logger = Box::leak(Box::new(Logger(Mutex::new(
        OpenOptions::new().append(true).create(true).open(path)?,
    ))));
    let _ = log::set_logger(logger);
    log::set_max_level(log::LevelFilter::Info);
    Ok(())
}
