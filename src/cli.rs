use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bagery")]
#[command(about = "A custom web bundler", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Increase verbosity level (-v for Info, -vv for Debug, -vvv for Trace)
    #[arg(short = 'v',  action = clap::ArgAction::Count)]
    pub log_level: u8,

    /// Disable colored output
    #[arg(long = "no-color")]
    pub no_color: bool,
}

impl Args {
    pub fn log_level(&self) -> crate::log::Level {
        use crate::log::Level;
        match self.log_level {
            0 => Level::Warn,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Start development server
    Dev(DevCommand),
    /// Build for production
    Build(BuildCommand),
}

#[derive(Parser)]
pub struct DevCommand {}

#[derive(Parser)]
pub struct BuildCommand {}

pub fn parse_args() -> Args {
    Args::parse()
}
