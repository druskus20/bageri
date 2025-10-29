use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(Level::Info as usize);
static COLORS_ENABLED: AtomicBool = AtomicBool::new(true);

struct Colors;

impl Colors {
    const RESET: &'static str = "\x1b[0m";
    const RED: &'static str = "\x1b[31m";
    const YELLOW: &'static str = "\x1b[33m";
    const BLUE: &'static str = "\x1b[34m";
    const CYAN: &'static str = "\x1b[36m";
    const GRAY: &'static str = "\x1b[90m";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if colors_enabled() {
            match self {
                Level::Error => write!(f, "{}ERROR{}", Colors::RED, Colors::RESET),
                Level::Warn => write!(f, "{}WARN{}", Colors::YELLOW, Colors::RESET),
                Level::Info => write!(f, "{}INFO{}", Colors::BLUE, Colors::RESET),
                Level::Debug => write!(f, "{}DEBUG{}", Colors::CYAN, Colors::RESET),
                Level::Trace => write!(f, "{}TRACE{}", Colors::GRAY, Colors::RESET),
            }
        } else {
            match self {
                Level::Error => write!(f, "ERROR"),
                Level::Warn => write!(f, "WARN"),
                Level::Info => write!(f, "INFO"),
                Level::Debug => write!(f, "DEBUG"),
                Level::Trace => write!(f, "TRACE"),
            }
        }
    }
}

pub fn set_max_level(level: Level) {
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}

pub fn set_colors_enabled(enabled: bool) {
    COLORS_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn colors_enabled() -> bool {
    COLORS_ENABLED.load(Ordering::Relaxed)
}

pub fn max_level() -> Level {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Info,
    }
}

fn should_log(level: Level) -> bool {
    level <= max_level()
}

pub fn log(level: Level, args: fmt::Arguments) {
    if should_log(level) {
        eprintln!("[{level}] {args}");
    }
}

#[macro_export]
macro_rules! __error {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Error, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! __warn {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Warn, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! __info {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Info, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! __debug {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Debug, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! __trace {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Trace, format_args!($($arg)*))
    };
}

pub mod prelude {
    pub use super::{Level, colors_enabled, max_level, set_colors_enabled, set_max_level};
    pub use crate::__debug as debug;
    pub use crate::__error as error;
    pub use crate::__info as info;
    pub use crate::__trace as trace;
    pub use crate::__warn as warn;
}
