//! HTTP API for agent monitoring

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{AgentMonitor, DashboardSummary, AgentMetricsSnapshot, AgentEvent, Alert};

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub monitor: Arc<AgentMonitor>,
}

/// Query parameters for metrics
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub limit: Option<usize>,
}

/// Create the monitoring API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/dashboard", get(get_dashboard))
        .route("/agents/:agent_id/metrics", get(get_agent_metrics))
        .route("/agents/:agent_id/history", get(get_agent_history))
        .route("/alerts", get(get_alerts))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> StatusCode {
    StatusCode::OK
}

/// Get dashboard summary
async fn get_dashboard(
    State(state): State<ApiState>,
) -> Result<Json<DashboardSummary>, StatusCode> {
    let summary = state.monitor.get_dashboard_summary();
    Ok(Json(summary))
}

/// Response for agent metrics
#[derive(Debug, Serialize)]
pub struct AgentMetricsResponse {
    pub agent_id: String,
    pub metrics: Option<AgentMetricsSnapshot>,
}

/// Get current metrics for a specific agent
async fn get_agent_metrics(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentMetricsResponse>, StatusCode> {
    let metrics = state.monitor.get_agent_metrics(&agent_id);
    
    Ok(Json(AgentMetricsResponse {
        agent_id,
        metrics,
    }))
}

/// Response for agent history
#[derive(Debug, Serialize)]
pub struct AgentHistoryResponse {
    pub agent_id: String,
    pub history: Vec<AgentMetricsSnapshot>,
}

/// Get metrics history for a specific agent
async fn get_agent_history(
    State(state): State<ApiState>,
    Path(agent_id): Path<String>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<AgentHistoryResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(100);
    let history = state
        .monitor
        .get_agent_history(&agent_id, limit)
        .unwrap_or_default();
    
    Ok(Json(AgentHistoryResponse {
        agent_id,
        history,
    }))
}

/// Response for alerts
#[derive(Debug, Serialize)]
pub struct AlertsResponse {
    pub alerts: Vec<Alert>,
    pub total_active: usize,
}

/// Get all active alerts
async fn get_alerts(
    State(state): State<ApiState>,
) -> Result<Json<AlertsResponse>, StatusCode> {
    let alerts = state.monitor.get_active_alerts();
    let total_active = alerts.len();
    
    Ok(Json(AlertsResponse {
        alerts,
        total_active,
    }))
}
