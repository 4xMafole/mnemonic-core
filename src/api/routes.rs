use axum::{routing::get, Router};

// This is our main router function. It will define all the `buttons` on our API vending machine.
pub fn create_router() -> Router {
    Router::new().route("/ping", get(ping))
}

// This is an `handler function`. It's the logic that runs when someone requests `/ping`.
async fn ping() -> &'static str {
    "pong"
}