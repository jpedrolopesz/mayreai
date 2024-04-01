use crate::controllers::home_controller::index;
use axum::{routing::get, Router};

pub fn init() -> Router {
    Router::new().route("/", get(index))
}
