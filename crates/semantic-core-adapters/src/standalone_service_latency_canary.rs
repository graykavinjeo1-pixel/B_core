use std::{
    env,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use semantic_core_adapters::{
    ConcurrentConversationRuntime, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, LanguageCodeIR, StandaloneHttpService, StandaloneHttpServiceConfig,
    CONVERSATION_TURN_REQUEST_SCHEMA, DEFAULT_HTTP_QUEUE_CAPACITY, DEFAULT_HTTP_WORKERS,
    MAX_HTTP_WORKERS,
};
use serde::Serialize;

const DEFAULT_SESSIONS: usize = 64;
const MAX_SESSIONS: usize = 256;
const DEFAULT_WAVES: usize = 1;
const MAX_WAVES: usize = 32;
const DEFAULT_TEXT: &str = "회의 시간이 오후 4시야.";
const MAX_INPUT_BYTES: usize = 16 * 1024;

#[derive(Serialize)]
struct WaveReport {
    wave: usize,
    sessions: usize,
    completed: usize,
    failed: usize,
    completed_turns: usize,
    failed_turns: usize,
    closed_conversations: usize,
    failed_conversation_closes: usize,
    active_conversations_after_close: Option<usize>,
    latency_p95_ms: f64,
    latency_max_ms: f64,
    latency_p95_budget_passed: bool,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    status: &'static str,
    sessions_started_together: usize,
    waves: usize,
    sessions_total: usize,
    input_bytes: usize,
    follow_up_input_bytes: Option<usize>,
    turns_per_session: usize,
    completed: usize,
    failed: usize,
    completed_turns: usize,
    failed_turns: usize,
    closed_conversations: usize,
    failed_conversation_closes: usize,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_max_ms: f64,
    latency_p95_budget_ms: Option<f64>,
    latency_p95_budget_passed: bool,
    wave_latency_p95_budget_passed: bool,
    max_wave_latency_p95_ms: f64,
    wave_reports: Vec<WaveReport>,
    throughput_requests_per_second: f64,
    workers: usize,
    queue_capacity: usize,
    peak_pending_connections: usize,
    average_queue_wait_micros: u64,
    p95_queue_wait_micros_upper_bound: u64,
    max_queue_wait_micros: u64,
    average_handler_micros: u64,
    p95_handler_micros_upper_bound: u64,
    max_handler_micros: u64,
    rejected_connections: usize,
    runtime_busy_rejections: usize,
    active_conversations_after_close: usize,
    gpu_used: bool,
}

fn main() {
    let sessions =
        positive_usize_env("B_CORE_HTTP_CANARY_SESSIONS", DEFAULT_SESSIONS).min(MAX_SESSIONS);
    let waves = positive_usize_env("B_CORE_HTTP_CANARY_WAVES", DEFAULT_WAVES).min(MAX_WAVES);
    let workers = positive_usize_env("B_CORE_HTTP_CANARY_WORKERS", DEFAULT_HTTP_WORKERS)
        .min(MAX_HTTP_WORKERS);
    let text = Arc::<str>::from(canary_text());
    let follow_up_text = canary_follow_up_text().map(Arc::<str>::from);
    let turns_per_session = if follow_up_text.is_some() { 2 } else { 1 };
    let latency_p95_budget_ms = positive_f64_env("B_CORE_HTTP_CANARY_MAX_P95_MS");
    let runtime =
        ConcurrentConversationRuntime::standalone_default().expect("standalone concurrent runtime");
    let service = Arc::new(
        StandaloneHttpService::bind(
            StandaloneHttpServiceConfig {
                bind_address: "127.0.0.1:0".into(),
                worker_count: workers,
                queue_capacity: DEFAULT_HTTP_QUEUE_CAPACITY,
                ..StandaloneHttpServiceConfig::default()
            },
            runtime,
        )
        .expect("standalone HTTP service"),
    );
    let address = service.local_addr().expect("bound address");
    let server = {
        let service = Arc::clone(&service);
        thread::spawn(move || {
            service
                .serve_n_connections(sessions * waves * (turns_per_session + 1) + waves)
                .expect("bounded HTTP serving")
        })
    };
    let wall_started = Instant::now();
    let mut client_results = Vec::with_capacity(sessions * waves);
    let mut wave_reports = Vec::with_capacity(waves);
    for wave in 0..waves {
        let barrier = Arc::new(Barrier::new(sessions + 1));
        let follow_up_connect_barrier = follow_up_text
            .as_ref()
            .map(|_| Arc::new(Barrier::new(sessions + 1)));
        let follow_up_send_barrier = follow_up_text
            .as_ref()
            .map(|_| Arc::new(Barrier::new(sessions + 1)));
        let clients = (0..sessions)
            .map(|session| {
                let barrier = Arc::clone(&barrier);
                let text = Arc::clone(&text);
                let follow_up_text = follow_up_text.as_ref().map(Arc::clone);
                let follow_up_connect_barrier = follow_up_connect_barrier.as_ref().map(Arc::clone);
                let follow_up_send_barrier = follow_up_send_barrier.as_ref().map(Arc::clone);
                thread::spawn(move || {
                    let mut stream = connect(address);
                    barrier.wait();
                    let mut results = vec![send_turn(stream, request(wave, session, 1, &text))];
                    if let (Some(follow_up_text), Some(connect_barrier), Some(send_barrier)) = (
                        follow_up_text,
                        follow_up_connect_barrier,
                        follow_up_send_barrier,
                    ) {
                        connect_barrier.wait();
                        stream = connect(address);
                        send_barrier.wait();
                        results.push(send_turn(
                            stream,
                            request(wave, session, 2, &follow_up_text),
                        ));
                    }
                    let closed = close_conversation(address, wave, session);
                    (results, closed)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        if let Some(connect_barrier) = &follow_up_connect_barrier {
            connect_barrier.wait();
        }
        if let Some(send_barrier) = &follow_up_send_barrier {
            send_barrier.wait();
        }
        let wave_client_results = clients
            .into_iter()
            .map(|client| client.join().expect("HTTP client"))
            .collect::<Vec<_>>();
        let wave_closed_conversations = wave_client_results
            .iter()
            .filter(|(_, closed)| *closed)
            .count();
        let mut wave_results = wave_client_results
            .iter()
            .flat_map(|(turns, _)| turns.iter().copied())
            .collect::<Vec<_>>();
        let wave_completed_turns = wave_results.iter().filter(|(ok, _)| *ok).count();
        let wave_completed = wave_results
            .chunks(turns_per_session)
            .filter(|turns| turns.iter().all(|(ok, _)| *ok))
            .count();
        wave_results.sort_by(|left, right| left.1.cmp(&right.1));
        let wave_latency_p95_ms = percentile(&wave_results, 0.95);
        let wave_latency_p95_budget_passed = latency_p95_budget_ms
            .is_none_or(|budget| wave_latency_p95_ms <= budget);
        let active_conversations_after_close = active_conversations(address);
        wave_reports.push(WaveReport {
            wave,
            sessions,
            completed: wave_completed,
            failed: sessions - wave_completed,
            completed_turns: wave_completed_turns,
            failed_turns: wave_results.len() - wave_completed_turns,
            closed_conversations: wave_closed_conversations,
            failed_conversation_closes: sessions - wave_closed_conversations,
            active_conversations_after_close,
            latency_p95_ms: wave_latency_p95_ms,
            latency_max_ms: wave_results
                .last()
                .map(|(_, latency)| latency.as_secs_f64() * 1_000.0)
                .unwrap_or_default(),
            latency_p95_budget_passed: wave_latency_p95_budget_passed,
        });
        client_results.extend(wave_client_results);
    }
    let total_sessions = sessions * waves;
    let closed_conversations = client_results.iter().filter(|(_, closed)| *closed).count();
    let mut results = client_results
        .into_iter()
        .flat_map(|(turns, _)| turns)
        .collect::<Vec<_>>();
    let wall_seconds = wall_started.elapsed().as_secs_f64();
    let run_report = server.join().expect("HTTP server");
    let completed_turns = results.iter().filter(|(ok, _)| *ok).count();
    let completed = results
        .chunks(turns_per_session)
        .filter(|turns| turns.iter().all(|(ok, _)| *ok))
        .count();
    results.sort_by(|left, right| left.1.cmp(&right.1));
    let latency_p50_ms = percentile(&results, 0.50);
    let latency_p95_ms = percentile(&results, 0.95);
    let latency_p95_budget_passed =
        latency_p95_budget_ms.is_none_or(|budget| latency_p95_ms <= budget);
    let report = Report {
        schema: "B_CORE_STANDALONE_HTTP_LATENCY_CANARY_5",
        status: if completed == total_sessions
            && closed_conversations == total_sessions
            && run_report.service.rejected_connections == 0
            && run_report.runtime.rejected_busy_requests == 0
            && run_report.runtime.active_conversations == 0
            && latency_p95_budget_passed
            && wave_reports.iter().all(|wave| {
                wave.completed == sessions
                    && wave.closed_conversations == sessions
                    && wave.active_conversations_after_close == Some(0)
                    && wave.latency_p95_budget_passed
            })
        {
            "PASS"
        } else {
            "FAIL"
        },
        sessions_started_together: sessions,
        waves,
        sessions_total: total_sessions,
        input_bytes: text.len(),
        follow_up_input_bytes: follow_up_text.as_ref().map(|text| text.len()),
        turns_per_session,
        completed,
        failed: total_sessions - completed,
        completed_turns,
        failed_turns: results.len() - completed_turns,
        closed_conversations,
        failed_conversation_closes: total_sessions - closed_conversations,
        latency_p50_ms,
        latency_p95_ms,
        latency_max_ms: results
            .last()
            .map(|(_, latency)| latency.as_secs_f64() * 1_000.0)
            .unwrap_or_default(),
        latency_p95_budget_ms,
        latency_p95_budget_passed,
        wave_latency_p95_budget_passed: wave_reports
            .iter()
            .all(|wave| wave.latency_p95_budget_passed),
        max_wave_latency_p95_ms: wave_reports
            .iter()
            .map(|wave| wave.latency_p95_ms)
            .fold(0.0, f64::max),
        wave_reports,
        throughput_requests_per_second: completed_turns as f64 / wall_seconds,
        workers: run_report.service.worker_count,
        queue_capacity: run_report.service.queue_capacity,
        peak_pending_connections: run_report.service.peak_pending_connections,
        average_queue_wait_micros: run_report.service.average_queue_wait_micros,
        p95_queue_wait_micros_upper_bound: run_report
            .service
            .p95_queue_wait_micros_upper_bound,
        max_queue_wait_micros: run_report.service.max_queue_wait_micros,
        average_handler_micros: run_report.service.average_handler_micros,
        p95_handler_micros_upper_bound: run_report
            .service
            .p95_handler_micros_upper_bound,
        max_handler_micros: run_report.service.max_handler_micros,
        rejected_connections: run_report.service.rejected_connections,
        runtime_busy_rejections: run_report.runtime.rejected_busy_requests,
        active_conversations_after_close: run_report.runtime.active_conversations,
        gpu_used: false,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("canary report")
    );
    if report.status != "PASS" {
        std::process::exit(1);
    }
}

fn connect(address: SocketAddr) -> TcpStream {
    let stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).expect("connect service");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .expect("read timeout");
    stream
}

fn send_turn(mut stream: TcpStream, request: ConversationTurnRequestIR) -> (bool, Duration) {
    let body = serde_json::to_vec(&request).expect("request JSON");
    let wire = format!(
        "POST /v1/conversation/turn HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len(),
        address = stream.peer_addr().expect("connected peer")
    );
    let started = Instant::now();
    stream.write_all(wire.as_bytes()).expect("request header");
    stream.write_all(&body).expect("request body");
    stream.shutdown(Shutdown::Write).expect("request complete");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("response body");
    let parsed = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .and_then(|body_start| {
            serde_json::from_slice::<ConversationTurnResponseIR>(&response[body_start + 4..]).ok()
        });
    (
        response.starts_with(b"HTTP/1.1 200 OK")
            && parsed.is_some_and(|response| response.validate_against(&request)),
        started.elapsed(),
    )
}

fn close_conversation(address: SocketAddr, wave: usize, session: usize) -> bool {
    let body = serde_json::to_vec(&serde_json::json!({
        "conversation_id": conversation_id(wave, session),
    }))
    .expect("close request JSON");
    let wire = format!(
        "POST /v1/conversation/close HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut stream = connect(address);
    stream.write_all(wire.as_bytes()).expect("close request header");
    stream.write_all(&body).expect("close request body");
    stream.shutdown(Shutdown::Write).expect("close request complete");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("close response body");
    response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .and_then(|body_start| {
            serde_json::from_slice::<serde_json::Value>(&response[body_start + 4..]).ok()
        })
        .is_some_and(|body| {
            body.get("ok").and_then(serde_json::Value::as_bool) == Some(true)
                && body.get("closed").and_then(serde_json::Value::as_bool) == Some(true)
        })
}

fn active_conversations(address: SocketAddr) -> Option<usize> {
    let mut stream = connect(address);
    let wire = format!(
        "GET /v1/stats HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(wire.as_bytes()).ok()?;
    stream.shutdown(Shutdown::Write).ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;
    let body_start = response.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let body = serde_json::from_slice::<serde_json::Value>(&response[body_start..]).ok()?;
    body.get("runtime")?
        .get("active_conversations")?
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
}
fn request(wave: usize, session: usize, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.into(),
        conversation_id: conversation_id(wave, session),
        turn_index,
        request_id: format!("HTTP-LATENCY-REQUEST-{wave:02}-{session:03}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.into(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::Korean),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn conversation_id(wave: usize, session: usize) -> String {
    format!("HTTP-LATENCY-WAVE-{wave:02}-SESSION-{session:03}")
}
fn canary_text() -> String {
    env::var("B_CORE_HTTP_CANARY_TEXT")
        .ok()
        .filter(|text| !text.trim().is_empty() && text.len() <= MAX_INPUT_BYTES)
        .unwrap_or_else(|| DEFAULT_TEXT.into())
}

fn canary_follow_up_text() -> Option<String> {
    env::var("B_CORE_HTTP_CANARY_FOLLOW_UP_TEXT")
        .ok()
        .filter(|text| !text.trim().is_empty() && text.len() <= MAX_INPUT_BYTES)
}


fn positive_usize_env(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn positive_f64_env(name: &str) -> Option<f64> {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn percentile(results: &[(bool, Duration)], quantile: f64) -> f64 {
    if results.is_empty() {
        return 0.0;
    }
    let index = ((results.len() - 1) as f64 * quantile).round() as usize;
    results[index].1.as_secs_f64() * 1_000.0
}
