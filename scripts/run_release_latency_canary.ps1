[CmdletBinding()]
param(
    [ValidateRange(1, 256)]
    [int]$Sessions = 64,
    [ValidateRange(1, 32)]
    [int]$HttpWaves = 1,
    [string]$ExecutablePath = 'D:\B_Core_validation\cargo-target-gnu\release\b-core-concurrency-canary.exe',
    [string]$HttpExecutablePath = 'D:\B_Core_validation\cargo-target-gnu\release\b-core-standalone-latency-canary.exe'
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) {
    throw "Release latency canary executable not found: $ExecutablePath"
}
if (-not (Test-Path -LiteralPath $HttpExecutablePath -PathType Leaf)) {
    throw "Release HTTP latency canary executable not found: $HttpExecutablePath"
}

# These are workload classes, not knowledge inputs. The canary accepts the
# same text for every isolated conversation and verifies the normal B_Core
# response contract on every request.
$cases = @(
    [pscustomobject]@{
        Name = 'SHORT_STATEMENT'
        Text = $null
        FollowUpText = $null
        RuntimeP95BudgetMillis = 30.0
        HttpP95BudgetMillis = 80.0
    },
    [pscustomobject]@{
        Name = 'MULTI_CLAUSE_WORK_REQUEST'
        Text = '해외 거래처의 원자재 선적 일정이 항만 파업으로 일주일 연기되었습니다. 기존 재고는 목요일에 소진될 예정이므로, 금요일부터 조립 2라인의 가동이 중단될 위험이 있습니다. 국내 대체 벤더에 긴급 발주를 진행할지 승인해 주세요.'
        FollowUpText = $null
        RuntimeP95BudgetMillis = 80.0
        HttpP95BudgetMillis = 80.0
    },
    [pscustomobject]@{
        Name = 'LONG_INCIDENT_REPORT'
        Text = '오전 11시에 배포한 할인 쿠폰 검증 변경 이후 결제 승인 대기 시간이 급격히 늘었습니다. 초기에는 트래픽 증가가 원인으로 보였지만, 애플리케이션 로그에서는 동일 주문에 대한 쿠폰 검증 요청이 반복되는 패턴이 확인됐습니다. 중복 요청 방지 스크립트가 새 버튼 클래스와 연결되지 않아 사용자가 결제 버튼을 여러 번 누를 때마다 검증 트랜잭션이 병렬로 생성된 것으로 보입니다. 현재 데이터베이스 연결 풀 사용률은 94%이며, 결제 모듈 세션 타임아웃도 함께 증가했습니다. 우선 변경 사항을 롤백하고, 중복 요청의 idempotency key가 모든 경로에서 검증되는지 확인해야 합니다. 롤백 뒤에도 연결 풀 사용률이 정상화되지 않으면 장기 실행 트랜잭션을 조회한 뒤 보류 중인 결제를 재처리할지 결정하겠습니다.'
        FollowUpText = $null
        RuntimeP95BudgetMillis = 200.0
        HttpP95BudgetMillis = 200.0
    },
    [pscustomobject]@{
        Name = 'COMPLEX_LONG_INCIDENT_REPORT'
        Text = '오늘 오전 결제 승인 지연은 단순 트래픽 급증이 아니라 쿠폰 검증 경로의 중복 요청 방지 연결이 끊어진 상태에서 발생한 것으로 보입니다. 같은 주문 번호에 대해 검증 트랜잭션이 반복 생성됐고, 이 트랜잭션들이 데이터베이스 연결 풀을 점유하면서 세션 타임아웃이 연쇄적으로 증가했습니다. 다만 모든 지연이 쿠폰 변경 때문이라고 단정할 수는 없습니다. 항만 파업으로 원자재 선적이 일주일 미뤄졌고, 기존 재고는 목요일에 소진될 예정이라 금요일부터 조립 2라인을 멈춰야 할 위험도 동시에 존재합니다. 결제 경로는 우선 롤백한 뒤 idempotency key가 웹과 모바일 경로 모두에서 동일하게 검증되는지 확인해야 합니다. 롤백 뒤에도 연결 풀 사용률이 정상화되지 않으면 장기 실행 트랜잭션과 재시도 큐를 분리해 조사해야 합니다. 생산 일정은 국내 대체 벤더의 납기와 단가를 비교한 뒤 긴급 발주 승인 여부를 결정해야 합니다. 납기를 고정하면 비용이나 기능 범위 중 하나를 조정해야 한다는 제약도 계획에 명시해야 합니다. 고객에게는 결제 실패가 확정됐다고 안내하지 말고 승인 지연을 조사 중이며 재시도가 중복 청구를 만들지 않도록 보호하고 있다고 설명해야 합니다. 운영팀은 롤백 완료 시각, 연결 풀 회복 시각, 재시도 건수, 보류 결제 수를 같은 사건 기록에 남겨 후속 판단의 근거로 사용해야 합니다.'
        FollowUpText = $null
        RuntimeP95BudgetMillis = 300.0
        HttpP95BudgetMillis = 300.0
    },
    [pscustomobject]@{
        Name = 'TWO_TURN_REFERENCE_FOLLOW_UP'
        Text = '회의 시간이 오후 4시야.'
        FollowUpText = '몇 시라고 했지?'
        RuntimeP95BudgetMillis = 50.0
        HttpP95BudgetMillis = 80.0
    }
)

