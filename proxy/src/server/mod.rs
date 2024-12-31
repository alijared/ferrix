pub mod config;
mod entry_point;

use pingora::server;
use pingora::server::Server;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub server: server::configuration::ServerConf,
    pub entry_points: Vec<entry_point::Config>,
}

pub fn new(config: server::configuration::ServerConf) -> Server {
    let opts = pingora::prelude::Opt::default();
    Server::new_with_opt_and_conf(opts, config)
}
