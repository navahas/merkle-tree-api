use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

mod lmdb_tree;
mod merkle_tree;
mod storage;
use lmdb_tree::LmdbMerkleTree;
use merkle_tree::{IncrementalMerkleTree, MerkleProof};

#[derive(Clone)]
struct AppState {
    tree: Arc<RwLock<IncrementalMerkleTree>>,
}

#[derive(Clone)]
struct LmdbAppState {
    tree: Arc<RwLock<LmdbMerkleTree>>,
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
    let leaves_bytes: Result<Vec<_>, _> = payload
        .leaves
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

async fn get_num_leaves(State(state): State<AppState>) -> Json<NumLeavesResponse> {
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
        let tree = state.tree.read().await;
        tree.get_proof(payload.index)
    };

    match maybe_proof {
        Some(proof) => Ok(Json(ProofResponse { proof })),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid index, tree cache invalid, or tree is empty".to_string(),
            }),
        )),
    }
}

// LMDB handlers
async fn lmdb_add_leaf(
    State(state): State<LmdbAppState>,
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

    let tree = state.tree.read().await;
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

async fn lmdb_add_leaves(
    State(state): State<LmdbAppState>,
    Json(payload): Json<AddLeavesRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let leaves_bytes: Result<Vec<_>, _> = payload
        .leaves
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

    let tree = state.tree.read().await;
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

async fn lmdb_get_num_leaves(State(state): State<LmdbAppState>) -> Json<NumLeavesResponse> {
    let tree = state.tree.read().await;
    Json(NumLeavesResponse {
        num_leaves: tree.num_leaves(),
    })
}

async fn lmdb_get_root(
    State(state): State<LmdbAppState>,
) -> Result<Json<RootResponse>, (StatusCode, Json<ErrorResponse>)> {
    let maybe_root = {
        let tree = state.tree.read().await;
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

async fn lmdb_get_proof(
    State(state): State<LmdbAppState>,
    Json(payload): Json<GetProofRequest>,
) -> Result<Json<ProofResponse>, (StatusCode, Json<ErrorResponse>)> {
    let maybe_proof = {
        let tree = state.tree.read().await;
        tree.get_proof(payload.index)
    };

    match maybe_proof {
        Some(proof) => Ok(Json(ProofResponse { proof })),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid index, tree cache invalid, or tree is empty".to_string(),
            }),
        )),
    }
}

#[tokio::main]
async fn main() {
    let heap_tree = Arc::new(RwLock::new(IncrementalMerkleTree::new()));

    let storage_path =
        std::env::var("STORAGE_PATH").unwrap_or_else(|_| "./merkle_tree.db".to_string());
    let lmdb_tree = match LmdbMerkleTree::new(&storage_path) {
        Ok(tree) => {
            println!("Loaded existing LMDB merkle tree from: {}", storage_path);
            Arc::new(RwLock::new(tree))
        }
        Err(e) => {
            println!(
                "Failed to load from storage ({}), creating new LMDB tree: {}",
                storage_path, e
            );
            Arc::new(RwLock::new(LmdbMerkleTree::new(&storage_path).unwrap()))
        }
    };

    let heap_state = AppState { tree: heap_tree };
    let lmdb_state = LmdbAppState { tree: lmdb_tree };

    let heap_routes = Router::new()
        .route("/add-leaf", post(add_leaf))
        .route("/add-leaves", post(add_leaves))
        .route("/get-num-leaves", get(get_num_leaves))
        .route("/get-root", get(get_root))
        .route("/get-proof", post(get_proof))
        .with_state(heap_state);

    let lmdb_routes = Router::new()
        .route("/lmdb/add-leaf", post(lmdb_add_leaf))
        .route("/lmdb/add-leaves", post(lmdb_add_leaves))
        .route("/lmdb/get-num-leaves", get(lmdb_get_num_leaves))
        .route("/lmdb/get-root", get(lmdb_get_root))
        .route("/lmdb/get-proof", post(lmdb_get_proof))
        .with_state(lmdb_state);

    let app = Router::new()
        .merge(heap_routes)
        .merge(lmdb_routes)
        .layer(CorsLayer::permissive());

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Server running on http://{}", addr);
    println!("Heap routes: /add-leaf, /add-leaves, /get-num-leaves, /get-root, /get-proof");
    println!(
        "LMDB routes: /lmdb/add-leaf, /lmdb/add-leaves, /lmdb/get-num-leaves, /lmdb/get-root, /lmdb/get-proof"
    );
    println!("Storage path: {}", storage_path);

    axum::serve(listener, app).await.unwrap();
}
