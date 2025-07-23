use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

mod merkle_tree;
use merkle_tree::{IncrementalMerkleTree, MerkleProof};

#[derive(Clone)]
struct AppState {
    tree: Arc<RwLock<IncrementalMerkleTree>>,
}

#[derive(Deserialize)]
struct AddLeafRequest {
    leaf: String,
}

#[derive(Deserialize)]
struct AddLeavesRequest {
    leaves: Vec<String>,
}

#[derive(Deserialize)]
struct GetProofRequest {
    index: usize,
}

#[derive(Serialize)]
struct NumLeavesResponse {
    num_leaves: usize,
}

#[derive(Serialize)]
struct RootResponse {
    root: String,
}

#[derive(Serialize)]
struct ProofResponse {
    proof: MerkleProof,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn add_leaf(
    State(state): State<AppState>,
    Json(payload): Json<AddLeafRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let leaf_bytes = hex::decode(&payload.leaf).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid hex string".to_string(),
            }),
        )
    })?;

    let mut tree = state.tree.write().await;
    tree.add_leaf(leaf_bytes).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(StatusCode::OK)
}

async fn add_leaves(
    State(state): State<AppState>,
    Json(payload): Json<AddLeavesRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let leaves_bytes: Result<Vec<_>, _> = payload.leaves
        .iter()
        .map(|leaf_hex| {
            hex::decode(leaf_hex).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Invalid hex string in leaves".to_string(),
                    }),
                )
            })
        })
        .collect();

    let leaves_bytes = leaves_bytes?;

    let mut tree = state.tree.write().await;
    tree.add_leaves(leaves_bytes).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(StatusCode::OK)
}

async fn get_num_leaves(
    State(state): State<AppState>,
) -> Json<NumLeavesResponse> {
    let tree = state.tree.read().await;
    Json(NumLeavesResponse {
        num_leaves: tree.num_leaves(),
    })
}

async fn get_root(
    State(state): State<AppState>,
) -> Result<Json<RootResponse>, (StatusCode, Json<ErrorResponse>)> {
    let maybe_root = {
        let mut tree = state.tree.write().await;
        tree.root().map(hex::encode)
    };

    match maybe_root {
        Some(root) => Ok(Json(RootResponse { root })),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tree is empty".to_string(),
            }),
        )),
    }
}

async fn get_proof(
    State(state): State<AppState>,
    Json(payload): Json<GetProofRequest>,
) -> Result<Json<ProofResponse>, (StatusCode, Json<ErrorResponse>)> {
    let maybe_proof = {
        let mut tree = state.tree.write().await;
        tree.get_proof(payload.index)
    };

    match maybe_proof {
        Some(proof) => Ok(Json(ProofResponse { proof })),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid index or tree is empty".to_string(),
            }),
        )),
    }
}

#[tokio::main]
async fn main() {
    let state = AppState {
        tree: Arc::new(RwLock::new(IncrementalMerkleTree::new())),
    };

    let app = Router::new()
        .route("/add-leaf", post(add_leaf))
        .route("/add-leaves", post(add_leaves))
        .route("/get-num-leaves", get(get_num_leaves))
        .route("/get-root", get(get_root))
        .route("/get-proof", post(get_proof))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Server running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
