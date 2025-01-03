mod handlers;
mod router;
mod schemas;

use crate::gateway::RouteTable;
use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use log::{debug, error, info};
use pingora::server::ShutdownWatch;
use pingora::services::background::BackgroundService;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct Service {
    port: u16,
    route_tables: Arc<DashMap<String, RouteTable>>,
}

impl Service {
    pub fn new(port: u16, route_tables: Arc<DashMap<String, RouteTable>>) -> Self {
        Self { port, route_tables }
    }

    pub async fn run(&self, mut shutdown: ShutdownWatch) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .map_err(|e| anyhow!("error creating listener: {}", e))?;
        info!("API server listening on {}", listener.local_addr().unwrap());

        let app = router::new(self.route_tables.clone());
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                if let Err(e) = shutdown.changed().await {
                    debug!("Error while waiting for API shutdown signal: {}", e);
                }
                info!("Shutting down API server");
            })
            .await
            .map_err(|e| anyhow!("error running API server: {}", e))
    }
}

#[async_trait]
impl BackgroundService for Service {
    async fn start(&self, shutdown: ShutdownWatch) {
        if let Err(e) = self.run(shutdown).await {
            error!("Error running API web service: {}", e);
        }
    }
}
