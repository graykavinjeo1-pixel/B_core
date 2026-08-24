#![recursion_limit = "512"]

pub mod autonomous_self_inspection;
pub mod autonomous_source_mutation;
pub mod benchmark_capability_canary;
mod bounded_parallel;
pub mod code_graft;
#[cfg(feature = "historical-campaigns")]
pub mod code_graft_acceptance;
pub mod compiler_guided_repair;
pub mod compound_growth;
pub mod compound_typed_goal;
#[cfg(feature = "historical-campaigns")]
pub mod core_x0;
pub mod cross_language_synthesis;
#[cfg(feature = "historical-campaigns")]
pub mod experiment;
pub mod fullstack_ops_knowledge;
pub mod generalized_self_application;
pub mod generated_sem5_capability;
pub mod generative_growth;
pub mod grammar_repair_synthesis;
pub mod growth_supervisor;
pub mod integrated_development;
pub mod intrinsic_drive;
pub mod meta_compiler_expansion;
#[cfg(feature = "historical-campaigns")]
pub mod mining;
pub mod north_star;
#[cfg(feature = "historical-campaigns")]
pub mod reporting;
pub mod repository_experience;
pub mod repository_change_experience;
pub mod repository_coding_knowledge;
pub mod repository_horizon;
pub mod repository_issue_understanding;
pub mod repository_requirement_graph;
pub mod same_attempt_revision;
pub mod self_healing_execution;
pub mod self_healing_pipeline;
pub mod self_repair_contract;
#[cfg(feature = "historical-campaigns")]
pub mod sem1;
#[cfg(feature = "historical-campaigns")]
pub mod sem10fresh;
#[cfg(feature = "historical-campaigns")]
pub mod sem11;
#[cfg(feature = "historical-campaigns")]
pub mod sem12;
#[cfg(feature = "historical-campaigns")]
pub mod sem13;
#[cfg(feature = "historical-campaigns")]
pub mod sem14;
#[cfg(feature = "historical-campaigns")]
pub mod sem15;
#[cfg(feature = "historical-campaigns")]
pub mod sem16;
#[cfg(feature = "historical-campaigns")]
pub mod sem17;
#[cfg(feature = "historical-campaigns")]
pub mod sem18;
#[cfg(feature = "historical-campaigns")]
pub mod sem19;
#[cfg(feature = "historical-campaigns")]
pub mod sem2;
#[cfg(feature = "historical-campaigns")]
pub mod sem20;
#[cfg(feature = "historical-campaigns")]
pub mod sem21;
#[cfg(feature = "historical-campaigns")]
pub mod sem22;
#[cfg(feature = "historical-campaigns")]
pub mod sem23;
#[cfg(feature = "historical-campaigns")]
pub mod sem24;
#[cfg(feature = "historical-campaigns")]
pub mod sem25;
#[cfg(feature = "historical-campaigns")]
pub mod sem26;
#[cfg(feature = "historical-campaigns")]
pub mod sem27;
#[cfg(feature = "historical-campaigns")]
pub mod sem27_r1;
#[cfg(feature = "historical-campaigns")]
pub mod sem28;
#[cfg(feature = "historical-campaigns")]
pub mod sem29;
#[cfg(feature = "historical-campaigns")]
pub mod sem3;
#[cfg(feature = "historical-campaigns")]
pub mod sem30;
#[cfg(feature = "historical-campaigns")]
pub mod sem31;
#[cfg(feature = "historical-campaigns")]
pub mod sem32;
#[cfg(feature = "historical-campaigns")]
pub mod sem32_r1;
#[cfg(feature = "historical-campaigns")]
pub mod sem33_r1;
#[cfg(feature = "historical-campaigns")]
pub mod sem34;
#[cfg(feature = "historical-campaigns")]
pub mod sem35;
#[cfg(feature = "historical-campaigns")]
pub mod sem35_r1;
#[cfg(feature = "historical-campaigns")]
pub mod sem36;
#[cfg(feature = "historical-campaigns")]
pub mod sem4;
pub mod sem5;
#[cfg(feature = "historical-campaigns")]
pub mod sem6;
#[cfg(feature = "historical-campaigns")]
pub mod sem7;
#[cfg(feature = "historical-campaigns")]
pub mod sem8;
#[cfg(feature = "historical-campaigns")]
pub mod sem9;
#[cfg(feature = "historical-campaigns")]
pub mod sem9r1;
pub mod source_bound_causal_frontend;
mod source_proposal_kernel;
pub mod structural_source_repair;
#[cfg(feature = "historical-campaigns")]
pub mod tasks;

// The active supervisor needs only the executable engine lineage, not the
// report/build machinery embedded in each sealed historical campaign module.
#[cfg(feature = "runtime-core")]
#[path = "sem20/engine.rs"]
pub mod sem20_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem21/engine.rs"]
pub mod sem21_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem22/engine.rs"]
pub mod sem22_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem23/engine.rs"]
pub mod sem23_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem24/engine.rs"]
pub mod sem24_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem25/engine.rs"]
pub mod sem25_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem26/engine.rs"]
pub mod sem26_engine;
#[cfg(feature = "runtime-core")]
#[path = "sem27/engine.rs"]
pub mod sem27_engine;

#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem20::engine as sem20_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem21::engine as sem21_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem22::engine as sem22_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem23::engine as sem23_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem24::engine as sem24_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem25::engine as sem25_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem26::engine as sem26_engine;
#[cfg(all(not(feature = "runtime-core"), feature = "historical-campaigns"))]
pub use sem27::engine as sem27_engine;

pub use dockable_semantic_core::{dsl, reasoning, substrate};

#[cfg(feature = "historical-campaigns")]
pub use experiment::{run_sem0, Sem0Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem1::experiment::{run_sem1, Sem1Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem2::experiment::{run_sem2, Sem2Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem3::experiment::{run_sem3, Sem3Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem4::experiment::{run_sem4, Sem4Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem5::experiment::{run_sem5, Sem5Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem6::experiment::{run_sem6, Sem6Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem7::experiment::{run_sem7, Sem7Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem8::experiment::{run_sem8, Sem8Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem9::experiment::{run_sem9, Sem9Outcome};
#[cfg(feature = "historical-campaigns")]
pub use sem9r1::experiment::{run_sem9_r1, Sem9R1Outcome};
