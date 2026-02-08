use std::sync::Arc;
use dashmap::DashMap;
use crate::model::SymbolState;

pub type SharedState = Arc<DashMap<String, SymbolState>>;

pub fn init_store() -> SharedState {
    Arc::new(DashMap::new())
}
