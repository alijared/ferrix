use k8s_openapi::api::core::v1::Endpoints;

pub fn get_ip_addresses(endpoints: Endpoints, port: u16) -> Vec<String> {
    endpoints
        .subsets
        .unwrap_or_default()
        .into_iter()
        .flat_map(|subset| {
            subset
                .addresses
                .unwrap_or_default()
                .into_iter()
                .map(|addr| format!("{}:{}", addr.ip, port))
        })
        .collect()
}
