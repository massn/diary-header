//! A minimal terminal spinner shown on stderr while waiting for slow operations.

use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const INTERVAL: Duration = Duration::from_millis(80);

/// Runs `f` while showing a spinner with `message` on stderr.
///
/// The spinner is only drawn when stderr is a terminal, so redirected
/// output (e.g. `diary-header 2> log.txt`) stays clean. The spinner line is
/// cleared before returning, so callers can print freely afterwards.
pub fn with_spinner<T>(message: &str, f: impl FnOnce() -> T) -> T {
    if !std::io::stderr().is_terminal() {
        return f();
    }

    let running = Arc::new(AtomicBool::new(true));
    let handle = {
        let running = Arc::clone(&running);
        let message = message.to_string();
        thread::spawn(move || {
            let mut stderr = std::io::stderr();
            for frame in FRAMES.iter().cycle() {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                let _ = write!(stderr, "\r{frame} {message}");
                let _ = stderr.flush();
                thread::sleep(INTERVAL);
            }
            // Clear the spinner line.
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        })
    };

    let result = f();
    running.store(false, Ordering::Relaxed);
    let _ = handle.join();
    result
}
