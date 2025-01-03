use crate::api::schemas;
use crate::gateway::RouteTable;
use axum::extract::State;
use axum::Json;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn routes(
    State(route_tables): State<Arc<DashMap<String, RouteTable>>>,
) -> Json<HashMap<String, Vec<schemas::Route>>> {
    let mut routes = HashMap::with_capacity(route_tables.len());
    for table in route_tables.iter() {
        let route_table = table
            .iter()
            .map(|v| {
                let lb = v.value();
                schemas::Route {
                    host: v.key().clone(),
                    sni: lb.get_sni(),
                    backends: lb.clone().get_ip_addresses(),
                }
            })
            .collect();
        routes.insert(table.key().clone(), route_table);
    }
    Json(routes)
}
