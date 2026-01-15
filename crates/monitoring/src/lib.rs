use axum::{
    extract::{Query, State},
    http::{header, HeaderValue},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use neleus_core_bus::BusStats;
use neleus_core_engine::{EngineSnapshot, TelemetrySink};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

const DASHBOARD_HTML: &str = include_str!("../assets/dashboard.html");

#[derive(Debug, Clone, Serialize)]
pub struct BusSnapshot {
    pub stats: BusStats,
    pub pending: usize,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TelemetrySnapshot {
    pub engine: EngineSnapshot,
    pub bus: BusSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp: u64,
    pub line: String,
}

struct LogBuffer {
    capacity: usize,
    next_id: u64,
    lines: VecDeque<LogEntry>,
}

impl LogBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            next_id: 1,
            lines: VecDeque::with_capacity(capacity),
        }
    }

    fn push_line(&mut self, line: String) {
        let entry = LogEntry {
            id: self.next_id,
            timestamp: now_nanos(),
            line,
        };
        self.next_id += 1;
        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(entry);
    }

    fn since(&self, since: Option<u64>) -> Vec<LogEntry> {
        match since {
            None => self.lines.iter().cloned().collect(),
            Some(cursor) => self
                .lines
                .iter()
                .filter(|entry| entry.id > cursor)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Clone)]
struct LogWriterFactory {
    buffer: Arc<Mutex<LogBuffer>>,
}

impl LogWriterFactory {
    fn new(buffer: Arc<Mutex<LogBuffer>>) -> Self {
        Self { buffer }
    }
}

struct LogWriter {
    buffer: Arc<Mutex<LogBuffer>>,
    partial: String,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let chunk = String::from_utf8_lossy(buf);
        self.partial.push_str(&chunk);

        while let Some(pos) = self.partial.find('\n') {
            let line = self.partial[..pos].to_string();
            self.partial = self.partial[pos + 1..].to_string();
            self.buffer.lock().push_line(line);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogWriterFactory {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            buffer: self.buffer.clone(),
            partial: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub also_stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            also_stdout: true,
        }
    }
}

#[derive(Clone)]
pub struct TelemetryHub {
    engine: Arc<RwLock<EngineSnapshot>>,
    bus: Arc<RwLock<BusSnapshot>>,
    logs: Arc<Mutex<LogBuffer>>,
}

impl TelemetryHub {
    pub fn new(log_capacity: usize) -> Self {
        let engine = EngineSnapshot::default();
        let bus = BusSnapshot {
            stats: BusStats::default(),
            pending: 0,
            updated_at: now_nanos(),
        };

        Self {
            engine: Arc::new(RwLock::new(engine)),
            bus: Arc::new(RwLock::new(bus)),
            logs: Arc::new(Mutex::new(LogBuffer::new(log_capacity))),
        }
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            engine: self.engine.read().clone(),
            bus: self.bus.read().clone(),
        }
    }

    pub fn logs_since(&self, since: Option<u64>) -> Vec<LogEntry> {
        self.logs.lock().since(since)
    }

    pub fn install_tracing(
        &self,
        config: LoggingConfig,
    ) -> Result<(), tracing_subscriber::util::TryInitError> {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(config.level));

        let buffer_layer = tracing_subscriber::fmt::layer()
            .with_writer(LogWriterFactory::new(self.logs.clone()))
            .with_ansi(false)
            .with_target(true)
            .with_level(true);

        let registry = tracing_subscriber::registry()
            .with(filter)
            .with(buffer_layer);

        if config.also_stdout {
            let stdout_layer = tracing_subscriber::fmt::layer()
                .with_writer(io::stdout)
                .with_target(true)
                .with_level(true);
            registry.with(stdout_layer).try_init()
        } else {
            registry.try_init()
        }
    }
}

impl TelemetrySink for TelemetryHub {
    fn on_engine_snapshot(&self, snapshot: EngineSnapshot) {
        *self.engine.write() = snapshot;
    }

    fn on_bus_stats(&self, stats: BusStats, pending: usize) {
        *self.bus.write() = BusSnapshot {
            stats,
            pending,
            updated_at: now_nanos(),
        };
    }
}

pub struct DashboardServer {
    hub: Arc<TelemetryHub>,
}

impl DashboardServer {
    pub fn new(hub: Arc<TelemetryHub>) -> Self {
        Self { hub }
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), std::io::Error> {
        let app = Router::new()
            .route("/", get(dashboard))
            .route("/api/snapshot", get(snapshot))
            .route("/api/logs", get(logs))
            .route("/metrics", get(metrics))
            .with_state(self.hub);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app.into_make_service()).await
    }
}

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn snapshot(State(hub): State<Arc<TelemetryHub>>) -> Json<TelemetrySnapshot> {
    Json(hub.snapshot())
}

#[derive(Debug, Deserialize)]
struct LogQuery {
    since: Option<u64>,
}

async fn logs(
    State(hub): State<Arc<TelemetryHub>>,
    Query(query): Query<LogQuery>,
) -> Json<Vec<LogEntry>> {
    Json(hub.logs_since(query.since))
}

async fn metrics(State(hub): State<Arc<TelemetryHub>>) -> impl IntoResponse {
    let snapshot = hub.snapshot();
    let body = render_prometheus(&snapshot);

    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    response
}

fn render_prometheus(snapshot: &TelemetrySnapshot) -> String {
    let engine = &snapshot.engine;
    let bus = &snapshot.bus;

    format!(
        "# HELP neleus_engine_processed_messages Total messages processed by engine\n\
# TYPE neleus_engine_processed_messages counter\n\
neleus_engine_processed_messages {}\n\
# HELP neleus_engine_processed_ticks Total ticks processed by engine\n\
# TYPE neleus_engine_processed_ticks counter\n\
neleus_engine_processed_ticks {}\n\
# HELP neleus_engine_last_tick_duration_micros Last tick duration in microseconds\n\
# TYPE neleus_engine_last_tick_duration_micros gauge\n\
neleus_engine_last_tick_duration_micros {}\n\
# HELP neleus_bus_messages_published Messages published to bus\n\
# TYPE neleus_bus_messages_published counter\n\
neleus_bus_messages_published {}\n\
# HELP neleus_bus_messages_delivered Messages delivered by bus\n\
# TYPE neleus_bus_messages_delivered counter\n\
neleus_bus_messages_delivered {}\n\
# HELP neleus_bus_messages_dropped Messages dropped by bus\n\
# TYPE neleus_bus_messages_dropped counter\n\
neleus_bus_messages_dropped {}\n\
# HELP neleus_bus_pending Pending messages in bus\n\
# TYPE neleus_bus_pending gauge\n\
neleus_bus_pending {}\n",
        engine.processed_messages,
        engine.processed_ticks,
        engine.last_tick_duration_micros,
        bus.stats.messages_published,
        bus.stats.messages_delivered,
        bus.stats.messages_dropped,
        bus.pending,
    )
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
