use crate::api::handlers;
use crate::load_balancer::RoundRobinLoadBalancer;
use axum::routing::get;
use axum::Router;
use dashmap::DashMap;
use std::sync::Arc;

pub fn new(
    route_tables: Arc<DashMap<String, Arc<DashMap<String, RoundRobinLoadBalancer>>>>,
) -> Router {
    Router::new()
        .route("/routes", get(handlers::routes))
        .with_state(route_tables)
}
