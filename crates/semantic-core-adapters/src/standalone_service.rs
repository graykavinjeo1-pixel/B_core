use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::approved_response::{
    ApprovedCompositionalResponseIR, ApprovedEventDiscourseStateIR, ApprovedEventPragmaticContextIR,
};
use crate::concurrent_runtime::{
    ConcurrentConversationRuntime, ConcurrentRuntimeError, ConcurrentRuntimeStatsIR,
};
use crate::construction_runtime::{ConstructionRuntime, PersonaConstructionContextIR};
use crate::conversation::ConversationTurnRequestIR;
use crate::discourse_focus::DiscourseFocusStateIR;
use crate::document_response::{interpret_document_semantics, realize_document_response};
use crate::language_knowledge::LanguageCodeIR;

pub const STANDALONE_SERVICE_STATS_SCHEMA: &str = "B_CORE_STANDALONE_SERVICE_STATS_IR_2";
pub const DEFAULT_HTTP_WORKERS: usize = 32;
pub const DEFAULT_HTTP_QUEUE_CAPACITY: usize = 256;
pub const DEFAULT_MAX_HTTP_BODY_BYTES: usize = 1_048_576;
pub const DEFAULT_IO_TIMEOUT_MILLIS: u64 = 10_000;
pub const MAX_HTTP_WORKERS: usize = 128;
pub const MAX_HTTP_QUEUE_CAPACITY: usize = 4_096;
const MAX_HTTP_HEADER_BYTES: usize = 16_384;
// The service records latency locally per worker, so this observability path
// never introduces a shared hot-counter bottleneck.  Percentiles are reported
// as conservative one-millisecond upper bounds; the exact maximum is exposed
// separately for rare overflow cases.
const LATENCY_HISTOGRAM_BUCKET_WIDTH_MICROS: u64 = 1_000;
const LATENCY_HISTOGRAM_LAST_REGULAR_BUCKET: usize = 1_000;
const LATENCY_HISTOGRAM_BUCKET_COUNT: usize = LATENCY_HISTOGRAM_LAST_REGULAR_BUCKET + 1;

#[derive(Debug, Deserialize)]
struct ConversationCloseRequest {
    conversation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandaloneHttpServiceConfig {
    pub bind_address: String,
    pub worker_count: usize,
    pub queue_capacity: usize,
    pub max_body_bytes: usize,
    pub io_timeout_millis: u64,
    /// Optional, restart-persistent learned construction artifact. The store
    /// is loaded once at startup; requests never choose an arbitrary path.
    pub construction_store_path: Option<String>,
    /// Optional contrastive persona artifact paired with the construction
    /// store. It is loaded once at startup and cannot be selected per request.
    pub persona_contrastive_store_path: Option<String>,
    /// Optional residual latent artifact used only by research shadow runs.
    pub persona_latent_operator_store_path: Option<String>,
    /// Optional learned state-to-language controller paired with the latent
    /// artifact.  It contains compact state deltas, never surface templates.
    pub language_state_controller_store_path: Option<String>,
}

impl Default for StandaloneHttpServiceConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8787".into(),
            worker_count: DEFAULT_HTTP_WORKERS,
            queue_capacity: DEFAULT_HTTP_QUEUE_CAPACITY,
            max_body_bytes: DEFAULT_MAX_HTTP_BODY_BYTES,
            io_timeout_millis: DEFAULT_IO_TIMEOUT_MILLIS,
            construction_store_path: None,
            persona_contrastive_store_path: None,
            persona_latent_operator_store_path: None,
            language_state_controller_store_path: None,
        }
    }
}

