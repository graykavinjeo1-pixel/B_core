#[derive(Debug, Clone)]
pub struct DomainRouter;

impl DomainRouter {
    pub fn route(stimulus: &str) -> String {
        let finance = [
            "삼성전자",
            "주가",
            "하락",
            "외국인",
            "반대매매",
            "수급",
            "finance",
            "market",
        ];
        if finance.iter().any(|marker| stimulus.contains(marker)) {
            "finance".to_string()
        } else if stimulus.contains("뜨거움") || stimulus.contains("emotion") {
            "emotion".to_string()
        } else {
            "daily".to_string()
        }
    }
}
