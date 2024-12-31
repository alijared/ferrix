use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub name: String,
    pub port: u16,
    #[serde(default)]
    pub secure: bool,
}
