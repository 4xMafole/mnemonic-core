use mnemonic_core::graph::GraphEngine;
use std::path::Path;

#[tokio::main]
async fn main() {
    // Initialize our logging system
    tracing_subscriber::fmt::init();

    // Let's create our engine instance, using a hardcoded path for now.
    let db_path = Path::new("./mre_data");
    let engine = GraphEngine::new(db_path).expect("Failed to create GraphEngine");

    tracing::info!("Mnemonic Runtime Environment is starting...");

    // This is a placeholder for our web server later.
    tracing::info!("Engine initialized successfully. Server will start here.");
}