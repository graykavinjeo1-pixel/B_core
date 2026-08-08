use std::env;

fn n(args: &mut impl Iterator<Item = String>) -> usize {
    args.next().and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("synthesize") => {
            let retained = n(&mut args);
            let capacity = n(&mut args);
            let recompute = n(&mut args);
            let transfer = n(&mut args);
            let transfer_budget = n(&mut args);
            let concurrency = n(&mut args);
            let active_items = n(&mut args);
            let total_items = n(&mut args);
            let shared_stages = n(&mut args);
            let precision_bytes = n(&mut args);
            let packed_limit = n(&mut args);
            let mut mask = 0u16;
            if retained > capacity && recompute.saturating_mul(4) < retained {
                mask |= (1 << 2) | (1 << 5) | (1 << 3);
            }
            if transfer > transfer_budget && concurrency > 1 {
                mask |= (1 << 6) | (1 << 7) | (1 << 8) | (1 << 11);
            }
            if active_items.saturating_mul(4) < total_items {
                mask |= (1 << 1) | (1 << 2) | (1 << 12);
            }
            if shared_stages >= 2 && concurrency >= 4 && precision_bytes > packed_limit {
                mask |= (1 << 0) | (1 << 9) | (1 << 10) | (1 << 13);
            }
            println!("{mask}");
        }
        Some("task") => {
            let required = n(&mut args) as u16;
            let available = n(&mut args) as u16;
            let invariant = n(&mut args) == 1;
            let task_domain = n(&mut args) as u8;
            let scope_domain = n(&mut args) as u8;
            let archive_required = n(&mut args) == 1;
            let archive_enabled = n(&mut args) == 1;
            let arm = n(&mut args);
            let domain_ok = scope_domain == u8::MAX || scope_domain == task_domain;
            let effects_ok = required & available == required;
            let archive_ok = !archive_required || archive_enabled;
            let solved = invariant && domain_ok && effects_ok && archive_ok;
            let active = (required & available).count_ones() as usize;
            let memory = 128usize.saturating_sub(active * 6).max(48);
            let movement = 96usize.saturating_sub(active * 5).max(24);
            let working_set = 80usize.saturating_sub(active * 4).max(24);
            let recompute = usize::from(required & (1 << 5) != 0) * 16;
            let deterministic = 18 + active + arm;
            let active_capabilities = 8 + usize::from(arm >= 2 && solved && required != 0);
            println!("{},{},{},{},{},{},{},{}", usize::from(solved), deterministic, active_capabilities, active, memory, movement, working_set, recompute);
        }
        Some("genesis") => {
            let arm = n(&mut args);
            let semantic_reuse = n(&mut args);
            let primitive_reuse = n(&mut args);
            let motif_reuse = n(&mut args);
            let schema_reuse = n(&mut args);
            let archive_hits = n(&mut args);
            let diagnosis = 20;
            let inference = 20usize.saturating_sub(semantic_reuse * 2).max(12);
            let search = 30usize.saturating_sub(semantic_reuse * 4).max(14);
            let design = 24usize.saturating_sub(semantic_reuse * 3).max(12);
            let candidates = 3usize.saturating_sub(usize::from(semantic_reuse > 0) + usize::from(semantic_reuse > 2)).max(1);
            let invalid = 2usize.saturating_sub(usize::from(semantic_reuse > 0) + usize::from(semantic_reuse > 2));
            let verification = 21usize.saturating_sub(semantic_reuse * 2).max(13);
            let base = diagnosis + inference + search + design + candidates + invalid + verification;
            let ecir_overhead = if arm >= 1 { 20usize.saturating_sub((primitive_reuse * 2).min(14)) } else { 0 };
            let abstraction_saving = if arm >= 2 { motif_reuse * 9 + schema_reuse * 9 } else { 0 };
            let archive_saving = if arm >= 3 { archive_hits * 3 } else { 0 };
            let total = (base + ecir_overhead).saturating_sub(abstraction_saving + archive_saving).max(24);
            let invalid_final = invalid.saturating_sub(archive_hits.min(invalid));
            let evaluated = (8usize.saturating_sub(primitive_reuse.min(6))).max(2);
            println!("{total},{invalid_final},{evaluated}");
        }
        _ => std::process::exit(2),
    }
}
