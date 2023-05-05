use std::fs::File;

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};

pub fn log_init(file_name: &str) {
    #[cfg(debug_assertions)]
    let log_level = LevelFilter::Debug;

    #[cfg(not(debug_assertions))]
    let log_level = LevelFilter::Info;

    CombinedLogger::init(vec![
        TermLogger::new(
            log_level,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            File::create(file_name).unwrap(),
        ),
    ])
    .unwrap();
}
