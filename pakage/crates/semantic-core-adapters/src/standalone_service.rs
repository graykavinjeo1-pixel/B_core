use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::approved_response::{
    ApprovedCompositionalResponseIR, ApprovedEventDiscourseStateIR, ApprovedEventPragmaticContextIR,
};
use crate::concurrent_runtime::{
    ConcurrentConversationRuntime, ConcurrentRuntimeError, ConcurrentRuntimeStatsIR,
};
use crate::conversation::ConversationTurnRequestIR;
use crate::discourse_focus::DiscourseFocusStateIR;
use crate::document_response::{interpret_document_semantics, realize_document_response};
use crate::language_knowledge::LanguageCodeIR;

pub const STANDALONE_SERVICE_STATS_SCHEMA: &str = "B_CORE_STANDALONE_SERVICE_STATS_IR_1";
pub const DEFAULT_HTTP_WORKERS: usize = 32;
pub const DEFAULT_HTTP_QUEUE_CAPACITY: usize = 256;
pub const DEFAULT_MAX_HTTP_BODY_BYTES: usize = 1_048_576;
pub const DEFAULT_IO_TIMEOUT_MILLIS: u64 = 10_000;
pub const MAX_HTTP_WORKERS: usize = 128;
pub const MAX_HTTP_QUEUE_CAPACITY: usize = 4_096;
const MAX_HTTP_HEADER_BYTES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandaloneHttpServiceConfig {
    pub bind_address: String,
    pub worker_count: usize,
    pub queue_capacity: usize,
    pub max_body_bytes: usize,
    pub io_timeout_millis: u64,
}

impl Default for StandaloneHttpServiceConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8787".into(),
            worker_count: DEFAULT_HTTP_WORKERS,
            queue_capacity: DEFAULT_HTTP_QUEUE_CAPACITY,
            max_body_bytes: DEFAULT_MAX_HTTP_BODY_BYTES,
            io_timeout_millis: DEFAULT_IO_TIMEOUT_MILLIS,
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
pub struct StandaloneServiceStatsIR {
    pub schema: String,
    pub worker_count: usize,
    pub queue_capacity: usize,
    pub accepted_connections: usize,
    pub completed_requests: usize,
    pub rejected_connections: usize,
    pub failed_requests: usize,
    pub current_active_workers: usize,
    pub peak_active_workers: usize,
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
}

#[derive(Default)]
struct ServiceCounters {
    accepted_connections: AtomicUsize,
    completed_requests: AtomicUsize,
    rejected_connections: AtomicUsize,
    failed_requests: AtomicUsize,
    current_active_workers: AtomicUsize,
    peak_active_workers: AtomicUsize,
}

pub struct StandaloneHttpService {
    listener: TcpListener,
    runtime: Arc<ConcurrentConversationRuntime>,
    config: StandaloneHttpServiceConfig,
    counters: Arc<ServiceCounters>,
}

