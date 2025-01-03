use crate::k8s;
use crate::load_balancer::RoundRobinLoadBalancer;
use anyhow::anyhow;
use async_trait::async_trait;
use crds::IngressRoute;
use dashmap::DashMap;
use futures_util::future::BoxFuture;
use k8s_openapi::api::core::v1::Endpoints;
use kube::runtime::watcher;
use kube::runtime::watcher::Event;
use kube::{Api, Resource};
use log::{debug, error, info};
use pingora::http::StatusCode;
use pingora::prelude::{HttpPeer, Session};
use pingora::proxy::ProxyHttp;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct SharedGateway(Arc<Gateway>);

impl SharedGateway {
    pub fn new(gateway: Gateway) -> Self {
        Self(Arc::new(gateway))
    }

    pub fn get_route_table(&self) -> Arc<DashMap<String, RoundRobinLoadBalancer>> {
        self.0.get_route_table()
    }
}

#[async_trait]
impl ProxyHttp for SharedGateway {
    type CTX = <Gateway as ProxyHttp>::CTX;

    fn new_ctx(&self) -> Self::CTX {
        self.0.as_ref().new_ctx()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        self.0.as_ref().upstream_peer(session, ctx).await
    }
}

pub struct Gateway {
    route_table: Arc<DashMap<String, RoundRobinLoadBalancer>>,
    managed_objects: Arc<DashMap<String, (String, Arc<Notify>)>>,
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            route_table: Arc::new(DashMap::new()),
            managed_objects: Arc::new(DashMap::new()),
        }
    }

    pub fn get_route_table(&self) -> Arc<DashMap<String, RoundRobinLoadBalancer>> {
        self.route_table.clone()
    }

    pub fn update_route_tables(
        route_tables: Arc<DashMap<String, SharedGateway>>,
    ) -> impl Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
           + Send
           + Sync
           + 'static {
        move |k8s_client, route| {
            let route_tables = route_tables.clone();
            Box::pin(async move {
                if let Some(gateway) = route_tables.get(&route.spec.entrypoint).map(|v| v.value().clone()) {
                    return gateway.0.update_route_table(k8s_client, route).await;
                }

                Ok(())
            })
        }
    }

    async fn update_route_table(
        &self,
        k8s_client: kube::Client,
        route: IngressRoute,
    ) -> Result<(), anyhow::Error> {
        let route_meta = route.meta().clone();
        let route_id = route_meta.uid.clone().unwrap();
        let host = route.spec.route.host.clone();
        let route_table = self.route_table.clone();
        let managed_objects = self.managed_objects.clone();
        let route = route.clone();

        if route_meta.deletion_timestamp.is_some() {
            Self::delete_route(&host, route_table.clone(), managed_objects.clone());
            return Ok(());
        }

        let notify = Arc::new(Notify::new());
        let (sni, ips) =
            Self::get_endpoints_from_route(k8s_client, route, notify.clone(), route_table.clone())
                .await
                .map_err(|e| anyhow!("unable to get endpoints for new service: {}", e))?;
        let lb = RoundRobinLoadBalancer::try_from_iter(&sni, ips)?;

        if let Some((object_host, notify)) = managed_objects.get(&host).map(|v| v.clone()) {
            if object_host == host {
                route_table.alter(&object_host, |_, _| lb);
                return Ok(());
            }

            Self::delete_route(&object_host, route_table.clone(), managed_objects.clone());
            route_table.insert(host.clone(), lb);
            managed_objects.alter(&route_id, |_, _| (host, notify));
        } else {
            route_table.insert(host.clone(), lb);
            managed_objects.insert(route_id, (host, notify));
        }

        Ok(())
    }

    fn delete_route(
        host: &str,
        route_table: Arc<DashMap<String, RoundRobinLoadBalancer>>,
        managed_objects: Arc<DashMap<String, (String, Arc<Notify>)>>,
    ) {
        let notify = managed_objects.get(host).map(|v| v.clone().1).unwrap();
        notify.notify_one();

        route_table.remove(host);
        managed_objects.remove(host);
    }

    async fn get_endpoints_from_route(
        client: kube::Client,
        route: IngressRoute,
        notify: Arc<Notify>,
        route_table: Arc<DashMap<String, RoundRobinLoadBalancer>>,
    ) -> Result<(String, Vec<String>), kube::Error> {
        let backup_namespace = route.meta().namespace.clone().unwrap();
        let service = route.spec.route.rules[0].service.clone();
        let namespace = service.namespace.clone().unwrap_or(backup_namespace);
        let api = Api::<Endpoints>::namespaced(client.clone(), &namespace);
        let ep = api.get(&service.name).await?;

        let sni = format!("{}.{}.svc.cluster.local", service.name, namespace.clone());
        let sni_clone = sni.clone();
        let host = route.spec.route.host.clone();
        tokio::spawn(async move {
            let watch_opts = watcher::Config {
                field_selector: Some(format!("metadata.name={}", service.name)),
                ..Default::default()
            };
            let mut watch =
                match k8s::watcher::create::<Endpoints>(client.clone(), watch_opts).await {
                    Ok(w) => w,
                    Err(e) => {
                        error!("Unable to create endpoints watcher: {}", e);
                        return;
                    }
                };

            loop {
                tokio::select! {
                    _ = notify.notified() => {
                        break;
                    }
                    event = watch.recv() => match event {
                        Some(event) => {
                            let endpoints = match event {
                                Event::Applied(e) => vec![e],
                                Event::Deleted(_) => continue,
                                Event::Restarted(e) => e
                            };

                            let ep = endpoints.last().unwrap();
                            let ips = k8s::endpoints::get_ip_addresses(ep.clone(), service.port);
                            match RoundRobinLoadBalancer::try_from_iter(&sni_clone, ips) {
                                Ok(lb) => {
                                    debug!("Load balancer updated with new endpoint addresses");
                                    route_table.alter(&host, |_, _| lb);
                                }
                                Err(e) => error!("Unable to update load balancer with new endpoints: {}", e),
                            }
                        }
                        None => continue
                    }
                }
            }
        });

        let ips = k8s::endpoints::get_ip_addresses(ep, service.port);
        Ok((sni, ips))
    }
}

#[async_trait]
impl ProxyHttp for Gateway {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let host = session
            .req_header()
            .headers
            .get("Host")
            .ok_or(pingora::Error::create(
                pingora::ErrorType::InvalidHTTPHeader,
                pingora::ErrorSource::Upstream,
                Some("No HTTP Host header present in request".into()),
                None,
            ))?
            .to_str()
            .map_err(|e| {
                pingora::Error::because(
                    pingora::ErrorType::InvalidHTTPHeader,
                    "Invalid Host header",
                    e,
                )
            })?;

        if let Some(lb) = self.route_table.get(host) {
            return lb.value().upstream_peer(session, ctx).await;
        }

        Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(
            StatusCode::NOT_FOUND.as_u16(),
        )))
    }
}
