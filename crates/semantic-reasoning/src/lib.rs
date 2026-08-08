#![recursion_limit = "512"]

pub mod core_x0;
pub mod experiment;
pub mod mining;
pub mod reporting;
pub mod sem1;
pub mod sem10fresh;
pub mod sem11;
pub mod sem12;
pub mod sem13;
pub mod sem14;
pub mod sem15;
pub mod sem16;
pub mod sem17;
pub mod sem18;
pub mod sem19;
pub mod sem2;
pub mod sem20;
pub mod sem21;
pub mod sem22;
pub mod sem23;
pub mod sem24;
pub mod sem3;
pub mod sem4;
pub mod sem5;
pub mod sem6;
pub mod sem7;
pub mod sem8;
pub mod sem9;
pub mod sem9r1;
pub mod tasks;

pub use dockable_semantic_core::{dsl, reasoning, substrate};

pub use experiment::{run_sem0, Sem0Outcome};
pub use sem1::experiment::{run_sem1, Sem1Outcome};
pub use sem2::experiment::{run_sem2, Sem2Outcome};
pub use sem3::experiment::{run_sem3, Sem3Outcome};
pub use sem4::experiment::{run_sem4, Sem4Outcome};
pub use sem5::experiment::{run_sem5, Sem5Outcome};
pub use sem6::experiment::{run_sem6, Sem6Outcome};
pub use sem7::experiment::{run_sem7, Sem7Outcome};
pub use sem8::experiment::{run_sem8, Sem8Outcome};
pub use sem9::experiment::{run_sem9, Sem9Outcome};
pub use sem9r1::experiment::{run_sem9_r1, Sem9R1Outcome};
