use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;

#[derive(Debug, Clone, CustomResource, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "ferrix.com",
    version = "v1",
    kind = "IngressRoute",
    doc = "IngressRoute is the CRD implementation of a Ferrix HTTP Router",
    namespaced
)]
pub struct IngressRouteSpec {
    pub entrypoint: String,
    pub route: IngressRouteRoute,
    pub tls: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IngressRouteRoute {
    pub host: String,
    pub rules: Vec<IngressRouteRule>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct IngressRouteRule {
    pub matches: String,
    pub service: IngressRouteService,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct IngressRouteService {
    pub name: String,
    pub port: u16,
}
