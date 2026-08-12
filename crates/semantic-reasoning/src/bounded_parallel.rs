//! Deterministic bounded fan-out/fan-in for independent local work.
//!
//! Workers claim input ordinals dynamically for load balance. The join always
//! restores input order, so parallel execution cannot change scientific or
//! source-candidate selection semantics.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

pub(crate) fn map_ordered<T, R, F>(items: &[T], lane: &str, operation: F) -> Result<Vec<R>, String>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Result<R, String> + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(items.len())
        .max(1);
    if worker_count == 1 {
        return items.iter().map(operation).collect();
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(items.len()));
    thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            handles.push(scope.spawn(|| -> Result<(), String> {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(index) else {
                        break;
                    };
                    let result = operation(item);
                    results
                        .lock()
                        .map_err(|_| format!("{lane}_RESULT_LOCK_POISONED"))?
                        .push((index, result));
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| format!("{lane}_WORKER_PANICKED"))??;
        }
        Ok(())
    })?;
    let mut ordered = results
        .into_inner()
        .map_err(|_| format!("{lane}_RESULT_LOCK_POISONED"))?;
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, result)| result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_restores_input_order_and_propagates_counterexamples() {
        let inputs = (0_u64..64).collect::<Vec<_>>();
        let observed = map_ordered(&inputs, "PARALLEL_CANARY", |value| {
            Ok(value.saturating_mul(value.saturating_add(1)))
        })
        .unwrap();
        let expected = inputs
            .iter()
            .map(|value| value.saturating_mul(value.saturating_add(1)))
            .collect::<Vec<_>>();
        assert_eq!(observed, expected);

        let error = map_ordered(&inputs, "PARALLEL_CANARY", |value| {
            if *value == 17 {
                Err("BOUND_COUNTEREXAMPLE".to_string())
            } else {
                Ok(*value)
            }
        })
        .unwrap_err();
        assert_eq!(error, "BOUND_COUNTEREXAMPLE");
    }
}
