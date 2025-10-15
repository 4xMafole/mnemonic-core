use axum::{extract::State, routing::{get, post}, Json, Router};
use std::sync::Arc;
use crate::{graph::GraphEngine, types::concept::{ConceptData, ConceptId}};
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
#[derive(Serialize)]
pub struct CreateConceptResponse {
    concept_id: Uuid,
}

// These structs are simplified for the UI. It doesn't need all the metadata.
#[derive(Serialize)]
struct GraphNode {
    id: String,
    label: String,
}

#[derive(Serialize)]
struct GraphEdge {
    id: String,
    source: String,
    target: String,
    label: String,
}

#[derive(Serialize)]
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

    // This is a temporary solution! We're accessing the internals directly.
    // Later, a real RETRIEVE query will handle this.
    let tm = state.engine.transaction_manager();
    let vs = tm.version_store();

    let nodes: Vec<GraphNode> = vs.get_all_active_concepts().unwrap_or_default()
    .iter()
    .map(|version| {
        GraphNode{
            id: version.concept_id.to_string(),
            label: match &version.data{
                // try to find a "name" property to use as a label.
                ConceptData::Structured(s) => {
                    let json: serde_json::Value = serde_json::from_str(s).unwrap_or_default();

                    json.get("name").and_then(|v| v.as_str()).unwrap_or("Concept").to_string()
                },
                _ => "Concept".to_string(),
            }
        }
    }).collect();

     tracing::info!("Returning {} nodes", nodes.len());
     
    // For now, edges will be empty. We'll implement this next.
    let edges = Vec::<GraphEdge>::new();

    Ok(Json(GraphData {nodes, edges}))
}