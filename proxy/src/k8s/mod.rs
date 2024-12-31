use kube::Resource;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

pub mod watcher;

pub trait Object: Resource + Clone + DeserializeOwned + Debug + Send + 'static {}

impl<T> Object for T where T: Resource + Clone + DeserializeOwned + Debug + Send + 'static {}
