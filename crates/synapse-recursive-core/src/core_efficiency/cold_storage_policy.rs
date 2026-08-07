use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColdStorageReport {
    pub resident_items_before: usize,
    pub resident_items_after: usize,
    pub retained_summaries: Vec<String>,
    pub moved_to_cold_storage: Vec<String>,
    pub raw_logs_resident: bool,
}

pub struct ColdStoragePolicy;

impl ColdStoragePolicy {
    pub fn compress_development_artifacts() -> ColdStorageReport {
        ColdStorageReport {
            resident_items_before: 8,
            resident_items_after: 3,
            retained_summaries: vec![
                "CodingLesson summary".to_string(),
                "DevelopmentMemory summary".to_string(),
                "Risk memory".to_string(),
            ],
            moved_to_cold_storage: vec![
                "large raw logs".to_string(),
                "full patch logs".to_string(),
                "full test output".to_string(),
                "temporary sandbox state".to_string(),
                "large benchmark raw log".to_string(),
            ],
            raw_logs_resident: false,
        }
    }
}
