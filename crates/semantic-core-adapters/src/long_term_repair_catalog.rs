use crate::long_term_repair::{RepairMethodIR, StatutoryRepairItemIR, StatutoryRepairMethodIR};

macro_rules! item {
    ($id:expr, $group:expr, $subgroup:expr, $work:expr, [$(($method:ident, $cycle:expr, $rate:expr)),+ $(,)?]) => {
        StatutoryRepairItemIR {
            id: $id.to_string(),
            group: $group.to_string(),
            subgroup: $subgroup.to_string(),
            work_type: $work.to_string(),
            methods: vec![$(StatutoryRepairMethodIR {
                method: RepairMethodIR::$method,
                cycle_years: $cycle,
                repair_rate_percent: $rate,
            }),+],
            notes: Vec::new(),
        }
    };
}

pub(crate) fn statutory_catalog() -> Vec<StatutoryRepairItemIR> {
    use RepairMethodIR::{
        FullCoating, FullRepair, FullReplacement, PartialRepair, PartialReplacement,
    };
    let _ = (
        FullCoating,
        FullRepair,
        FullReplacement,
        PartialRepair,
        PartialReplacement,
    );
    let mut catalog = vec![
        item!(
            "1-가-1",
            "1. 건물외부",
            "가. 지붕",
            "방수",
            [(FullRepair, 15, 100)]
        ),
        item!(
            "1-가-2",
            "1. 건물외부",
            "가. 지붕",
            "금속기와 잇기",
            [(PartialRepair, 5, 10), (FullReplacement, 20, 100)]
        ),
        item!(
            "1-가-3",
            "1. 건물외부",
            "가. 지붕",
            "아스팔트싱글 잇기",
            [(PartialRepair, 5, 10), (FullReplacement, 20, 100)]
        ),
        item!(
            "1-나-1",
            "1. 건물외부",
            "나. 외부",
            "돌 붙이기",
            [(PartialRepair, 25, 5)]
        ),
        item!(
            "1-나-2",
            "1. 건물외부",
            "나. 외부",
            "페인트칠",
            [(FullCoating, 8, 100)]
        ),
        item!(
            "1-다-1",
            "1. 건물외부",
            "다. 외부 창·문",
            "출입문(자동문)",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "2-가-1",
            "2. 건물내부",
            "가. 내부",
            "페인트칠",
            [(FullCoating, 8, 100)]
        ),
        item!(
            "2-나-1",
            "2. 건물내부",
            "나. 바닥",
            "지하주차장(바닥)",
            [(PartialRepair, 5, 10), (FullReplacement, 15, 100)]
        ),
        item!(
            "3-가-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "가. 예비전원(자가발전) 설비",
            "발전기",
            [(PartialRepair, 10, 10), (FullReplacement, 30, 100)]
        ),
        item!(
            "3-가-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "가. 예비전원(자가발전) 설비",
            "배전반",
            [(PartialReplacement, 10, 10), (FullReplacement, 20, 100)]
        ),
        item!(
            "3-나-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "나. 변전설비",
            "변압기",
            [(FullReplacement, 25, 100)]
        ),
        item!(
            "3-나-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "나. 변전설비",
            "수전반",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-나-3",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "나. 변전설비",
            "배전반",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-다-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "다. 자동화재감지설비",
            "감지기",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-다-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "다. 자동화재감지설비",
            "수신반",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-라-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "라. 소화설비",
            "소화펌프",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-라-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "라. 소화설비",
            "스프링클러 헤드",
            [(FullReplacement, 25, 100)]
        ),
        item!(
            "3-라-3",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "라. 소화설비",
            "소화수관(강관)",
            [(FullReplacement, 25, 100)]
        ),
        item!(
            "3-마-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "마. 승강기 및 인양기",
            "기계장치",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-마-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "마. 승강기 및 인양기",
            "와이어로프·시브",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-마-3",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "마. 승강기 및 인양기",
            "제어반",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-마-4",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "마. 승강기 및 인양기",
            "조속기",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-마-5",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "마. 승강기 및 인양기",
            "문개폐장치",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-바-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "바. 기계식주차장",
            "기계장치",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-바-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "바. 기계식주차장",
            "와이어로프·시브",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-바-3",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "바. 기계식주차장",
            "체인·스프로킷",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-바-4",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "바. 기계식주차장",
            "제어반",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "3-사-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "사. 피뢰설비 및 옥외전등",
            "피뢰설비",
            [(PartialRepair, 10, 30)]
        ),
        item!(
            "3-사-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "사. 피뢰설비 및 옥외전등",
            "보안등",
            [(FullReplacement, 25, 100)]
        ),
        item!(
            "3-아-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "아. 통신 및 방송설비",
            "앰프 및 스피커",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-아-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "아. 통신 및 방송설비",
            "방송수신 공동설비",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "3-자-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "자. 보일러실 및 기계실",
            "동력반",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "3-차-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "차. 보안·방범시설",
            "감시반",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-차-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "차. 보안·방범시설",
            "녹화장치",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-차-3",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "차. 보안·방범시설",
            "영상정보처리기기 및 침입탐지시설",
            [(FullReplacement, 5, 100)]
        ),
        item!(
            "3-카-1",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "카. 지능형 홈네트워크 설비",
            "홈네트워크 기기",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "3-카-2",
            "3. 전기·소화·승강기 및 지능형 홈네트워크 설비",
            "카. 지능형 홈네트워크 설비",
            "단지공용 시스템 장비",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "4-가-1",
            "4. 급수·가스·배수 및 환기설비",
            "가. 급수설비",
            "급수펌프",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "4-가-2",
            "4. 급수·가스·배수 및 환기설비",
            "가. 급수설비",
            "저수조(STS·합성수지)",
            [(FullReplacement, 25, 100)]
        ),
        item!(
            "4-가-3",
            "4. 급수·가스·배수 및 환기설비",
            "가. 급수설비",
            "급수관(강관)",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "4-나-1",
            "4. 급수·가스·배수 및 환기설비",
            "나. 가스설비",
            "배관",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "4-나-2",
            "4. 급수·가스·배수 및 환기설비",
            "나. 가스설비",
            "밸브",
            [(PartialRepair, 10, 30)]
        ),
        item!(
            "4-다-1",
            "4. 급수·가스·배수 및 환기설비",
            "다. 배수설비",
            "펌프",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "4-다-2",
            "4. 급수·가스·배수 및 환기설비",
            "다. 배수설비",
            "오·배수관(주철)",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "4-다-3",
            "4. 급수·가스·배수 및 환기설비",
            "다. 배수설비",
            "오·배수관(PVC)",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "4-라-1",
            "4. 급수·가스·배수 및 환기설비",
            "라. 환기설비",
            "환기팬",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "5-가-1",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "보일러",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "5-가-2",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "급수탱크",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "5-가-3",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "순환펌프",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "5-가-4",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "난방관(강관)",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "5-가-5",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "자동제어기기",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "5-가-6",
            "5. 난방 및 급탕설비",
            "가. 난방설비",
            "열교환기",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "5-나-1",
            "5. 난방 및 급탕설비",
            "나. 급탕설비",
            "순환펌프",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "5-나-2",
            "5. 난방 및 급탕설비",
            "나. 급탕설비",
            "급탕탱크",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "5-나-3",
            "5. 난방 및 급탕설비",
            "나. 급탕설비",
            "급탕관(강관)",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "6-1",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "아스팔트 포장",
            [(PartialRepair, 5, 10), (FullRepair, 15, 100)]
        ),
        item!(
            "6-2",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "울타리",
            [(FullReplacement, 20, 100)]
        ),
        item!(
            "6-3",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "어린이놀이시설",
            [(PartialRepair, 5, 10), (FullReplacement, 15, 100)]
        ),
        item!(
            "6-4",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "보도블록",
            [(PartialRepair, 5, 10), (FullReplacement, 15, 100)]
        ),
        item!(
            "6-5",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "정화조",
            [(PartialRepair, 5, 15)]
        ),
        item!(
            "6-6",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "배수로 및 맨홀",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "6-7",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "현관입구·지하주차장 진입로 지붕",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "6-8",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "자전거보관소",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "6-9",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "주차차단기",
            [(FullReplacement, 10, 100)]
        ),
        item!(
            "6-10",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "조경시설물",
            [(PartialRepair, 10, 10)]
        ),
        item!(
            "6-11",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "안내표지판",
            [(PartialRepair, 10, 30)]
        ),
        item!(
            "6-12",
            "6. 옥외 부대시설 및 옥외 복리시설",
            "옥외시설",
            "전기자동차 고정형 충전기",
            [(PartialRepair, 5, 10), (FullReplacement, 10, 100)]
        ),
        item!(
            "7-1",
            "7. 피난시설",
            "피난시설",
            "방화문",
            [(FullReplacement, 15, 100)]
        ),
        item!(
            "7-2",
            "7. 피난시설",
            "피난시설",
            "옥상 비상문 자동개폐장치",
            [(PartialRepair, 5, 30), (FullReplacement, 15, 100)]
        ),
    ];
    for item in &mut catalog {
        let notes: &[&str] = match item.id.as_str() {
            "3-나-1" | "4-가-1" => &["고효율 기자재 적용"],
            "3-바-1" => &["법정 범위에 해당하는 기계식주차장에 적용"],
            "3-사-2" => &["HID 또는 LED 보안등"],
            "4-라-1" => &["소형 환풍기 제외"],
            "6-12" => &["공동주택이 직접 설치·운영·관리하는 충전기에 한함"],
            "7-1" => &["공용부분에 한함"],
            _ => &[],
        };
        item.notes = notes.iter().map(|note| (*note).to_string()).collect();
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_is_exactly_69_unique_items_in_seven_groups() {
        let catalog = statutory_catalog();
        assert_eq!(catalog.len(), 69);
        assert_eq!(
            catalog
                .iter()
                .map(|item| &item.id)
                .collect::<BTreeSet<_>>()
                .len(),
            69
        );
        assert_eq!(
            catalog
                .iter()
                .map(|item| &item.group)
                .collect::<BTreeSet<_>>()
                .len(),
            7
        );
        assert!(catalog.iter().all(|item| !item.methods.is_empty()));
    }
}
