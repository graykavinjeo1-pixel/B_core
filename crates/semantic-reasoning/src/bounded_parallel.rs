//! Deterministic bounded fan-out/fan-in for independent local work.
//!
//! Workers claim input ordinals dynamically for load balance. The join always
//! restores input order, so parallel execution cannot change scientific or
//! source-candidate selection semantics.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

thread_local! {
    static PARALLEL_WORKER_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn worker_count_for(item_count: usize, minimum_items_per_worker: usize) -> usize {
    if item_count == 0 {
        return 0;
    }
    if PARALLEL_WORKER_DEPTH.with(|depth| depth.get() > 0) {
        return 1;
    }
    let minimum_items_per_worker = minimum_items_per_worker.max(1);
    let useful_workers = item_count.div_ceil(minimum_items_per_worker);
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(useful_workers)
        .min(item_count)
        .max(1)
}

pub(crate) fn map_ordered<T, R, F>(items: &[T], lane: &str, operation: F) -> Result<Vec<R>, String>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Result<R, String> + Sync,
{
    map_ordered_batched(items, lane, 1, operation)
}

pub(crate) fn map_ordered_batched<T, R, F>(
    items: &[T],
    lane: &str,
    minimum_items_per_worker: usize,
    operation: F,
) -> Result<Vec<R>, String>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Result<R, String> + Sync,
{
    map_ordered_batched_by(items, lane, minimum_items_per_worker, operation, |error| {
        error
    })
}

pub(crate) fn map_ordered_batched_by<T, R, E, F, G>(
    items: &[T],
    lane: &str,
    minimum_items_per_worker: usize,
    operation: F,
    error_from_string: G,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
    F: Fn(&T) -> Result<R, E> + Sync,
    G: Fn(String) -> E + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = worker_count_for(items.len(), minimum_items_per_worker);
    if worker_count == 1 {
        return items.iter().map(operation).collect();
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(items.len()));
    thread::scope(|scope| -> Result<(), E> {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            handles.push(scope.spawn(|| -> Result<(), E> {
                PARALLEL_WORKER_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(index) else {
                        break;
                    };
                    let result = operation(item);
                    results
                        .lock()
                        .map_err(|_| error_from_string(format!("{lane}_RESULT_LOCK_POISONED")))?
                        .push((index, result));
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| error_from_string(format!("{lane}_WORKER_PANICKED")))??;
        }
        Ok(())
    })?;
    let mut ordered = results
        .into_inner()
        .map_err(|_| error_from_string(format!("{lane}_RESULT_LOCK_POISONED")))?;
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
            if *value == 7 {
                Err("FIRST_BOUND_COUNTEREXAMPLE".to_string())
            } else if *value == 17 {
                Err("LATER_BOUND_COUNTEREXAMPLE".to_string())
            } else {
                Ok(*value)
            }
        })
        .unwrap_err();
        assert_eq!(error, "FIRST_BOUND_COUNTEREXAMPLE");
    }

    #[test]
    fn batching_avoids_tiny_fanout_and_bounds_lightweight_workers() {
        assert_eq!(worker_count_for(0, 16), 0);
        assert_eq!(worker_count_for(2, 16), 1);
        assert!(worker_count_for(256, 16) <= 16);

        let caller = thread::current().id();
        let worker_ids = Mutex::new(Vec::new());
        let observed = map_ordered_batched(&[3_u64, 5], "LIGHT_CANARY", 16, |value| {
            worker_ids.lock().unwrap().push(thread::current().id());
            Ok(value.saturating_mul(2))
        })
        .unwrap();
        assert_eq!(observed, [6, 10]);
        assert!(worker_ids
            .into_inner()
            .unwrap()
            .into_iter()
            .all(|worker| worker == caller));
    }

    #[test]
    fn nested_parallel_graph_keeps_fanout_at_the_coarse_level() {
        let nested_worker_counts = Mutex::new(Vec::new());
        let inputs = (0_u64..64).collect::<Vec<_>>();
        let observed = map_ordered(&inputs, "OUTER_GRAPH_CANARY", |value| {
            nested_worker_counts
                .lock()
                .unwrap()
                .push(worker_count_for(256, 1));
            Ok(value.saturating_add(1))
        })
        .unwrap();
        assert_eq!(observed.len(), inputs.len());
        assert!(nested_worker_counts
            .into_inner()
            .unwrap()
            .into_iter()
            .all(|workers| workers == 1));
    }
}
