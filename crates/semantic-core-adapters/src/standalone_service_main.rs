use std::env;
use std::process::ExitCode;

use semantic_core_adapters::{
    ConcurrentConversationRuntime, StandaloneHttpService, StandaloneHttpServiceConfig,
    DEFAULT_CONVERSATION_SHARDS, DEFAULT_HTTP_QUEUE_CAPACITY, DEFAULT_HTTP_WORKERS,
    DEFAULT_MAX_HTTP_BODY_BYTES, DEFAULT_MAX_IN_FLIGHT,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("B_CORE_STANDALONE_SERVICE_ERROR={error:?}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), semantic_core_adapters::StandaloneServiceError> {
    let shard_count = env_usize("B_CORE_CONVERSATION_SHARDS", DEFAULT_CONVERSATION_SHARDS);
    let max_in_flight = env_usize("B_CORE_MAX_IN_FLIGHT", DEFAULT_MAX_IN_FLIGHT);
    let runtime = ConcurrentConversationRuntime::new(shard_count, max_in_flight)
        .map_err(semantic_core_adapters::StandaloneServiceError::Runtime)?;
    let config = StandaloneHttpServiceConfig {
        bind_address: env::var("B_CORE_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8787".into()),
        worker_count: env_usize("B_CORE_HTTP_WORKERS", DEFAULT_HTTP_WORKERS),
        queue_capacity: env_usize("B_CORE_HTTP_QUEUE_CAPACITY", DEFAULT_HTTP_QUEUE_CAPACITY),
        max_body_bytes: env_usize("B_CORE_MAX_HTTP_BODY_BYTES", DEFAULT_MAX_HTTP_BODY_BYTES),
        construction_store_path: env::var("B_CORE_CONSTRUCTION_STORE_PATH")
            .ok()
            .filter(|path| !path.trim().is_empty()),
        persona_contrastive_store_path: env::var("B_CORE_PERSONA_CONTRASTIVE_STORE_PATH")
            .ok()
            .filter(|path| !path.trim().is_empty()),
        persona_latent_operator_store_path: env::var("B_CORE_PERSONA_LATENT_OPERATOR_STORE_PATH")
            .ok()
            .filter(|path| !path.trim().is_empty()),
        language_state_controller_store_path: env::var(
            "B_CORE_LANGUAGE_STATE_CONTROLLER_STORE_PATH",
        )
        .ok()
        .filter(|path| !path.trim().is_empty()),
        ..StandaloneHttpServiceConfig::default()
    };
    let service = StandaloneHttpService::bind(config, runtime)?;
    let address = service.local_addr()?;
    eprintln!("B_CORE_STANDALONE_SERVICE=READY");
    eprintln!("B_CORE_BIND_ADDRESS={address}");
    eprintln!("B_CORE_GPU_USED=false");
    service.serve()
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
