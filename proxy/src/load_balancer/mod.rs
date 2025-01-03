use async_trait::async_trait;
use pingora::lb::LoadBalancer;
use pingora::prelude::{HttpPeer, RoundRobin, Session};
use pingora::proxy::ProxyHttp;
use std::net::ToSocketAddrs;
use std::sync::Arc;

#[derive(Clone)]
pub struct RoundRobinLoadBalancer {
    sni: String,
    load_balancer: Arc<LoadBalancer<RoundRobin>>,
}

impl RoundRobinLoadBalancer {
    pub fn try_from_iter<A, T: IntoIterator<Item = A>>(
        sni: &str,
        addresses: T,
    ) -> std::io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let upstreams = LoadBalancer::try_from_iter(addresses)?;
        Ok(Self {
            sni: sni.to_string(),
            load_balancer: Arc::new(upstreams),
        })
    }

    pub fn get_sni(&self) -> String {
        self.sni.clone()
    }

    pub fn get_ip_addresses(self) -> Vec<String> {
        let backends = self.load_balancer.backends();
        backends
            .get_backend()
            .iter()
            .map(|b| b.addr.to_string())
            .collect()
    }
}

#[async_trait]
impl ProxyHttp for RoundRobinLoadBalancer {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let upstream = self.load_balancer.select(b"", 256).unwrap();
        let peer = Box::new(HttpPeer::new(upstream, false, self.sni.clone()));
        Ok(peer)
    }
}