$environmentNames = @(
    'B_CORE_CANARY_SESSIONS',
    'B_CORE_CANARY_MAX_P95_MS',
    'B_CORE_CANARY_TEXT',
    'B_CORE_CANARY_FOLLOW_UP_TEXT',
    'B_CORE_HTTP_CANARY_SESSIONS',
    'B_CORE_HTTP_CANARY_WAVES',
    'B_CORE_HTTP_CANARY_MAX_P95_MS',
    'B_CORE_HTTP_CANARY_TEXT',
    'B_CORE_HTTP_CANARY_FOLLOW_UP_TEXT'
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $item = Get-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    $previousEnvironment[$name] = if ($null -eq $item) { $null } else { $item.Value }
}

$summaries = [System.Collections.Generic.List[object]]::new()
try {
    foreach ($case in $cases) {
        $env:B_CORE_CANARY_SESSIONS = "$Sessions"
        $env:B_CORE_CANARY_MAX_P95_MS = "$($case.RuntimeP95BudgetMillis)"
        if ($null -eq $case.Text) {
            Remove-Item -LiteralPath 'Env:B_CORE_CANARY_TEXT' -ErrorAction SilentlyContinue
        } else {
            $env:B_CORE_CANARY_TEXT = $case.Text
        }
        if ($null -eq $case.FollowUpText) {
            Remove-Item -LiteralPath 'Env:B_CORE_CANARY_FOLLOW_UP_TEXT' -ErrorAction SilentlyContinue
        } else {
            $env:B_CORE_CANARY_FOLLOW_UP_TEXT = $case.FollowUpText
        }
        $runtimeTextBytes = if ($null -eq $env:B_CORE_CANARY_TEXT) { 0 } else { [Text.Encoding]::UTF8.GetByteCount($env:B_CORE_CANARY_TEXT) }
        $followUpTextBytes = if ($null -eq $env:B_CORE_CANARY_FOLLOW_UP_TEXT) { 0 } else { [Text.Encoding]::UTF8.GetByteCount($env:B_CORE_CANARY_FOLLOW_UP_TEXT) }
        Write-Verbose ("runtime case={0} input_chars={1} input_utf8_bytes={2} follow_up_utf8_bytes={3}" -f $case.Name, $env:B_CORE_CANARY_TEXT.Length, $runtimeTextBytes, $followUpTextBytes)

        $raw = & $ExecutablePath
        if ($LASTEXITCODE -ne 0) {
            throw "Latency canary failed for $($case.Name) with exit code $LASTEXITCODE`n$($raw -join [Environment]::NewLine)"
        }
        $report = ($raw -join [Environment]::NewLine) | ConvertFrom-Json
        if ($report.status -ne 'PASS' -or -not $report.latency_p95_budget_passed) {
            throw "Latency canary budget failed for $($case.Name)"
        }
        $summaries.Add([pscustomobject]@{
            case = $case.Name
            path = 'RUNTIME'
            sessions = $report.sessions_started_together
            turns_per_session = $report.turns_per_session
            completed_turns = $report.completed_turns
            closed_conversations = $null
            failed_conversation_closes = $null
            active_conversations_after_close = $null
            input_bytes = $report.input_bytes
            p95_millis = $report.latency_p95_ms
            p95_budget_millis = $report.latency_p95_budget_ms
            throughput_requests_per_second = $report.throughput_requests_per_second
            rejected_busy = $report.rejected_busy
            peak_pending_connections = $null
            average_queue_wait_micros = $null
            max_queue_wait_micros = $null
        })

        if ($null -eq $case.HttpP95BudgetMillis) {
            continue
        }
        $env:B_CORE_HTTP_CANARY_SESSIONS = "$Sessions"
        $env:B_CORE_HTTP_CANARY_WAVES = "$HttpWaves"
        $env:B_CORE_HTTP_CANARY_MAX_P95_MS = "$($case.HttpP95BudgetMillis)"
        if ($null -eq $case.Text) {
            Remove-Item -LiteralPath 'Env:B_CORE_HTTP_CANARY_TEXT' -ErrorAction SilentlyContinue
        } else {
            $env:B_CORE_HTTP_CANARY_TEXT = $case.Text
        }
        if ($null -eq $case.FollowUpText) {
            Remove-Item -LiteralPath 'Env:B_CORE_HTTP_CANARY_FOLLOW_UP_TEXT' -ErrorAction SilentlyContinue
        } else {
            $env:B_CORE_HTTP_CANARY_FOLLOW_UP_TEXT = $case.FollowUpText
        }

        $raw = & $HttpExecutablePath
        if ($LASTEXITCODE -ne 0) {
            throw "HTTP latency canary failed for $($case.Name) with exit code $LASTEXITCODE`n$($raw -join [Environment]::NewLine)"
        }
        $report = ($raw -join [Environment]::NewLine) | ConvertFrom-Json
        if ($report.status -ne 'PASS' -or -not $report.latency_p95_budget_passed) {
            throw "HTTP latency canary budget failed for $($case.Name)"
        }
        $summaries.Add([pscustomobject]@{
            case = $case.Name
            path = 'HTTP'
            sessions = $report.sessions_started_together
            waves = $report.waves
            sessions_total = $report.sessions_total
            turns_per_session = $report.turns_per_session
            completed_turns = $report.completed_turns
            closed_conversations = $report.closed_conversations
            failed_conversation_closes = $report.failed_conversation_closes
            active_conversations_after_close = $report.active_conversations_after_close
            input_bytes = $report.input_bytes
            p95_millis = $report.latency_p95_ms
            p95_budget_millis = $report.latency_p95_budget_ms
            throughput_requests_per_second = $report.throughput_requests_per_second
            rejected_busy = $report.rejected_connections + $report.runtime_busy_rejections
            peak_pending_connections = $report.peak_pending_connections
            average_queue_wait_micros = $report.average_queue_wait_micros
            max_queue_wait_micros = $report.max_queue_wait_micros
        })
    }
} finally {
    foreach ($name in $environmentNames) {
        $value = $previousEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        } else {
            Set-Item -LiteralPath "Env:$name" -Value $value
        }
    }
}

Write-Output 'B_CORE_RELEASE_LATENCY_CANARY_SUITE=PASS'
$summaries | ConvertTo-Json -Depth 3
