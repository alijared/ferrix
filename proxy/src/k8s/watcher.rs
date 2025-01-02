use crate::k8s::Object;
use anyhow::anyhow;
use async_trait::async_trait;
use crds::IngressRoute;
use futures_util::future::BoxFuture;
use futures_util::TryStreamExt;
use kube::runtime::watcher::Event;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Resource};
use log::{debug, error, info};
use pingora::server::ShutdownWatch;
use pingora::services::background::BackgroundService;
use std::pin::pin;
use tokio::select;
use tokio::sync::mpsc;

pub struct Service<F>
where
    F: Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
        + Send
        + Sync
        + 'static,
{
    update: F,
    failure: mpsc::Sender<anyhow::Error>,
}

impl<F> Service<F>
where
    F: Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
        + Send
        + Sync
        + 'static,
{
    pub fn new(update: F, failure_bus: mpsc::Sender<anyhow::Error>) -> Self {
        Self {
            update,
            failure: failure_bus,
        }
    }
}

#[async_trait]
impl<F> BackgroundService for Service<F>
where
    F: Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
        + Send
        + Sync
        + 'static,
{
    async fn start(&self, mut shutdown: ShutdownWatch) {
        info!("Starting Kubernetes watch service");

        let client = match kube::client::Client::try_default().await {
            Ok(c) => c,
            Err(e) => {
                if let Err(e) = self
                    .failure
                    .clone()
                    .send(anyhow!("unable to create Kubernetes client: {}", e))
                    .await
                {
                    error!("Error sending error result failure channel: {}", e);
                }
                return;
            }
        };

        debug!("Kubernetes client acquisition successful");
        debug!("Creating Kubernetes watcher is running");
        let mut watch =
            match create::<IngressRoute>(client.clone(), watcher::Config::default()).await {
                Ok(w) => w,
                Err(e) => {
                    if let Err(e) = self
                        .failure
                        .clone()
                        .send(anyhow!("unable to create Kubernetes watcher: {}", e))
                        .await
                    {
                        error!("Error sending error result failure channel: {}", e);
                    }
                    return;
                }
            };

        loop {
            select! {
                _ = shutdown.changed() => {
                    info!("Stopping Kubernetes watch service");
                    break;
                }
                event = watch.recv() => match event {
                    Some(event) => {
                        debug!("Received a watch event");

                        let routes= match event {
                            Event::Deleted(route) => {
                                vec![route]
                            }
                            Event::Applied(route) => {
                                vec![route]
                            }
                            Event::Restarted(routes) => {
                                routes
                            }
                        };

                        for route in routes {
                            if let Err(e) = (self.update)(client.clone(), route).await {
                                error!("Error running watch service update: {}", e);
                            }
                        }
                    },
                    None => continue,
                }
            }
        }
    }
}

pub async fn create<T: Object>(
    client: kube::client::Client,
    config: watcher::Config,
) -> Result<mpsc::Receiver<Event<T>>, anyhow::Error>
where
    <T as Resource>::DynamicType: Default,
{
    let api = Api::all(client);
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(async move {
        let stream = watcher(api, config).default_backoff();
        let mut stream = pin!(stream);
        loop {
            select! {
                event = stream.try_next() => match event {
                    Ok(e) => {
                        if let Some(event) = e {
                            tx.send(event).await.unwrap();
                        }
                    },
                    Err(e) => {
                        error!("Unable to read from stream: {}", e);
                    }
                }
            }
        }
    });
    Ok(rx)
}
