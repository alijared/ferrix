use crate::api::handlers;
use crate::gateway::RouteTable;
use axum::routing::get;
use axum::Router;
use dashmap::DashMap;
use std::sync::Arc;

pub fn new(route_tables: Arc<DashMap<String, RouteTable>>) -> Router {
    Router::new()
        .route("/routes", get(handlers::routes))
        .with_state(route_tables)
}
