use std::path::Path;
use std::time::Duration;

pub struct DocumentTiming {
    pub total: Duration,
    pub parse: Duration,
    pub byte_len: usize,
}

pub fn log_startup(duration: Duration) {
    eprintln!("[perf] startup: {} ms", duration.as_millis());
}

pub fn log_initial_load(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] initial_load: {} ms total, {} ms parse, {} ({})",
        timing.total.as_millis(),
        timing.parse.as_millis(),
        format_byte_len(timing.byte_len),
        path.display()
    );
}

pub fn log_reload(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] reload: {} ms total, {} ms parse, {} ({})",
        timing.total.as_millis(),
        timing.parse.as_millis(),
        format_byte_len(timing.byte_len),
        path.display()
    );
}

pub fn log_reload_skipped(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] reload_skipped: {} ms total, unchanged content, {} ({})",
        timing.total.as_millis(),
        format_byte_len(timing.byte_len),
        path.display()
    );
}

fn format_byte_len(byte_len: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;

    let bytes = byte_len as f64;
    if bytes >= MIB {
        format!("{:.2} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{} B", byte_len)
    }
}
