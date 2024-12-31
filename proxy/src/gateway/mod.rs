use crate::load_balancer::RoundRobinLoadBalancer;
use async_trait::async_trait;
use crds::IngressRoute;
use dashmap::DashMap;
use futures_util::future::BoxFuture;
use kube::Resource;
use log::info;
use pingora::http::StatusCode;
use pingora::prelude::{HttpPeer, Session};
use pingora::proxy::ProxyHttp;
use std::sync::Arc;

#[derive(Clone)]
pub struct SharedGateway(Arc<Gateway>);

impl SharedGateway {
    pub fn new(gateway: Gateway) -> Self {
        Self(Arc::new(gateway))
    }

    pub fn update_route_table(
        &self,
    ) -> impl Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
           + Send
           + Sync
           + 'static {
        self.0.update_route_table()
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
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            route_table: Arc::new(DashMap::new()),
        }
    }

    pub fn update_route_table(
        &self,
    ) -> impl Fn(kube::client::Client, IngressRoute) -> BoxFuture<'static, Result<(), anyhow::Error>>
           + Send
           + Sync
           + 'static {
        let route_table = self.route_table.clone();
        move |k8s_client, route| {
            Box::pin({
                let route_meta = route.meta();
                let host = route.spec.route.host.clone();

                let route_table = route_table.clone();
                async move {
                    let lb = RoundRobinLoadBalancer::try_from_iter(
                        "crumble.svc.cluster.local",
                        ["127.0.0.1:8083"],
                    )?;

                    route_table
                        .entry(host.clone())
                        .and_modify(|existing_lb| {
                            info!(
                                "Updating IngressRoute '{}' in the {} namespace",
                                route_meta.clone().name.unwrap(),
                                route_meta.clone().namespace.unwrap()
                            );
                            *existing_lb = lb.clone();
                        })
                        .or_insert_with(|| {
                            info!(
                                "New IngressRoute '{}' added in the {} namespace",
                                route_meta.clone().name.unwrap(),
                                route_meta.clone().namespace.unwrap()
                            );
                            lb
                        });

                    Ok(())
                }
            })
        }
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