impl StandaloneHttpServiceConfig {
    pub fn validate(&self) -> Result<(), StandaloneServiceError> {
        if self.bind_address.trim().is_empty()
            || self.worker_count == 0
            || self.worker_count > MAX_HTTP_WORKERS
            || self.queue_capacity < self.worker_count
            || self.queue_capacity > MAX_HTTP_QUEUE_CAPACITY
            || self.max_body_bytes == 0
            || self.max_body_bytes > 16 * 1_048_576
            || self.io_timeout_millis == 0
            || self.io_timeout_millis > 120_000
        {
            return Err(StandaloneServiceError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedDocumentRequestIR {
    pub response: ApprovedCompositionalResponseIR,
    #[serde(default)]
    pub event_discourse_states: Vec<ApprovedEventDiscourseStateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_focus_state: Option<DiscourseFocusStateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_completed_turns: Option<u64>,
    #[serde(default)]
    pub event_pragmatic_contexts: Vec<ApprovedEventPragmaticContextIR>,
    pub output_language: LanguageCodeIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedDocumentInterpretRequestIR {
    pub response: ApprovedCompositionalResponseIR,
    #[serde(default)]
    pub event_discourse_states: Vec<ApprovedEventDiscourseStateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_focus_state: Option<DiscourseFocusStateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_completed_turns: Option<u64>,
    #[serde(default)]
    pub event_pragmatic_contexts: Vec<ApprovedEventPragmaticContextIR>,
    pub output_language: LanguageCodeIR,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaConstructionDocumentRequestIR {
    pub document: ApprovedDocumentRequestIR,
    pub context: PersonaConstructionContextIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandaloneServiceStatsIR {
    pub schema: String,
    pub worker_count: usize,
    pub queue_capacity: usize,
    pub current_pending_connections: usize,
    pub peak_pending_connections: usize,
    pub accepted_connections: usize,
    pub completed_requests: usize,
    pub rejected_connections: usize,
    pub failed_requests: usize,
    pub current_active_workers: usize,
    pub peak_active_workers: usize,
    /// Time from bounded-queue admission until a worker begins handling the
    /// connection.  This separates tolerable queueing from slow execution.
    pub average_queue_wait_micros: u64,
    /// Conservative one-millisecond upper bound of the p95 queue wait.  This
    /// distinguishes saturation delay from work performed by a handler.
    pub p95_queue_wait_micros_upper_bound: u64,
    pub max_queue_wait_micros: u64,
    /// Time spent in the HTTP handler after a worker begins the request.
    pub average_handler_micros: u64,
    /// Conservative one-millisecond upper bound of the p95 handler duration.
    pub p95_handler_micros_upper_bound: u64,
    pub max_handler_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandaloneServiceRunReportIR {
    pub service: StandaloneServiceStatsIR,
    pub runtime: ConcurrentRuntimeStatsIR,
}

#[derive(Debug)]
pub enum StandaloneServiceError {
    InvalidConfiguration,
    Bind(io::Error),
    LocalAddress(io::Error),
    Runtime(ConcurrentRuntimeError),
    WorkerSpawn(io::Error),
    WorkerPanicked,
    QueueDisconnected,
    ConstructionArtifactInvalid,
}

struct ServiceCounters {
    accepted_connections: AtomicUsize,
    completed_requests: AtomicUsize,
    rejected_connections: AtomicUsize,
    failed_requests: AtomicUsize,
    current_active_workers: AtomicUsize,
    peak_active_workers: AtomicUsize,
    total_queue_wait_micros: AtomicU64,
    max_queue_wait_micros: AtomicU64,
    total_handler_micros: AtomicU64,
    max_handler_micros: AtomicU64,
    worker_latency_histograms: Vec<WorkerLatencyHistograms>,
}

struct WorkerLatencyHistograms {
    queue_wait: LatencyHistogram,
    handler: LatencyHistogram,
}

struct LatencyHistogram {
    buckets: Box<[AtomicUsize]>,
}

impl LatencyHistogram {
    fn new() -> Self {
        Self {
            buckets: (0..LATENCY_HISTOGRAM_BUCKET_COUNT)
                .map(|_| AtomicUsize::new(0))
                .collect(),
        }
    }

    fn observe(&self, micros: u64) {
        let bucket = usize::try_from(micros / LATENCY_HISTOGRAM_BUCKET_WIDTH_MICROS)
            .unwrap_or(LATENCY_HISTOGRAM_LAST_REGULAR_BUCKET)
            .min(LATENCY_HISTOGRAM_LAST_REGULAR_BUCKET);
        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }
}

impl ServiceCounters {
    fn new(worker_count: usize) -> Self {
        Self {
            accepted_connections: AtomicUsize::new(0),
            completed_requests: AtomicUsize::new(0),
            rejected_connections: AtomicUsize::new(0),
            failed_requests: AtomicUsize::new(0),
            current_active_workers: AtomicUsize::new(0),
            peak_active_workers: AtomicUsize::new(0),
            total_queue_wait_micros: AtomicU64::new(0),
            max_queue_wait_micros: AtomicU64::new(0),
            total_handler_micros: AtomicU64::new(0),
            max_handler_micros: AtomicU64::new(0),
            worker_latency_histograms: (0..worker_count)
                .map(|_| WorkerLatencyHistograms {
                    queue_wait: LatencyHistogram::new(),
                    handler: LatencyHistogram::new(),
                })
                .collect(),
        }
    }
}

pub struct StandaloneHttpService {
    listener: TcpListener,
    runtime: Arc<ConcurrentConversationRuntime>,
    construction_runtime: Option<Arc<ConstructionRuntime>>,
    config: StandaloneHttpServiceConfig,
    counters: Arc<ServiceCounters>,
    connection_queue: Arc<BoundedConnectionQueue>,
}

impl StandaloneHttpService {
    pub fn bind(
        config: StandaloneHttpServiceConfig,
        runtime: ConcurrentConversationRuntime,
    ) -> Result<Self, StandaloneServiceError> {
        config.validate()?;
        // Service readiness includes the sealed lexical index.  No accepted
        // request may pay the one-time parse/index cost as cold TTFT.
        crate::lexical_knowledge_pack::preload_builtin_pack();
        let construction_runtime = match (
            config.construction_store_path.as_deref(),
            config.persona_contrastive_store_path.as_deref(),
        ) {
            (Some(construction), Some(contrastive)) => {
                let runtime = match (
                    config.persona_latent_operator_store_path.as_deref(),
                    config.language_state_controller_store_path.as_deref(),
                ) {
                    (Some(latent), Some(controller)) => {
                        ConstructionRuntime::load_from_paths_with_latent_and_controller(
                            construction,
                            contrastive,
                            latent,
                            controller,
                        )
                    }
                    (Some(latent), None) => ConstructionRuntime::load_from_paths_with_latent(
                        construction,
                        contrastive,
                        latent,
                    ),
                    _ => ConstructionRuntime::load_from_paths(construction, contrastive),
                }
                .map_err(|_| StandaloneServiceError::ConstructionArtifactInvalid)?;
                Some(Arc::new(runtime))
            }
            (Some(construction), None) => Some(Arc::new(
                ConstructionRuntime::load_from_path(construction)
                    .map_err(|_| StandaloneServiceError::ConstructionArtifactInvalid)?,
            )),
            (None, _) => None,
        };
        let mut addresses = config
            .bind_address
            .to_socket_addrs()
            .map_err(StandaloneServiceError::Bind)?;
        let address = addresses
            .next()
            .ok_or(StandaloneServiceError::InvalidConfiguration)?;
        let listener = TcpListener::bind(address).map_err(StandaloneServiceError::Bind)?;
        let worker_count = config.worker_count;
        Ok(Self {
            listener,
            runtime: Arc::new(runtime),
            construction_runtime,
            connection_queue: Arc::new(BoundedConnectionQueue::new(config.queue_capacity)),
            config,
            counters: Arc::new(ServiceCounters::new(worker_count)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, StandaloneServiceError> {
        self.listener
            .local_addr()
            .map_err(StandaloneServiceError::LocalAddress)
    }

    pub fn stats(&self) -> StandaloneServiceStatsIR {
        service_stats(&self.config, &self.counters, &self.connection_queue)
    }

    pub fn serve(&self) -> Result<(), StandaloneServiceError> {
        self.serve_inner(None).map(|_| ())
    }

    /// Bounded run used by a release canary. Every accepted connection counts,
    /// including malformed or rejected requests, so the host always terminates.
    pub fn serve_n_connections(
        &self,
        connection_limit: usize,
    ) -> Result<StandaloneServiceRunReportIR, StandaloneServiceError> {
        if connection_limit == 0 {
            return Err(StandaloneServiceError::InvalidConfiguration);
        }
        self.serve_inner(Some(connection_limit))
    }

    fn serve_inner(
        &self,
        connection_limit: Option<usize>,
    ) -> Result<StandaloneServiceRunReportIR, StandaloneServiceError> {
        let queue = Arc::clone(&self.connection_queue);
        let mut workers: Vec<thread::JoinHandle<()>> = Vec::with_capacity(self.config.worker_count);

        for worker_index in 0..self.config.worker_count {
            let worker_queue = Arc::clone(&queue);
            let runtime = Arc::clone(&self.runtime);
            let construction_runtime = self.construction_runtime.clone();
            let counters = Arc::clone(&self.counters);
            let config = self.config.clone();
            let worker = match thread::Builder::new()
                .name(format!("b-core-http-{worker_index}"))
                .spawn(move || {
                    worker_loop(
                        worker_index,
                        worker_queue,
                        runtime,
                        construction_runtime,
                        counters,
                        config,
                    )
                }) {
                Ok(worker) => worker,
                Err(error) => {
                    queue.close();
                    join_workers(workers);
                    return Err(StandaloneServiceError::WorkerSpawn(error));
                }
            };
            workers.push(worker);
        }

        let mut accepted = 0usize;
        for incoming in self.listener.incoming() {
            let stream = match incoming {
                Ok(stream) => stream,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    queue.close();
                    join_workers(workers);
                    return Err(StandaloneServiceError::Bind(error));
                }
            };
            accepted += 1;
            self.counters
                .accepted_connections
                .fetch_add(1, Ordering::AcqRel);
            match queue.try_push(stream) {
                QueuePushResult::Queued => {}
                QueuePushResult::Full(mut stream) => {
                    self.counters
                        .rejected_connections
                        .fetch_add(1, Ordering::AcqRel);
                    // A rejected connection never enters a worker, but drain
                    // only bytes already buffered before replying. This keeps
                    // overload handling bounded while avoiding a reset caused
                    // by closing a socket with a complete small request still
                    // unread in the receive buffer.
                    discard_buffered_request_bytes(&mut stream);
                    let _ = write_json_response(
                        &mut stream,
                        503,
                        "Service Unavailable",
                        &json!({"ok": false, "error": "SERVER_BUSY"}),
                    );
                }
                QueuePushResult::Closed => {
                    queue.close();
                    join_workers(workers);
                    return Err(StandaloneServiceError::QueueDisconnected);
                }
            }
            if connection_limit.is_some_and(|limit| accepted >= limit) {
                break;
            }
        }
        queue.close();

        for worker in workers {
            worker
                .join()
                .map_err(|_| StandaloneServiceError::WorkerPanicked)?;
        }

        Ok(StandaloneServiceRunReportIR {
            service: service_stats(&self.config, &self.counters, &queue),
            runtime: self.runtime.stats(),
        })
    }
}

fn join_workers(workers: Vec<thread::JoinHandle<()>>) {
    for worker in workers {
        let _ = worker.join();
    }
}

fn worker_loop(
    worker_index: usize,
    queue: Arc<BoundedConnectionQueue>,
    runtime: Arc<ConcurrentConversationRuntime>,
    construction_runtime: Option<Arc<ConstructionRuntime>>,
    counters: Arc<ServiceCounters>,
    config: StandaloneHttpServiceConfig,
) {
    let latency_histograms = &counters.worker_latency_histograms[worker_index];
    while let Some(queued) = queue.recv() {
        let queue_wait_micros = duration_micros(queued.enqueued_at.elapsed());
        counters
            .total_queue_wait_micros
            .fetch_add(queue_wait_micros, Ordering::AcqRel);
        update_peak_u64(&counters.max_queue_wait_micros, queue_wait_micros);
        latency_histograms.queue_wait.observe(queue_wait_micros);
        let mut stream = queued.stream;
        let active = counters
            .current_active_workers
            .fetch_add(1, Ordering::AcqRel)
            + 1;
        update_peak(&counters.peak_active_workers, active);
        let _guard = ActiveWorkerGuard {
            counters: Arc::clone(&counters),
        };
        configure_stream(&stream, config.io_timeout_millis);
        let handler_started = Instant::now();
        let outcome = handle_connection(
            &mut stream,
            &runtime,
            construction_runtime.as_deref(),
            &counters,
            &config,
            &queue,
        );
        let handler_micros = duration_micros(handler_started.elapsed());
        counters
            .total_handler_micros
            .fetch_add(handler_micros, Ordering::AcqRel);
        update_peak_u64(&counters.max_handler_micros, handler_micros);
        latency_histograms.handler.observe(handler_micros);
        if !matches!(outcome, Ok(HttpHandlingOutcome::Success)) {
            counters.failed_requests.fetch_add(1, Ordering::AcqRel);
        }
        counters.completed_requests.fetch_add(1, Ordering::AcqRel);
    }
}

/// A bounded, multi-consumer connection queue.  Unlike a `Receiver` behind a
/// mutex, workers release the queue lock while waiting, so all configured
/// workers can be ready for a burst at once.  Capacity rejection remains
/// deterministic and fail-closed.
struct BoundedConnectionQueue {
    state: Mutex<BoundedConnectionQueueState>,
    ready: Condvar,
    capacity: usize,
    peak_pending_connections: AtomicUsize,
}

struct BoundedConnectionQueueState {
    pending: VecDeque<QueuedConnection>,
    closed: bool,
}

struct QueuedConnection {
    stream: TcpStream,
    enqueued_at: Instant,
}

enum QueuePushResult {
    Queued,
    Full(TcpStream),
    Closed,
}

impl BoundedConnectionQueue {
    fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(BoundedConnectionQueueState {
                pending: VecDeque::with_capacity(capacity),
                closed: false,
            }),
            ready: Condvar::new(),
            capacity,
            peak_pending_connections: AtomicUsize::new(0),
        }
    }

    fn try_push(&self, stream: TcpStream) -> QueuePushResult {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return QueuePushResult::Closed,
        };
        if state.closed {
            return QueuePushResult::Closed;
        }
        if state.pending.len() >= self.capacity {
            return QueuePushResult::Full(stream);
        }
        state.pending.push_back(QueuedConnection {
            stream,
            enqueued_at: Instant::now(),
        });
        update_peak(&self.peak_pending_connections, state.pending.len());
        self.ready.notify_one();
        QueuePushResult::Queued
    }

    fn recv(&self) -> Option<QueuedConnection> {
        let mut state = self.state.lock().ok()?;
        loop {
            if let Some(stream) = state.pending.pop_front() {
                return Some(stream);
            }
            if state.closed {
                return None;
            }
            state = self.ready.wait(state).ok()?;
        }
    }

    fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            self.ready.notify_all();
        }
    }

    fn current_pending_connections(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.pending.len())
            .unwrap_or_default()
    }

    fn peak_pending_connections(&self) -> usize {
        self.peak_pending_connections.load(Ordering::Acquire)
    }
}

struct ActiveWorkerGuard {
    counters: Arc<ServiceCounters>,
}

impl Drop for ActiveWorkerGuard {
    fn drop(&mut self) {
        self.counters
            .current_active_workers
            .fetch_sub(1, Ordering::AcqRel);
    }
}

fn configure_stream(stream: &TcpStream, timeout_millis: u64) {
    let timeout = Some(Duration::from_millis(timeout_millis));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);
    let _ = stream.set_nodelay(true);
}

fn handle_connection(
    stream: &mut TcpStream,
    runtime: &ConcurrentConversationRuntime,
    construction_runtime: Option<&ConstructionRuntime>,
    counters: &ServiceCounters,
    config: &StandaloneHttpServiceConfig,
    connection_queue: &BoundedConnectionQueue,
) -> io::Result<HttpHandlingOutcome> {
    let request = match read_http_request(stream, config.max_body_bytes) {
        Ok(request) => request,
        Err(error) => {
            return write_json_response(
                stream,
                error.status,
                error.reason,
                &json!({"ok": false, "error": error.code}),
            )
            .map(|_| HttpHandlingOutcome::Failed);
        }
    };

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => write_json_response(
            stream,
            200,
            "OK",
            &json!({"ok": true, "standalone": true, "gpu_used": false}),
        )
        .map(|_| HttpHandlingOutcome::Success),
        ("GET", "/v1/runtime/stats" | "/v1/stats") => write_json_response(
            stream,
            200,
            "OK",
            &json!({
                "ok": true,
                "service": service_stats(config, counters, connection_queue),
                "runtime": runtime.stats(),
            }),
        )
        .map(|_| HttpHandlingOutcome::Success),
        ("POST", "/v1/conversation/close") => {
            let parsed: ConversationCloseRequest = match serde_json::from_slice(&request.body) {
                Ok(parsed) => parsed,
                Err(_) => {
                    return write_json_response(
                        stream,
                        400,
                        "Bad Request",
                        &json!({"ok": false, "error": "INVALID_JSON_OR_REQUEST"}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            match runtime.close_conversation(&parsed.conversation_id) {
                Ok(closed) => write_json_response(
                    stream,
                    200,
                    "OK",
                    &json!({"ok": true, "closed": closed}),
                )
                .map(|_| HttpHandlingOutcome::Success),
                Err(ConcurrentRuntimeError::Busy) => write_json_response(
                    stream,
                    503,
                    "Service Unavailable",
                    &json!({"ok": false, "error": "RUNTIME_BUSY"}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
                Err(error) => write_json_response(
                    stream,
                    422,
                    "Unprocessable Content",
                    &json!({"ok": false, "error": format!("{error:?}")}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
            }
        }
        ("POST", "/v1/conversation/turn") => {
            let parsed: ConversationTurnRequestIR = match serde_json::from_slice(&request.body) {
                Ok(parsed) => parsed,
                Err(_) => {
                    return write_json_response(
                        stream,
                        400,
                        "Bad Request",
                        &json!({"ok": false, "error": "INVALID_JSON_OR_REQUEST"}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            match runtime.process_conversation_turn(&parsed) {
                Ok(response) => write_json_response(stream, 200, "OK", &response)
                    .map(|_| HttpHandlingOutcome::Success),
                Err(ConcurrentRuntimeError::Busy) => write_json_response(
                    stream,
                    503,
                    "Service Unavailable",
                    &json!({"ok": false, "error": "RUNTIME_BUSY"}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
                Err(error) => write_json_response(
                    stream,
                    422,
                    "Unprocessable Content",
                    &json!({"ok": false, "error": format!("{error:?}")}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
            }
        }
        ("POST", "/v1/document/realize") => {
            let parsed: ApprovedDocumentRequestIR = match serde_json::from_slice(&request.body) {
                Ok(parsed) => parsed,
                Err(_) => {
                    return write_json_response(
                        stream,
                        400,
                        "Bad Request",
                        &json!({"ok": false, "error": "INVALID_JSON_OR_REQUEST"}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            let projected = match project_approved_document(
                &parsed.response,
                &parsed.event_discourse_states,
                parsed.discourse_focus_state.as_ref(),
                parsed.discourse_completed_turns,
                &parsed.event_pragmatic_contexts,
            ) {
                Ok(projected) => projected,
                Err(error) => {
                    return write_json_response(
                        stream,
                        422,
                        "Unprocessable Content",
                        &json!({"ok": false, "error": error}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            match realize_document_response(&projected, parsed.output_language) {
                Ok(response) => write_json_response(stream, 200, "OK", &response)
                    .map(|_| HttpHandlingOutcome::Success),
                Err(error) => write_json_response(
                    stream,
                    422,
                    "Unprocessable Content",
                    &json!({"ok": false, "error": error}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
            }
        }
        ("POST", "/v1/document/realize/persona") => {
            let Some(construction_runtime) = construction_runtime else {
                return write_json_response(
                    stream,
                    503,
                    "Service Unavailable",
                    &json!({"ok": false, "error": "CONSTRUCTION_RUNTIME_DISABLED"}),
                )
                .map(|_| HttpHandlingOutcome::Failed);
            };
            let parsed: PersonaConstructionDocumentRequestIR =
                match serde_json::from_slice(&request.body) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        return write_json_response(
                            stream,
                            400,
                            "Bad Request",
                            &json!({"ok": false, "error": "INVALID_JSON_OR_REQUEST"}),
                        )
                        .map(|_| HttpHandlingOutcome::Failed);
                    }
                };
            let projected = match project_approved_document(
                &parsed.document.response,
                &parsed.document.event_discourse_states,
                parsed.document.discourse_focus_state.as_ref(),
                parsed.document.discourse_completed_turns,
                &parsed.document.event_pragmatic_contexts,
            ) {
                Ok(projected) => projected,
                Err(error) => {
                    return write_json_response(
                        stream,
                        422,
                        "Unprocessable Content",
                        &json!({"ok": false, "error": error}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            match construction_runtime.realize(
                &projected,
                parsed.document.output_language,
                parsed.context,
            ) {
                Ok(response) => write_json_response(stream, 200, "OK", &response)
                    .map(|_| HttpHandlingOutcome::Success),
                Err(error) => write_json_response(
                    stream,
                    422,
                    "Unprocessable Content",
                    &json!({"ok": false, "error": error}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
            }
        }
        ("POST", "/v1/document/interpret") => {
            let parsed: ApprovedDocumentInterpretRequestIR =
                match serde_json::from_slice(&request.body) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        return write_json_response(
                            stream,
                            400,
                            "Bad Request",
                            &json!({"ok": false, "error": "INVALID_JSON_OR_REQUEST"}),
                        )
                        .map(|_| HttpHandlingOutcome::Failed);
                    }
                };
            let projected = match project_approved_document(
                &parsed.response,
                &parsed.event_discourse_states,
                parsed.discourse_focus_state.as_ref(),
                parsed.discourse_completed_turns,
                &parsed.event_pragmatic_contexts,
            ) {
                Ok(projected) => projected,
                Err(error) => {
                    return write_json_response(
                        stream,
                        422,
                        "Unprocessable Content",
                        &json!({"ok": false, "error": error}),
                    )
                    .map(|_| HttpHandlingOutcome::Failed);
                }
            };
            match interpret_document_semantics(&parsed.markdown, &projected, parsed.output_language)
            {
                Ok(interpretation) => write_json_response(stream, 200, "OK", &interpretation)
                    .map(|_| HttpHandlingOutcome::Success),
                Err(error) => write_json_response(
                    stream,
                    422,
                    "Unprocessable Content",
                    &json!({"ok": false, "error": error}),
                )
                .map(|_| HttpHandlingOutcome::Failed),
            }
        }
        _ => write_json_response(
            stream,
            404,
            "Not Found",
            &json!({"ok": false, "error": "ROUTE_NOT_FOUND"}),
        )
        .map(|_| HttpHandlingOutcome::Failed),
    }
}

enum HttpHandlingOutcome {
    Success,
    Failed,
}

fn project_approved_document(
    response: &ApprovedCompositionalResponseIR,
    explicit_states: &[ApprovedEventDiscourseStateIR],
    discourse_focus: Option<&DiscourseFocusStateIR>,
    completed_turns: Option<u64>,
    pragmatic_contexts: &[ApprovedEventPragmaticContextIR],
) -> Result<ApprovedCompositionalResponseIR, String> {
    let mut states = explicit_states.to_vec();
    if !pragmatic_contexts.is_empty() {
        let (Some(discourse_focus), Some(completed_turns)) = (discourse_focus, completed_turns)
        else {
            return Err("MISSING_APPROVED_EVENT_DISCOURSE_CONTEXT".into());
        };
        let derived = response.derive_event_discourse_states(
            discourse_focus,
            completed_turns,
            pragmatic_contexts,
        )?;
        if derived.iter().any(|candidate| {
            states
                .iter()
                .any(|state| state.subject_node_id == candidate.subject_node_id)
        }) {
            return Err("AMBIGUOUS_APPROVED_EVENT_DISCOURSE_SOURCE".into());
        }
        states.extend(derived);
    }
    response.apply_event_discourse_states(&states)
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

struct HttpRequestError {
    status: u16,
    reason: &'static str,
    code: &'static str,
}

fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: usize,
) -> Result<HttpRequest, HttpRequestError> {
    let mut bytes = Vec::with_capacity(4096);
    let header_end = loop {
        if let Some(position) = find_bytes(&bytes, b"\r\n\r\n") {
            break position + 4;
        }
        if bytes.len() >= MAX_HTTP_HEADER_BYTES {
            return Err(http_error(
                431,
                "Request Header Fields Too Large",
                "HEADERS_TOO_LARGE",
            ));
        }
        let mut chunk = [0u8; 4096];
        let read = stream
            .read(&mut chunk)
            .map_err(|_| http_error(400, "Bad Request", "READ_FAILED"))?;
        if read == 0 {
            return Err(http_error(400, "Bad Request", "INCOMPLETE_HEADERS"));
        }
        bytes.extend_from_slice(&chunk[..read]);
    };

    let header_text = std::str::from_utf8(&bytes[..header_end - 4])
        .map_err(|_| http_error(400, "Bad Request", "NON_UTF8_HEADERS"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| http_error(400, "Bad Request", "MISSING_REQUEST_LINE"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| http_error(400, "Bad Request", "MISSING_METHOD"))?
        .to_string();
    let raw_path = parts
        .next()
        .ok_or_else(|| http_error(400, "Bad Request", "MISSING_PATH"))?
        .to_string();
    let version = parts
        .next()
        .ok_or_else(|| http_error(400, "Bad Request", "MISSING_HTTP_VERSION"))?;
    if parts.next().is_some() || !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
        return Err(http_error(400, "Bad Request", "INVALID_REQUEST_LINE"));
    }
    if !matches!(method.as_str(), "GET" | "POST") {
        return Err(http_error(405, "Method Not Allowed", "METHOD_NOT_ALLOWED"));
    }

    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(http_error(400, "Bad Request", "INVALID_HEADER"));
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(http_error(400, "Bad Request", "DUPLICATE_CONTENT_LENGTH"));
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| http_error(400, "Bad Request", "INVALID_CONTENT_LENGTH"))?,
            );
        }
        if name.trim().eq_ignore_ascii_case("transfer-encoding") {
            return Err(http_error(
                400,
                "Bad Request",
                "TRANSFER_ENCODING_UNSUPPORTED",
            ));
        }
    }

    let expected_body = if method == "POST" {
        content_length
            .ok_or_else(|| http_error(411, "Length Required", "CONTENT_LENGTH_REQUIRED"))?
    } else {
        content_length.unwrap_or(0)
    };
    if expected_body > max_body_bytes {
        return Err(http_error(413, "Content Too Large", "BODY_TOO_LARGE"));
    }
    while bytes.len() < header_end + expected_body {
        let remaining = header_end + expected_body - bytes.len();
        let mut chunk = vec![0u8; remaining.min(8192)];
        let read = stream
            .read(&mut chunk)
            .map_err(|_| http_error(400, "Bad Request", "READ_FAILED"))?;
        if read == 0 {
            return Err(http_error(400, "Bad Request", "INCOMPLETE_BODY"));
        }
        bytes.extend_from_slice(&chunk[..read]);
    }

    Ok(HttpRequest {
        method,
        path: raw_path.split('?').next().unwrap_or(&raw_path).into(),
        body: bytes[header_end..header_end + expected_body].to_vec(),
    })
}

fn write_json_response<T: Serialize>(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    value: &T,
) -> io::Result<()> {
    let body = serde_json::to_vec(value).map_err(io::Error::other)?;
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

/// Refusal must remain bounded: consume only what the kernel has already
/// delivered and never wait for a header or body from an overloaded client.
fn discard_buffered_request_bytes(stream: &mut TcpStream) {
    const MAX_DISCARDED_BYTES: usize = 16 * 1024;
    const OVERLOAD_DRAIN_WAIT: Duration = Duration::from_millis(10);
    // A short bounded first read covers a request that is in flight while the
    // listener decides to reject it. It is intentionally independent of the
    // normal handler timeout so an overloaded peer cannot occupy the acceptor.
    let _ = stream.set_read_timeout(Some(OVERLOAD_DRAIN_WAIT));
    let mut scratch = [0_u8; 1024];
    let mut remaining = MAX_DISCARDED_BYTES;
    let first_limit = remaining.min(scratch.len());
    if let Ok(read) = stream.read(&mut scratch[..first_limit]) {
        remaining = remaining.saturating_sub(read);
    }
    let _ = stream.set_nonblocking(true);
    while remaining > 0 {
        let limit = remaining.min(scratch.len());
        match stream.read(&mut scratch[..limit]) {
            Ok(0) => break,
            Ok(read) => remaining -= read,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(None);
}

fn service_stats(
    config: &StandaloneHttpServiceConfig,
    counters: &ServiceCounters,
    connection_queue: &BoundedConnectionQueue,
) -> StandaloneServiceStatsIR {
    let completed_requests = counters.completed_requests.load(Ordering::Acquire);
    StandaloneServiceStatsIR {
        schema: STANDALONE_SERVICE_STATS_SCHEMA.into(),
        worker_count: config.worker_count,
        queue_capacity: config.queue_capacity,
        current_pending_connections: connection_queue.current_pending_connections(),
        peak_pending_connections: connection_queue.peak_pending_connections(),
        accepted_connections: counters.accepted_connections.load(Ordering::Acquire),
        completed_requests,
        rejected_connections: counters.rejected_connections.load(Ordering::Acquire),
        failed_requests: counters.failed_requests.load(Ordering::Acquire),
        current_active_workers: counters.current_active_workers.load(Ordering::Acquire),
        peak_active_workers: counters.peak_active_workers.load(Ordering::Acquire),
        average_queue_wait_micros: average_micros(
            counters.total_queue_wait_micros.load(Ordering::Acquire),
            completed_requests,
        ),
        p95_queue_wait_micros_upper_bound: histogram_percentile_upper_bound_micros(
            &counters.worker_latency_histograms,
            completed_requests,
            95,
            counters.max_queue_wait_micros.load(Ordering::Acquire),
            |histograms| &histograms.queue_wait,
        ),
        max_queue_wait_micros: counters.max_queue_wait_micros.load(Ordering::Acquire),
        average_handler_micros: average_micros(
            counters.total_handler_micros.load(Ordering::Acquire),
            completed_requests,
        ),
        p95_handler_micros_upper_bound: histogram_percentile_upper_bound_micros(
            &counters.worker_latency_histograms,
            completed_requests,
            95,
            counters.max_handler_micros.load(Ordering::Acquire),
            |histograms| &histograms.handler,
        ),
        max_handler_micros: counters.max_handler_micros.load(Ordering::Acquire),
    }
}

fn histogram_percentile_upper_bound_micros(
    worker_histograms: &[WorkerLatencyHistograms],
    observations: usize,
    percentile: usize,
    max_micros: u64,
    select: impl Fn(&WorkerLatencyHistograms) -> &LatencyHistogram,
) -> u64 {
    if observations == 0 {
        return 0;
    }
    let rank = observations.saturating_mul(percentile).div_ceil(100);
    let mut cumulative = 0usize;
    for bucket in 0..LATENCY_HISTOGRAM_BUCKET_COUNT {
        cumulative = cumulative.saturating_add(
            worker_histograms
                .iter()
                .map(|histograms| select(histograms).buckets[bucket].load(Ordering::Relaxed))
                .sum::<usize>(),
        );
        if cumulative >= rank {
            return if bucket == LATENCY_HISTOGRAM_LAST_REGULAR_BUCKET {
                max_micros
            } else {
                (u64::try_from(bucket).unwrap_or_default() + 1)
                    .saturating_mul(LATENCY_HISTOGRAM_BUCKET_WIDTH_MICROS)
                    .saturating_sub(1)
                    .min(max_micros)
            };
        }
    }
    max_micros
}

fn update_peak(peak: &AtomicUsize, candidate: usize) {
    let mut current = peak.load(Ordering::Acquire);
    while candidate > current {
        match peak.compare_exchange_weak(current, candidate, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

fn update_peak_u64(peak: &AtomicU64, candidate: u64) {
    let mut current = peak.load(Ordering::Acquire);
    while candidate > current {
        match peak.compare_exchange_weak(current, candidate, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

fn duration_micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

fn average_micros(total_micros: u64, completed_requests: usize) -> u64 {
    match u64::try_from(completed_requests) {
        Ok(0) | Err(_) => 0,
        Ok(completed) => total_micros / completed,
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn http_error(status: u16, reason: &'static str, code: &'static str) -> HttpRequestError {
    HttpRequestError {
        status,
        reason,
        code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approved_response::{
        compositional_response_sha256, ApprovedClauseUnitIR, ApprovedCompositionalClaimIR,
        ApprovedDiscourseRelationIR, ApprovedEventExpressionPreferenceIR, ApprovedEventFocusIR,
        ApprovedEventPerspectiveIR, ApprovedEventPhaseIR, ApprovedEventPragmaticContextIR,
        ApprovedEventPredicateSenseIR, ApprovedEventRealizationClassIR, ApprovedEventRealizationIR,
        ApprovedEventVoiceIR, ApprovedLexicalNodeIR, ApprovedModalityIR, ApprovedOpenValueIR,
        ApprovedOperationIR, ApprovedRelationTypeIR, ApprovedResponseStyleIR,
        ApprovedSemanticTypeIR, ApprovedSpeechActIR, ApprovedVerbosityIR,
        APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA, COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
    };
    use crate::conversation::{ConversationInputModalityIR, CONVERSATION_TURN_REQUEST_SCHEMA};
    use crate::discourse_focus::DiscourseFocusCandidateIR;
    use crate::language_knowledge::LanguageRegisterIR;
    use std::sync::Barrier;
    use std::time::{Duration, Instant};

    fn inspection_document_request() -> ApprovedDocumentRequestIR {
        let event = ApprovedLexicalNodeIR {
            node_id: "filter_inspection".into(),
            semantic_type: ApprovedSemanticTypeIR::Event,
            canonical_lexical_label: "음압 공조 필터 점검".into(),
        };
        let claims = [
            (
                "P_INSPECT_AGENT",
                ApprovedRelationTypeIR::Agent,
                "facility_engineer",
                ApprovedSemanticTypeIR::Person,
                "시설 엔지니어",
            ),
            (
                "P_INSPECT_THEME",
                ApprovedRelationTypeIR::Theme,
                "negative_pressure_filter",
                ApprovedSemanticTypeIR::Concept,
                "음압 공조 필터",
            ),
        ]
        .into_iter()
        .map(
            |(proposition_id, relation, node_id, semantic_type, label)| {
                ApprovedCompositionalClaimIR {
                    proposition_id: proposition_id.into(),
                    subject: event.clone(),
                    relation,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: node_id.into(),
                        semantic_type,
                        canonical_lexical_label: label.into(),
                    }),
                    polarity: true,
                    modality: ApprovedModalityIR::Asserted,
    status_frame: None,
}
            },
        )
        .collect::<Vec<_>>();
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: ApprovedSpeechActIR::Inform,
            operation: ApprovedOperationIR::Assert,
            clause_plan: claims
                .iter()
                .enumerate()
                .map(|(index, claim)| ApprovedClauseUnitIR {
                    unit_index: index,
                    role: "ASSERT".into(),
                    proposition_ids: vec![claim.proposition_id.clone()],
                    predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
                })
                .collect(),
            claims,
            event_realizations: vec![ApprovedEventRealizationIR {
                subject_node_id: "filter_inspection".into(),
                class: ApprovedEventRealizationClassIR::Inspection,
                predicate_sense: Some(ApprovedEventPredicateSenseIR::Inspect),
                phase: Some(ApprovedEventPhaseIR::Completed),
                perspective: Some(ApprovedEventPerspectiveIR {
                    voice: ApprovedEventVoiceIR::Active,
                    focus: ApprovedEventFocusIR::Agent,
                }),
                voice: None,
                information_structure: None,
            }],
            discourse_relation: ApprovedDiscourseRelationIR::Statement,
            style: ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Formal,
                verbosity: ApprovedVerbosityIR::Short,
            },
            source_world_state_sha256: "a".repeat(64),
            source_deliberation_sha256: "b".repeat(64),
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        ApprovedDocumentRequestIR {
            response,
            event_discourse_states: Vec::new(),
            discourse_focus_state: None,
            discourse_completed_turns: None,
            event_pragmatic_contexts: Vec::new(),
            output_language: LanguageCodeIR::Korean,
        }
    }

    #[test]
    fn invalid_capacity_is_rejected_before_bind() {
        let config = StandaloneHttpServiceConfig {
            worker_count: 0,
            ..StandaloneHttpServiceConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(StandaloneServiceError::InvalidConfiguration)
        ));
    }

    #[test]
    fn document_http_paths_share_typed_event_discourse_projection() {
        let event = ApprovedLexicalNodeIR {
            node_id: "transfer".into(),
            semantic_type: ApprovedSemanticTypeIR::Event,
            canonical_lexical_label: "전송".into(),
        };
        let values = [
            (
                "P_AGENT",
                ApprovedRelationTypeIR::Agent,
                "minsu",
                ApprovedSemanticTypeIR::Person,
                "민수",
            ),
            (
                "P_THEME",
                ApprovedRelationTypeIR::Theme,
                "report",
                ApprovedSemanticTypeIR::Concept,
                "보고서",
            ),
            (
                "P_DESTINATION",
                ApprovedRelationTypeIR::Destination,
                "server",
                ApprovedSemanticTypeIR::Location,
                "서버",
            ),
        ];
        let claims = values
            .into_iter()
            .map(
                |(id, relation, node_id, semantic_type, label)| ApprovedCompositionalClaimIR {
                    proposition_id: id.into(),
                    subject: event.clone(),
                    relation,
                    value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                        node_id: node_id.into(),
                        semantic_type,
                        canonical_lexical_label: label.into(),
                    }),
                    polarity: true,
                    modality: ApprovedModalityIR::Asserted,
    status_frame: None,
},
            )
            .collect::<Vec<_>>();
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: ApprovedSpeechActIR::Inform,
            operation: ApprovedOperationIR::Assert,
            clause_plan: claims
                .iter()
                .enumerate()
                .map(|(index, claim)| ApprovedClauseUnitIR {
                    unit_index: index,
                    role: "ASSERT".into(),
                    proposition_ids: vec![claim.proposition_id.clone()],
                    predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
                })
                .collect(),
            claims,
            event_realizations: vec![ApprovedEventRealizationIR {
                subject_node_id: "transfer".into(),
                class: ApprovedEventRealizationClassIR::Transfer,
                predicate_sense: Some(ApprovedEventPredicateSenseIR::Transfer),
                phase: Some(ApprovedEventPhaseIR::Completed),
                perspective: None,
                voice: None,
                information_structure: None,
            }],
            discourse_relation: ApprovedDiscourseRelationIR::Statement,
            style: ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Formal,
                verbosity: ApprovedVerbosityIR::Short,
            },
            source_world_state_sha256: "a".repeat(64),
            source_deliberation_sha256: "b".repeat(64),
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        let mut discourse_focus = DiscourseFocusStateIR::default();
        discourse_focus.apply_turn(
            1,
            &[DiscourseFocusCandidateIR::explicit_topic(
                "표면은 신뢰하지 않음",
                Some("server"),
            )],
        );
        let pragmatic_context = ApprovedEventPragmaticContextIR {
            schema: APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA.into(),
            subject_node_id: "transfer".into(),
            requested_focus_node_id: Some("server".into()),
            backgrounded_node_ids: vec!["minsu".into()],
            recoverable_node_ids: vec!["minsu".into()],
            expression_preference: ApprovedEventExpressionPreferenceIR::Auto,
        };

        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: 2,
                queue_capacity: 4,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .unwrap();
        let address = service.local_addr().unwrap();
        let server = thread::spawn(move || service.serve_n_connections(2).unwrap());

        let realize_request = ApprovedDocumentRequestIR {
            response: response.clone(),
            event_discourse_states: Vec::new(),
            discourse_focus_state: Some(discourse_focus.clone()),
            discourse_completed_turns: Some(1),
            event_pragmatic_contexts: vec![pragmatic_context.clone()],
            output_language: LanguageCodeIR::Korean,
        };
        let realize_body = serde_json::to_vec(&realize_request).unwrap();
        let realized_bytes = post_json(address, "/v1/document/realize", &realize_body);
        let realized: crate::document_response::DocumentResponseOutputIR =
            serde_json::from_slice(&realized_bytes).unwrap();
        assert!(
            realized
                .markdown
                .contains("서버에는 보고서가 전송됐습니다."),
            "{}",
            realized.markdown
        );

        let interpret_request = ApprovedDocumentInterpretRequestIR {
            response,
            event_discourse_states: Vec::new(),
            discourse_focus_state: Some(discourse_focus),
            discourse_completed_turns: Some(1),
            event_pragmatic_contexts: vec![pragmatic_context],
            output_language: LanguageCodeIR::Korean,
            markdown: realized.markdown,
        };
        let interpret_body = serde_json::to_vec(&interpret_request).unwrap();
        let interpreted_bytes = post_json(address, "/v1/document/interpret", &interpret_body);
        let interpreted: crate::document_response::DocumentSemanticInterpretationIR =
            serde_json::from_slice(&interpreted_bytes).unwrap();
        assert_eq!(
            interpreted.recovered_claim_ids,
            vec!["P_AGENT", "P_THEME", "P_DESTINATION"]
        );
        assert!(server.join().unwrap().service.failed_requests == 0);
    }

    #[test]
    fn document_http_realizes_explicit_inspection_event_frame() {
        let request = inspection_document_request();
        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: 1,
                queue_capacity: 1,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .unwrap();
        let address = service.local_addr().unwrap();
        let server = thread::spawn(move || service.serve_n_connections(1).unwrap());
        let response_bytes = post_json(
            address,
            "/v1/document/realize",
            &serde_json::to_vec(&request).unwrap(),
        );
        let realized: crate::document_response::DocumentResponseOutputIR =
            serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(
            realized.markdown,
            "시설 엔지니어가 음압 공조 필터를 점검했습니다."
        );
        assert!(realized.validate(&request.response));
        let report = server.join().unwrap();
        assert_eq!(report.service.completed_requests, 1);
        assert_eq!(report.service.failed_requests, 0);
    }

    fn post_json(address: SocketAddr, path: &str, body: &[u8]) -> Vec<u8> {
        let wire = format!(
            "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(wire.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert!(
            response.starts_with(b"HTTP/1.1 200 OK"),
            "unexpected response: {}",
            String::from_utf8_lossy(&response)
        );
        let body_start = find_bytes(&response, b"\r\n\r\n").unwrap() + 4;
        response[body_start..].to_vec()
    }

    #[test]
    fn stats_endpoint_reports_live_queue_capacity_and_depth() {
        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: 1,
                queue_capacity: 3,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .unwrap();
        let address = service.local_addr().unwrap();
        let server = thread::spawn(move || service.serve_n_connections(1).unwrap());
        let wire =
            format!("GET /v1/stats HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(wire.as_bytes()).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200 OK"));
        let body_start = find_bytes(&response, b"\r\n\r\n").unwrap() + 4;
        let body: serde_json::Value = serde_json::from_slice(&response[body_start..]).unwrap();
        assert_eq!(body["service"]["queue_capacity"], 3);
        assert_eq!(body["service"]["current_pending_connections"], 0);
        assert_eq!(body["service"]["peak_pending_connections"], 1);
        assert!(body["service"]["average_queue_wait_micros"].is_u64());
        assert!(body["service"]["p95_queue_wait_micros_upper_bound"].is_u64());
        assert!(body["service"]["max_queue_wait_micros"].is_u64());
        assert!(body["service"]["average_handler_micros"].is_u64());
        assert!(body["service"]["p95_handler_micros_upper_bound"].is_u64());
        assert!(body["service"]["max_handler_micros"].is_u64());
        let report = server.join().unwrap();
        assert_eq!(report.service.completed_requests, 1);
        assert_eq!(report.service.current_pending_connections, 0);
    }

    #[test]
    fn per_worker_latency_histograms_report_conservative_p95_without_shared_hot_buckets() {
        let counters = ServiceCounters::new(2);
        counters.worker_latency_histograms[0].queue_wait.observe(100);
        counters.worker_latency_histograms[0].handler.observe(2_100);
        counters.worker_latency_histograms[1].queue_wait.observe(1_500);
        counters.worker_latency_histograms[1].handler.observe(3_500);
        counters.completed_requests.store(2, Ordering::Release);

        assert_eq!(
            histogram_percentile_upper_bound_micros(
                &counters.worker_latency_histograms,
                2,
                95,
                1_500,
                |histograms| &histograms.queue_wait,
            ),
            1_500
        );
        assert_eq!(
            histogram_percentile_upper_bound_micros(
                &counters.worker_latency_histograms,
                2,
                95,
                3_500,
                |histograms| &histograms.handler,
            ),
            3_500
        );
    }

    #[test]
    fn bounded_queue_rejects_overflow_then_drains_without_leaking_pending_connections() {
        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = Arc::new(
            StandaloneHttpService::bind(
                StandaloneHttpServiceConfig {
                    bind_address: "127.0.0.1:0".into(),
                    worker_count: 1,
                    queue_capacity: 1,
                    io_timeout_millis: 1_000,
                    ..StandaloneHttpServiceConfig::default()
                },
                runtime,
            )
            .unwrap(),
        );
        let address = service.local_addr().unwrap();
        let server_service = Arc::clone(&service);
        let server = thread::spawn(move || server_service.serve_n_connections(3).unwrap());

        // The first connection keeps the sole worker in request parsing.  The
        // second is admitted to the one-slot queue; the third must receive a
        // deterministic fail-closed overload response rather than grow an
        // unbounded backlog.
        let mut active = TcpStream::connect(address).unwrap();
        active
            .write_all(b"GET /v1/stats HTTP/1.1\r\nHost: pending")
            .unwrap();
        let active_deadline = Instant::now() + Duration::from_secs(1);
        while service.stats().current_active_workers != 1 && Instant::now() < active_deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(service.stats().current_active_workers, 1);
        let pending = TcpStream::connect(address).unwrap();
        let pending_deadline = Instant::now() + Duration::from_secs(1);
        while service.stats().current_pending_connections != 1 && Instant::now() < pending_deadline
        {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(service.stats().current_pending_connections, 1);
        let mut overflow = TcpStream::connect(address).unwrap();
        overflow
            .write_all(b"GET /v1/stats HTTP/1.1\r\nHost: overflow\r\n\r\n")
            .unwrap();
        overflow.shutdown(std::net::Shutdown::Write).unwrap();
        let mut response = Vec::new();
        match overflow.read_to_end(&mut response) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
            Err(error) => panic!("failed to read overload response: {error}"),
        }
        assert!(
            response.starts_with(b"HTTP/1.1 503 Service Unavailable"),
            "unexpected overload response: {}",
            String::from_utf8_lossy(&response)
        );
        assert!(String::from_utf8_lossy(&response).contains("SERVER_BUSY"));

        drop(active);
        drop(pending);
        let report = server.join().unwrap();
        assert_eq!(report.service.accepted_connections, 3);
        assert_eq!(report.service.rejected_connections, 1);
        assert_eq!(report.service.peak_pending_connections, 1);
        assert_eq!(report.service.current_pending_connections, 0);
    }

    #[test]
    fn two_hundred_preconnected_document_sessions_complete_through_bounded_workers() {
        const CLIENTS: usize = 200;
        let request = inspection_document_request();
        let request_body = Arc::new(serde_json::to_vec(&request).unwrap());
        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: 32,
                queue_capacity: 256,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .unwrap();
        let address = service.local_addr().unwrap();
        let server = thread::spawn(move || service.serve_n_connections(CLIENTS).unwrap());
        let barrier = Arc::new(Barrier::new(CLIENTS + 1));
        let mut clients = Vec::with_capacity(CLIENTS);

        for _ in 0..CLIENTS {
            let barrier = Arc::clone(&barrier);
            let request_body = Arc::clone(&request_body);
            clients.push(thread::spawn(move || {
                let wire = format!(
                    "POST /v1/document/realize HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    request_body.len()
                );
                // This isolates bounded worker execution from the host's TCP
                // connection-establishment backlog during a 200-client burst.
                let mut stream = TcpStream::connect(address).unwrap();
                barrier.wait();
                let started = Instant::now();
                stream.write_all(wire.as_bytes()).unwrap();
                stream.write_all(request_body.as_slice()).unwrap();
                stream.shutdown(std::net::Shutdown::Write).unwrap();
                let mut response = Vec::new();
                stream.read_to_end(&mut response).unwrap();
                assert!(
                    response.starts_with(b"HTTP/1.1 200 OK"),
                    "unexpected response: {}",
                    String::from_utf8_lossy(&response)
                );
                let body_start = find_bytes(&response, b"\r\n\r\n").unwrap() + 4;
                let parsed: crate::document_response::DocumentResponseOutputIR =
                    serde_json::from_slice(&response[body_start..]).unwrap();
                assert_eq!(parsed.markdown, "시설 엔지니어가 음압 공조 필터를 점검했습니다.");
                started.elapsed()
            }));
        }

        let wall_started = Instant::now();
        barrier.wait();
        let latencies = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        let wall = wall_started.elapsed();
        let report = server.join().unwrap();
        assert_eq!(latencies.len(), CLIENTS);
        assert_eq!(report.service.accepted_connections, CLIENTS);
        assert_eq!(report.service.completed_requests, CLIENTS);
        assert_eq!(report.service.rejected_connections, 0);
        assert_eq!(report.service.failed_requests, 0);
        assert!(report.service.peak_active_workers <= 32);
        assert_eq!(report.service.current_pending_connections, 0);
        assert!(report.service.peak_pending_connections <= report.service.queue_capacity);
        // Document realization is bounded by the HTTP worker pool rather than
        // the conversation runtime's admission queue.
        assert_eq!(report.runtime.accepted_requests, 0);
        assert_eq!(report.runtime.rejected_busy_requests, 0);
        let mut latency_millis = latencies
            .iter()
            .map(|duration| duration.as_secs_f64() * 1_000.0)
            .collect::<Vec<_>>();
        latency_millis.sort_by(f64::total_cmp);
        eprintln!(
            "B_CORE_DOCUMENT_HTTP_200_SESSION_CANARY={}",
            serde_json::to_string(&json!({
                "status": "PASS",
                "measurement": "preconnected_request_release",
                "clients": CLIENTS,
                "workers": report.service.worker_count,
                "queue_capacity": report.service.queue_capacity,
                "peak_pending_connections": report.service.peak_pending_connections,
                "wall_millis": wall.as_secs_f64() * 1_000.0,
                "p50_millis": latency_millis[CLIENTS / 2],
                "p95_millis": latency_millis[(CLIENTS * 95 / 100).saturating_sub(1)],
                "max_millis": latency_millis[CLIENTS - 1],
                "throughput_requests_per_second": CLIENTS as f64 / wall.as_secs_f64(),
                "peak_active_workers": report.service.peak_active_workers,
                "rejected_connections": report.service.rejected_connections,
                "runtime_busy_rejections": report.runtime.rejected_busy_requests,
                "gpu_used": false,
            }))
            .unwrap()
        );
    }

    #[test]
    fn two_hundred_http_sessions_complete_through_bounded_workers() {
        const CLIENTS: usize = 200;
        let runtime = ConcurrentConversationRuntime::standalone_default().unwrap();
        let service = StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: 32,
                queue_capacity: 256,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .unwrap();
        let address = service.local_addr().unwrap();
        let server = thread::spawn(move || service.serve_n_connections(CLIENTS).unwrap());
        let barrier = Arc::new(Barrier::new(CLIENTS + 1));
        let mut clients = Vec::with_capacity(CLIENTS);

        for session in 0..CLIENTS {
            let barrier = Arc::clone(&barrier);
            clients.push(thread::spawn(move || {
                let request = ConversationTurnRequestIR {
                    schema: CONVERSATION_TURN_REQUEST_SCHEMA.into(),
                    conversation_id: format!("HTTP-SESSION-{session:03}"),
                    turn_index: 1,
                    request_id: format!("HTTP-REQUEST-{session:03}"),
                    modality: ConversationInputModalityIR::Text,
                    raw_text: "회의 시간이 오후 4시야.".into(),
                    input_confidence_millis: 1_000,
                    alternatives: Vec::new(),
                    output_language: Some(LanguageCodeIR::Korean),
                    context_tags: Vec::new(),
                    max_plan_steps: 12,
                };
                let body = serde_json::to_vec(&request).unwrap();
                let wire = format!(
                    "POST /v1/conversation/turn HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                barrier.wait();
                let started = Instant::now();
                let mut stream = TcpStream::connect(address).unwrap();
                stream.write_all(wire.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
                stream.shutdown(std::net::Shutdown::Write).unwrap();
                let mut response = Vec::new();
                stream.read_to_end(&mut response).unwrap();
                let body_start = find_bytes(&response, b"\r\n\r\n").unwrap() + 4;
                assert!(
                    response.starts_with(b"HTTP/1.1 200 OK"),
                    "unexpected response: {}",
                    String::from_utf8_lossy(&response)
                );
                let parsed: crate::cognitive::ConversationTurnResponseIR =
                    serde_json::from_slice(&response[body_start..]).unwrap();
                assert_eq!(parsed.conversation_id, request.conversation_id);
                started.elapsed()
            }));
        }

        let wall_started = Instant::now();
        barrier.wait();
        let latencies = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        let wall = wall_started.elapsed();
        let report = server.join().unwrap();
        assert_eq!(latencies.len(), CLIENTS);
        assert_eq!(report.service.accepted_connections, CLIENTS);
        assert_eq!(report.service.completed_requests, CLIENTS);
        assert_eq!(report.service.rejected_connections, 0);
        assert_eq!(report.service.failed_requests, 0);
        assert!(report.service.peak_active_workers <= 32);
        assert_eq!(report.service.current_pending_connections, 0);
        assert!(report.service.peak_pending_connections <= report.service.queue_capacity);
        assert_eq!(report.runtime.accepted_requests, CLIENTS);
        assert_eq!(report.runtime.rejected_busy_requests, 0);
        let mut latency_millis = latencies
            .iter()
            .map(|duration| duration.as_secs_f64() * 1_000.0)
            .collect::<Vec<_>>();
        latency_millis.sort_by(f64::total_cmp);
        eprintln!(
            "B_CORE_HTTP_200_SESSION_CANARY={} ",
            serde_json::to_string(&json!({
                "status": "PASS",
                "clients": CLIENTS,
                "workers": report.service.worker_count,
                "queue_capacity": report.service.queue_capacity,
                "peak_pending_connections": report.service.peak_pending_connections,
                "wall_millis": wall.as_secs_f64() * 1_000.0,
                "p50_millis": latency_millis[CLIENTS / 2],
                "p95_millis": latency_millis[(CLIENTS * 95 / 100).saturating_sub(1)],
                "max_millis": latency_millis[CLIENTS - 1],
                "throughput_requests_per_second": CLIENTS as f64 / wall.as_secs_f64(),
                "peak_active_workers": report.service.peak_active_workers,
                "rejected_connections": report.service.rejected_connections,
                "runtime_busy_rejections": report.runtime.rejected_busy_requests,
                "gpu_used": false,
            }))
            .unwrap()
        );
    }

    #[test]
    fn parser_rejects_chunked_bodies() {
        let error = parse_request_bytes(
            b"POST /v1/conversation/turn HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
        );
        assert_eq!(error, "TRANSFER_ENCODING_UNSUPPORTED");
    }

    fn parse_request_bytes(bytes: &[u8]) -> &'static str {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let bytes = bytes.to_vec();
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();
        let error = read_http_request(&mut stream, DEFAULT_MAX_HTTP_BODY_BYTES).unwrap_err();
        client.join().unwrap();
        error.code
    }
}
