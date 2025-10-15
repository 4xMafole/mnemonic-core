use axum::{extract::State, routing::{get, post}, Json, Router};
use tokio::task;
use std::sync::Arc;
use crate::{graph::GraphEngine, types::concept::{ConceptData, ConceptId}, MnemonicError};
use crate::types::relationship::{RelationshipId, RelationType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;



// This struct will hold all shared state for our application
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<GraphEngine>,
}

// This defines the shape of the JSON we expect for creating a concept.
// e.g {"data": {"name": "Alice"}}
#[derive(Deserialize)]
pub struct CreateConceptPayload {
    data: serde_json::Value,
}

// This defines the shape of the JSON we will send back.
// e.g {"concept_id": "..."}
#[derive(Serialize, Deserialize)]
pub struct CreateConceptResponse {
    concept_id: Uuid,
}

// These structs are simplified for the UI. It doesn't need all the metadata.
#[derive(Serialize, Deserialize)]

struct GraphNode {
    id: String,
    label: String,
}

#[derive(Serialize, Deserialize)]

struct GraphEdge {
    id: String,
    source: String,
    target: String,
    label: String,
}

#[derive(Serialize, Deserialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

// Request: { "source": "...", "type": "...", "target": "..." }
#[derive(Deserialize)]
struct RelatePayload {
    source: ConceptId,
    #[serde(rename = "type")]
    relationship_type: RelationType,
    target: ConceptId,
}

#[derive(Serialize)]
struct RelateResponse {
    relationship_id: RelationshipId,
}

// This is our main router function. It will define all the `buttons` on our API vending machine.
pub fn create_router(app_state: AppState) -> Router {
    Router::new()
    .route("/ping", get(ping))
    .route("/concepts", post(create_concept))
    .route("/graph", get(get_graph_data))
    .route("/relationships", post(relate_concepts))
    .with_state(app_state)
}

// This is an `handler function`. It's the logic that runs when someone requests `/ping`.
async fn ping() -> &'static str {
    "pong"
}

async fn create_concept(
    State(state): State<AppState>,
    Json(payload): Json<CreateConceptPayload>,
) -> Result<Json<CreateConceptResponse>, String> {
    print!("Received request to create concept with data: {:?}", payload.data);

    // This is where we finally call the engine we built!
    match state.engine.store(payload.data).await {
        Ok(concept_id) => Ok(Json(CreateConceptResponse { concept_id })),
        Err(e) => Err(format!("Failed to store concept: {}", e)),
    }
}

 async fn relate_concepts(
        State(state): State<AppState>,
        Json(payload): Json<RelatePayload>,
    ) -> Result<Json<RelateResponse>, String> {
        
        match state.engine.relate(payload.source, payload.relationship_type, payload.target).await {
            Ok(relationship_id) => Ok(Json(RelateResponse { relationship_id })),
            Err(e) => Err(format!("Failed to relate concepts: {}", e)),
        }
    }

    async fn get_graph_data(
    State(state): State<AppState>,
) -> Result<Json<GraphData>, String> {
    
    // We get the Transaction Manager...
    let tm = state.engine.transaction_manager();
    // ...and from it, the already-hydrated Version Store.
    let vs = tm.version_store();

    // Spawn a blocking task because RwLock is synchronous.
    let graph_data_result = task::spawn_blocking(move || {
        // Fetch nodes from the IN-MEMORY, hydrated Version Store.
        let nodes: Vec<GraphNode> = vs.get_all_active_concepts().unwrap_or_default()
            .iter()
            .map(|version| GraphNode {
                id: version.concept_id.to_string(),
                label: match &version.data {
                    ConceptData::Structured(s) => {
                        let json: serde_json::Value = serde_json::from_str(s).unwrap_or_default();
                        json.get("name").and_then(|v| v.as_str()).unwrap_or("Concept").to_string()
                    },
                    _ => "Concept".to_string(),
                }
            })
            .collect();
    
        // Fetch edges from the IN-MEMORY, hydrated Version Store.
        let edges: Vec<GraphEdge> = vs.get_all_active_relationships().unwrap_or_default()
            .iter()
            .map(|version| GraphEdge {
                id: version.relationship_id.to_string(),
                source: version.source.to_string(),
                target: version.target.to_string(),
                label: version.relationship_type.clone(),
            })
            .collect();
            
        Ok(GraphData { nodes, edges })
    }).await.map_err(|e| format!("Task error: {}", e))?;

    let graph_data: GraphData = graph_data_result.map_err(|e: MnemonicError| e.to_string())?;

    tracing::info!("Returning {} nodes and {} edges", graph_data.nodes.len(), graph_data.edges.len());
    Ok(Json(graph_data))
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from the parent module (routes.rs)
    use crate::graph::GraphEngine;
    use axum_test::TestServer; 
    use serde_json::json;
    use tempfile::tempdir;

    /// Helper function to quickly create a testable server.
    fn setup_test_server() -> TestServer {
        let dir = tempdir().unwrap();
        let engine = Arc::new(GraphEngine::new(dir.path()).unwrap());
        let app_state = AppState { engine };
        let app = create_router(app_state);
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_ping_route() {
        let server = setup_test_server();

        // Make a GET request to the /ping route.
        let response = server.get("/ping").await;
        
        response.assert_status_ok();
        response.assert_text("pong");
    }

    #[tokio::test]
    async fn test_create_concept_happy_path() {
        let server = setup_test_server();

        // Make a POST request to /concepts with a valid JSON body.
        let response = server
            .post("/concepts")
            .json(&json!({
                "data": {
                    "type": "test_concept",
                    "name": "API Test"
                }
            }))
            .await;

        response.assert_status_ok();
        
        // Check that the JSON response we get back has the field we expect.
        let json: CreateConceptResponse = response.json();
        assert!(!json.concept_id.is_nil()); // Ensure we got a valid UUID
    }

    #[tokio::test]
    async fn test_full_api_lifecycle_for_graph() {
        let server = setup_test_server();

        // 1. Create a "person" concept.
        let person_response: CreateConceptResponse = server
            .post("/concepts")
            .json(&json!({"data": {"name": "API Alice"}}))
            .await
            .json();
        let person_id = person_response.concept_id;

        // 2. Create a "project" concept.
        let project_response: CreateConceptResponse = server
            .post("/concepts")
            .json(&json!({"data": {"name": "API Project"}}))
            .await
            .json();
        let project_id = project_response.concept_id;
        
        // 3. Relate them using the /relationships endpoint.
        let rel_response = server
            .post("/relationships")
            .json(&json!({
                "source": person_id,
                "type": "works_on",
                "target": project_id
            }))
            .await;
        
        rel_response.assert_status_ok();

        // 4. Finally, get the full graph and verify everything is there.
        let graph_response: GraphData = server.get("/graph").await.json();

        // Assert that we have exactly two nodes and one edge.
        assert_eq!(graph_response.nodes.len(), 2);
        assert_eq!(graph_response.edges.len(), 1);

        // A more specific check on the edge.
        let edge = &graph_response.edges[0];
        assert_eq!(edge.source, person_id.to_string());
        assert_eq!(edge.target, project_id.to_string());
        assert_eq!(edge.label, "works_on");
    }
}