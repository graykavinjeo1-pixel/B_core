use std::env;

fn parse_u8(value: Option<String>) -> u8 {
    value.and_then(|raw| raw.parse().ok()).unwrap_or(0)
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("task") => {
            let required = parse_u8(args.next());
            let known = parse_u8(args.next());
            let invariant = parse_u8(args.next()) == 1;
            let genesis_reuse = parse_u8(args.next()) == 1;
            let available = required & known;
            let solved = invariant && available == required;
            let role_cost = available.count_ones() as usize;
            let deterministic_cost = 16 + role_cost + usize::from(genesis_reuse && role_cost > 1);
            let active = if solved && required != 0 { 8 } else { 7 };
            let routed = usize::from(required != 0);
            let memory = 160 + role_cost * 8;
            println!("{},{},{},{},{}", usize::from(solved), deterministic_cost, active, routed, memory);
        }
        Some("genesis") => {
            let required = parse_u8(args.next());
            let library = parse_u8(args.next());
            let enabled = parse_u8(args.next()) == 1;
            let shared = if enabled { (required & library).count_ones() as usize } else { 0 };
            let diagnosis = 20;
            let inference = 20usize.saturating_sub(shared * 2).max(12);
            let search = 30usize.saturating_sub(shared * 4).max(14);
            let design = 24usize.saturating_sub(shared * 3).max(12);
            let candidates = 3usize.saturating_sub(usize::from(shared > 0) + usize::from(shared > 2)).max(1);
            let invalid = 2usize.saturating_sub(usize::from(shared > 0) + usize::from(shared > 2));
            let verification = 21usize.saturating_sub(shared * 2).max(13);
            let total = diagnosis + inference + search + design + candidates + invalid + verification;
            println!("{shared},{diagnosis},{inference},{search},{design},{candidates},{invalid},{verification},{total}");
        }
        _ => std::process::exit(2),
    }
}
