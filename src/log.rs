#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            eprintln!("[frisk] {}", format!($($arg)*));
        }
    }};
}
