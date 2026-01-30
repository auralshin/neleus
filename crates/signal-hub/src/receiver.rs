//! Signal receivers - HTTP, WebSocket, gRPC endpoints

use crate::{Result, Signal, SignalHub, SignalHubError};
use async_trait::async_trait;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

/// Signal receiver trait
#[async_trait]
pub trait SignalReceiver: Send + Sync {
    /// Start receiving signals
    async fn start(&self, addr: SocketAddr) -> Result<()>;
    
    /// Stop receiving signals
    async fn stop(&self) -> Result<()>;
    
    /// Check if receiver is running
    fn is_running(&self) -> bool;
}

/// HTTP Signal Receiver
pub struct HttpSignalReceiver {
    hub: SignalHub,
}

impl HttpSignalReceiver {
    pub fn new(hub: SignalHub) -> Self {
        Self { hub }
    }
    
    pub async fn start(&self, addr: SocketAddr) -> Result<()> {
        let hub = Arc::new(self.hub.clone());
        
        let app = Router::new()
            .route("/health", get(health_check))
            .route("/api/v1/signals", post(receive_signal))
            .route("/api/v1/signals/batch", post(receive_signal_batch))
            .route("/api/v1/signals/query", post(query_signals))
            .with_state(hub);
        
        tracing::info!("Starting HTTP signal receiver on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| SignalHubError::ReceiverError(e.to_string()))?;
        
        axum::serve(listener, app)
            .await
            .map_err(|e| SignalHubError::ReceiverError(e.to_string()))?;
        
        Ok(())
    }
}

#[async_trait]
impl SignalReceiver for HttpSignalReceiver {
    async fn start(&self, addr: SocketAddr) -> Result<()> {
        HttpSignalReceiver::start(self, addr).await
    }
    
    async fn stop(&self) -> Result<()> {
        // Graceful shutdown would be implemented here
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        true // Would track actual state
    }
}

// HTTP Handlers

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "signal-hub"
    }))
}

#[derive(Debug, Deserialize)]
struct SignalRequest {
    #[serde(flatten)]
    signal: Signal,
    /// API key for authentication
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct SignalResponse {
    signal_id: String,
    status: String,
    message: Option<String>,
}

async fn receive_signal(
    State(hub): State<Arc<SignalHub>>,
    Json(request): Json<SignalRequest>,
) -> impl IntoResponse {
    let mut signal = request.signal;
    
    // Add API key to metadata if provided
    if let Some(key) = request.api_key {
        signal.metadata.insert("api_key".to_string(), key);
    }
    
    match hub.process_signal(signal).await {
        Ok(signal_id) => (
            StatusCode::ACCEPTED,
            Json(SignalResponse {
                signal_id,
                status: "accepted".to_string(),
                message: None,
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(SignalResponse {
                signal_id: String::new(),
                status: "rejected".to_string(),
                message: Some(e.to_string()),
            }),
        ),
    }
}

#[derive(Debug, Deserialize)]
struct BatchSignalRequest {
    signals: Vec<Signal>,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchSignalResponse {
    accepted: Vec<String>,
    rejected: Vec<RejectedSignal>,
}

#[derive(Debug, Serialize)]
struct RejectedSignal {
    signal_id: String,
    reason: String,
}

async fn receive_signal_batch(
    State(hub): State<Arc<SignalHub>>,
    Json(request): Json<BatchSignalRequest>,
) -> impl IntoResponse {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    
    for mut signal in request.signals {
        if let Some(ref key) = request.api_key {
            signal.metadata.insert("api_key".to_string(), key.clone());
        }
        
        match hub.process_signal(signal.clone()).await {
            Ok(signal_id) => accepted.push(signal_id),
            Err(e) => rejected.push(RejectedSignal {
                signal_id: signal.id,
                reason: e.to_string(),
            }),
        }
    }
    
    Json(BatchSignalResponse { accepted, rejected })
}

async fn query_signals(
    State(hub): State<Arc<SignalHub>>,
    Json(query): Json<crate::SignalQuery>,
) -> impl IntoResponse {
    match hub.query_signals(query).await {
        Ok(signals) => (StatusCode::OK, Json(signals)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<Signal>::new())),
    }
}

/// WebSocket Signal Receiver (placeholder)
pub struct WebSocketSignalReceiver {
    hub: SignalHub,
}

impl WebSocketSignalReceiver {
    pub fn new(hub: SignalHub) -> Self {
        Self { hub }
    }
}

#[async_trait]
impl SignalReceiver for WebSocketSignalReceiver {
    async fn start(&self, _addr: SocketAddr) -> Result<()> {
        // TODO: Implement WebSocket receiver
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        false
    }
}
