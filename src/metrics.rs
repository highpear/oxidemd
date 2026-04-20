use std::path::Path;
use std::time::Duration;

pub struct DocumentTiming {
    pub total: Duration,
    pub parse: Duration,
}

pub fn log_startup(duration: Duration) {
    eprintln!("[perf] startup: {} ms", duration.as_millis());
}

pub fn log_initial_load(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] initial_load: {} ms total, {} ms parse ({})",
        timing.total.as_millis(),
        timing.parse.as_millis(),
        path.display()
    );
}

pub fn log_reload(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] reload: {} ms total, {} ms parse ({})",
        timing.total.as_millis(),
        timing.parse.as_millis(),
        path.display()
    );
}

pub fn log_reload_skipped(path: &Path, timing: &DocumentTiming) {
    eprintln!(
        "[perf] reload_skipped: {} ms total, unchanged content ({})",
        timing.total.as_millis(),
        path.display()
    );
}
