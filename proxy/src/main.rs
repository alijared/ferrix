use crate::gateway::{Gateway, SharedGateway};
use anyhow::anyhow;
use clap::Parser;
use dashmap::DashMap;
use log::{error, info};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use pingora::prelude::background_service;
use pingora::proxy::http_proxy_service;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod api;
mod gateway;
mod k8s;
mod load_balancer;
mod server;

#[derive(Parser, Debug)]
#[command(version, about = "I'm a turnip", long_about = None)]
struct CliArgs {
    #[arg(
        short,
        long,
        help = "Config file location",
        default_value = "/etc/ferrix/config.yaml"
    )]
    config_file: String,

    #[arg(long, help = "Application log level", default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,

    #[arg(long, help = "Enable HTTP API interface")]
    api_enabled: bool,

    #[arg(long, help = "Port to run the HTTP API", default_value_t = 8080)]
    api_port: u16,
}

fn main() {
    let cli_args = CliArgs::parse();
    env_logger::builder()
        .filter_level(cli_args.log_level)
        .init();

    if let Err(e) = run(cli_args) {
        error!("{}", e);
    }
}

fn run(args: CliArgs) -> Result<(), anyhow::Error> {
    let config = server::config::load(&args.config_file)?;
    let mut server = server::new(config.server);
    let (watch_failure_tx, mut watch_failure_rx) = tokio::sync::mpsc::channel(1);

    server.bootstrap();

    let entry_points = DashMap::with_capacity(config.entry_points.len());
    let route_tables = DashMap::with_capacity(config.entry_points.len());
    for ep in config.entry_points {
        let gateway = SharedGateway::new(Gateway::new());
        route_tables.insert(ep.name.clone(), gateway.get_route_table());
        let mut proxy = http_proxy_service(&server.configuration, gateway.clone());

        proxy.add_tcp(format!("[::]:{}", ep.port).as_str());
        server.add_service(proxy);
        entry_points.insert(ep.name.clone(), gateway.clone());
    }

    server.add_services(vec![Box::new(background_service(
        "Kubernetes IngressRoute watcher",
        k8s::watcher::Service::new(
            Gateway::update_route_tables(Arc::new(entry_points)),
            watch_failure_tx,
        ),
    ))]);

    if args.api_enabled {
        info!("Starting up HTTP API");
        server.add_service(background_service(
            "API",
            api::Service::new(args.api_port, Arc::new(route_tables)),
        ))
    }

    let rt = Runtime::new().map_err(|e| anyhow!("Failed to create watch failure runtime {}", e))?;
    rt.spawn(async move {
        if let Some(failure) = watch_failure_rx.recv().await {
            error!("Watcher error: {}", failure);
            signal::kill(Pid::this(), Signal::SIGINT).unwrap();
        }
    });
    server.run_forever();
}
