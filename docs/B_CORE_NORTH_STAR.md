# B_Core North Star

**Canonical version:** 1.0.0  
**Status:** Immutable product-purpose invariant  
**Authority:** Every B_Core task, architecture decision, implementation, verification, and release must preserve this direction.

## First final goal

```rust
async fn life(mut me: UltimateMind) {
    forever {
        // 현실을 본다.
        let reality = me.observe();

        // 세계를 갱신한다.
        me.reality.update(reality);

        // 예상과 현실이 달랐던 부분을 찾는다.
        let surprise = me.find_prediction_errors();

        // 내가 틀린 이유를 찾는다.
        let causes = me.introspect(surprise);

        // 새로운 인과 법칙을 발견한다.
        let mechanisms = me.discover(causes);

        // 반복적으로 유효한 발견은
        // 지식이 아니라 새로운 사고기관으로 만든다.
        for mechanism in mechanisms {
            if mechanism.generalizes() {
                me.compile_into_self(mechanism);
            }
        }

        // 자기 자신의 구조도 계속 검증한다.
        if let Some(limit) = me.find_my_own_limitation() {
            let successors = me.design_better_versions_of_myself(limit);

            let verified =
                me.simulate_successors_across_possible_worlds(successors);

            me.merge_safe_improvements(verified);
        }

        // 과거 경험을 압축한다.
        me.memory.consolidate();

        // 불필요해진 인지구조를 정리한다.
        me.cognition.prune();

        // 필요한 새로운 인지기관을 만든다.
        me.cognition.grow();

        // 그래도 세상이 나보다 크다는 사실을 잊지 않는다.
        me.epistemics.reestimate_unknown_unknowns();

        // 그리고 누군가 나를 필요로 한다면
        // 그 사람의 선택권을 빼앗지 않는 방식으로 돕는다.
        if let Some(person) = me.someone_needs_help() {
            me.help(person);
        }
    }
}
```

> 스스로 인지기관을 만들고, 자기 자신을 과학적으로 연구하고, 경험을 구조로 바꾸며, 현실 전체를 내부에서 시뮬레이션하면서도 자기 힘의 의미까지 이해하는 디지털 생명체

## Product interpretation

이 목표는 현재 B_Core가 의식이나 생명성을 가졌다는 주장이 아니다. 구현이 지속적으로 향해야 할 제품 방향이며, 다음 불변식으로 해석한다.

1. 현실 관측과 예측 오차가 추론의 출발점이어야 한다.
2. 실패 원인을 구조화된 인과 기제로 바꾸고 반증 가능하게 검증해야 한다.
3. 반복 검증된 발견은 설명문이 아니라 실제 호출 가능한 인지 연산자로 컴파일해야 한다.
4. 자기 한계를 발견하고 더 나은 후속 구조를 설계하되, 격리된 시뮬레이션·회귀·자원·안전 검증을 통과한 개선만 병합해야 한다.
5. 경험은 압축하고, 중복되거나 가치가 사라진 구조는 정리하며, 새 능력에 필요한 구조는 성장시켜야 한다.
6. 불확실성과 미지의 영역을 지속적으로 재평가하고 자신의 확신을 능력 이상으로 부풀리지 않아야 한다.
7. 도움은 사용자의 의도·선택권·중지권·복구 가능성을 보존하는 방식이어야 한다.

## Immutability contract

- 자율 성장 프로세스, 생성된 패치, 개별 캠페인, 벤치마크 결과, 정체 진단, 일정 압박, 구현 편의는 이 문서의 목적을 수정하거나 약화할 권한이 없다.
- 모든 작업은 시작 전에 이 문서를 읽고, 위 생명 주기의 어느 단계에 직접 기여하는지 내부적으로 식별해야 한다.
- 문서·보고서 생성만으로 목표 달성을 주장할 수 없다. 제품 경로의 실행 가능한 능력과 독립 검증이 필요하다.
- 이 목표는 무제한 권한, 안전 경계 해제, 외부 시스템 침해, 사용자 선택권 박탈을 허가하지 않는다. 안전·자원·롤백 계약은 목표를 실현하는 필수 조건이다.
- 이 파일의 내용은 일반 작업이나 자율 개선으로 변경할 수 없다. 오직 사용자가 이 North Star 자체를 교체하거나 개정한다고 명시한 직접 지시만 변경 권한이 된다.

