use crate::server::Config;
use std::fs::File;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to read config file: {0}")]
    IO(std::io::Error),

    #[error("Unable to deserialize config file: {0}")]
    Parse(serde_yml::Error),
}

pub fn load(filename: &str) -> Result<Config, Error> {
    let file = File::open(filename).map_err(Error::IO)?;
    serde_yml::from_reader(file).map_err(Error::Parse)
}
