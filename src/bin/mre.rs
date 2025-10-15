use mnemonic_core::api::routes::{AppState, create_router};
use mnemonic_core::graph::GraphEngine;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // Initialize our logging system
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize our GraphEngine (the heart of our application)
    let db_path = Path::new("./mre_data");
    let engine = Arc::new(GraphEngine::new(db_path).expect("Failed to create GraphEngine"));
    // Await the seed function to ensure it completes before the server starts listening.
engine.seed_if_empty().await.expect("Failed to seed the database");

    // Create our application state
    let app_state = AppState {
        engine: Arc::clone(&engine),
    };
    // Create the router from our api module.
    // Allow requests from any origin
    let cors = CorsLayer::new()
        .allow_origin(Any) // Allow any origin
        .allow_methods(Any) // Allow any HTTP method (GET, POST, etc.)
        .allow_headers(Any); // Allow any HTTP headers
    let app = create_router(app_state).layer(cors);

    // Define the network address to run our server on.
    // 0.0.0.0 is important for Docker/Codespaces. 8080 is the port.
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Server listening on {}", addr);

    // This is the magic line. It creates the server and tells it to
    // handle requests using our app router, forever.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
