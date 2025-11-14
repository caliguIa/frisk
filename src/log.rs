/// Simple logging macro for debugging
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            eprintln!("[kickoff] {}", format!($($arg)*));
        }
    }};
}