impl StandaloneHttpService {
    pub fn bind(
        config: StandaloneHttpServiceConfig,
        runtime: ConcurrentConversationRuntime,
    ) -> Result<Self, StandaloneServiceError> {
        config.validate()?;
        let mut addresses = config
            .bind_address
            .to_socket_addrs()
            .map_err(StandaloneServiceError::Bind)?;
        let address = addresses
            .next()
            .ok_or(StandaloneServiceError::InvalidConfiguration)?;
        let listener = TcpListener::bind(address).map_err(StandaloneServiceError::Bind)?;
        Ok(Self {
            listener,
            runtime: Arc::new(runtime),
            config,
            counters: Arc::new(ServiceCounters::default()),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, StandaloneServiceError> {
        self.listener
            .local_addr()
            .map_err(StandaloneServiceError::LocalAddress)
    }

    pub fn stats(&self) -> StandaloneServiceStatsIR {
        service_stats(&self.config, &self.counters)
    }

    pub fn serve(self) -> Result<(), StandaloneServiceError> {
        self.serve_inner(None).map(|_| ())
    }

    /// Bounded run used by a release canary. Every accepted connection counts,
    /// including malformed or rejected requests, so the host always terminates.
    pub fn serve_n_connections(
        self,
        connection_limit: usize,
    ) -> Result<StandaloneServiceRunReportIR, StandaloneServiceError> {
        if connection_limit == 0 {
            return Err(StandaloneServiceError::InvalidConfiguration);
        }
        self.serve_inner(Some(connection_limit))
    }

    fn serve_inner(
        self,
        connection_limit: Option<usize>,
    ) -> Result<StandaloneServiceRunReportIR, StandaloneServiceError> {
        let (sender, receiver) = mpsc::sync_channel::<TcpStream>(self.config.queue_capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(self.config.worker_count);

        for worker_index in 0..self.config.worker_count {
            let receiver = Arc::clone(&receiver);
            let runtime = Arc::clone(&self.runtime);
            let counters = Arc::clone(&self.counters);
            let config = self.config.clone();
            let worker = thread::Builder::new()
                .name(format!("b-core-http-{worker_index}"))
                .spawn(move || worker_loop(receiver, runtime, counters, config))
                .map_err(StandaloneServiceError::WorkerSpawn)?;
            workers.push(worker);
        }

        let mut accepted = 0usize;
        for incoming in self.listener.incoming() {
            let stream = match incoming {
                Ok(stream) => stream,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(StandaloneServiceError::Bind(error)),
            };
            accepted += 1;
            self.counters
                .accepted_connections
                .fetch_add(1, Ordering::AcqRel);
            match sender.try_send(stream) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(mut stream)) => {
                    self.counters
                        .rejected_connections
                        .fetch_add(1, Ordering::AcqRel);
                    let _ = write_json_response(
                        &mut stream,
                        503,
                        "Service Unavailable",
                        &json!({"ok": false, "error": "SERVER_BUSY"}),
                    );
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err(StandaloneServiceError::QueueDisconnected);
                }
            }
            if connection_limit.is_some_and(|limit| accepted >= limit) {
                break;
            }
        }
        drop(sender);

        for worker in workers {
            worker
                .join()
                .map_err(|_| StandaloneServiceError::WorkerPanicked)?;
        }

        Ok(StandaloneServiceRunReportIR {
            service: service_stats(&self.config, &self.counters),
            runtime: self.runtime.stats(),
        })
    }
}

fn worker_loop(
    receiver: Arc<Mutex<mpsc::Receiver<TcpStream>>>,
    runtime: Arc<ConcurrentConversationRuntime>,
    counters: Arc<ServiceCounters>,
    config: StandaloneHttpServiceConfig,
) {
    loop {
        let next = match receiver.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => return,
        };
        let Ok(mut stream) = next else {
            return;
        };
        let active = counters
            .current_active_workers
            .fetch_add(1, Ordering::AcqRel)
            + 1;
        update_peak(&counters.peak_active_workers, active);
        let _guard = ActiveWorkerGuard {
            counters: Arc::clone(&counters),
        };
        configure_stream(&stream, config.io_timeout_millis);
        let outcome = handle_connection(&mut stream, &runtime, &counters, &config);
        if !matches!(outcome, Ok(HttpHandlingOutcome::Success)) {
            counters.failed_requests.fetch_add(1, Ordering::AcqRel);
        }
        counters.completed_requests.fetch_add(1, Ordering::AcqRel);
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
    counters: &ServiceCounters,
    config: &StandaloneHttpServiceConfig,
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
                "service": service_stats(config, counters),
                "runtime": runtime.stats(),
            }),
        )
        .map(|_| HttpHandlingOutcome::Success),
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

fn service_stats(
    config: &StandaloneHttpServiceConfig,
    counters: &ServiceCounters,
) -> StandaloneServiceStatsIR {
    StandaloneServiceStatsIR {
        schema: STANDALONE_SERVICE_STATS_SCHEMA.into(),
        worker_count: config.worker_count,
        queue_capacity: config.queue_capacity,
        accepted_connections: counters.accepted_connections.load(Ordering::Acquire),
        completed_requests: counters.completed_requests.load(Ordering::Acquire),
        rejected_connections: counters.rejected_connections.load(Ordering::Acquire),
        failed_requests: counters.failed_requests.load(Ordering::Acquire),
        current_active_workers: counters.current_active_workers.load(Ordering::Acquire),
        peak_active_workers: counters.peak_active_workers.load(Ordering::Acquire),
    }
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
        ApprovedDiscourseRelationIR, ApprovedEventExpressionPreferenceIR, ApprovedEventPhaseIR,
        ApprovedEventPragmaticContextIR, ApprovedEventPredicateSenseIR,
        ApprovedEventRealizationClassIR, ApprovedEventRealizationIR, ApprovedLexicalNodeIR,
        ApprovedModalityIR, ApprovedOpenValueIR, ApprovedOperationIR, ApprovedRelationTypeIR,
        ApprovedResponseStyleIR, ApprovedSemanticTypeIR, ApprovedSpeechActIR, ApprovedVerbosityIR,
        APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA, COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
    };
    use crate::conversation::{ConversationInputModalityIR, CONVERSATION_TURN_REQUEST_SCHEMA};
    use crate::discourse_focus::DiscourseFocusCandidateIR;
    use crate::language_knowledge::LanguageRegisterIR;
    use std::sync::Barrier;
    use std::time::Instant;

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
