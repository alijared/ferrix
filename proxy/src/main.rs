use crate::gateway::{Gateway, SharedGateway};
use anyhow::anyhow;
use clap::Parser;
use log::error;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use pingora::prelude::background_service;
use pingora::proxy::http_proxy_service;
use tokio::runtime::Runtime;

mod gateway;
mod k8s;
mod load_balancer;
mod server;

#[derive(Parser, Debug)]
#[command(version, about = "I'm a turnip", long_about = None)]
struct CliArgs {
    #[arg(short, long, default_value = "/etc/ferrix/config.yaml")]
    config_file: String,

    #[arg(long, default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
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

    let gateway = SharedGateway::new(Gateway::new());
    let mut proxy = http_proxy_service(&server.configuration, gateway.clone());
    proxy.add_tcp(format!("[::]:{}", config.entry_points[0].port).as_str());

    server.bootstrap();
    server.add_services(vec![
        Box::new(background_service(
            "Kubernetes IngressRoute watcher",
            k8s::watcher::Service::new(gateway.update_route_table(), watch_failure_tx),
        )),
        Box::new(proxy),
    ]);

    let rt = Runtime::new().map_err(|e| anyhow!("Failed to create watch failure runtime {}", e))?;
    rt.spawn(async move {
        if let Some(failure) = watch_failure_rx.recv().await {
            error!("Watcher error: {}", failure);
            signal::kill(Pid::this(), Signal::SIGINT).unwrap();
        }
    });
    server.run_forever();
}
