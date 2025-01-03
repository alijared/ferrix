use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Route {
    pub host: String,
    pub sni: String,
    pub backends: Vec<String>,
}
