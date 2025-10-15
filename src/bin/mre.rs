use mnemonic_core::graph::GraphEngine;
use std::path::Path;
use mnemonic_core::api::routes::{create_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize our logging system
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    // Initialize our GraphEngine (the heart of our application)
    let db_path = Path::new("./mre_data");
    let engine = Arc::new(GraphEngine::new(db_path).expect("Failed to create GraphEngine"));

    // Create our application state.
    let app_state = AppState {
        engine: Arc::clone(&engine),
    };
    // Create the router from our api module.
    let app = create_router(app_state);

    // Define the network address to run our server on.
    // 0.0.0.0 is important for Docker/Codespaces. 8080 is the port.
    let addr = SocketAddr::from(([0,0,0,0], 8080));
    tracing::info!("Server listening on {}", addr);

    // This is the magic line. It creates the server and tells it to
    // handle requests using our app router, forever.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}