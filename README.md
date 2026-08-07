# B_Core

SYNAPSE 원본 프로젝트에서 인간 사고 회로 모방 실험에 직접 필요한 부분만 분리한 최소 Rust 워크스페이스입니다.

## 포함 범위

- `crates/synapse-core`: 희소 후보 인덱싱, top-k 활성화, 활성 노드 전파, 억제, 경쟁, 공명, Thought Crystal
- `crates/synapse-recursive-core`: 제한된 재귀 개선 구조
  - 자율성/승인 경계
  - 코드 성장 및 패치 피드백
  - 격리 샌드박스와 저위험 반복 루프
  - SelfDevRuntime 및 ClosedGrowthCycle
  - 상시 코어/온디맨드 재귀 스택의 wake/sleep·메모리 예산
- `docs`: 코어 수학 모델, 아키텍처, 안전 규칙

`synapse-recursive-core`는 `synapse-core`를 재노출하므로 한 의존성에서 두 계층을 함께 사용할 수 있습니다.

## 제외 범위

학습 데이터, 데이터셋, 모델 가중치, 생성 산출물, 평가 증거, 리포트, 캐시, 빌드 결과, 언어/음성/아바타/UI 모듈은 포함하지 않았습니다.

## 확인

```powershell
cargo test --workspace
```

원본 프로젝트는 변경하지 않았으며, 이 폴더는 독립적으로 수정하거나 삭제할 수 있습니다.
