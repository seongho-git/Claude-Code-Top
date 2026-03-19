# Claude Code Monitor — 구현 스펙

> htop 스타일의 터미널 UI로 Claude Code 세션과 비용을 실시간 모니터링한다.

---

## 목차

1. [전체 레이아웃 구조](#1-전체-레이아웃-구조)
2. [데이터 소스 및 수집 방법](#2-데이터-소스-및-수집-방법)
3. [영역별 상세 스펙](#3-영역별-상세-스펙)
   - 3.1 타이틀 바
   - 3.2 요약 헤더 바
   - 3.3 로컬 세션 목록
   - 3.4 원격 세션 목록
   - 3.5 세션 상세 패널
   - 3.6 토큰 사용량 스파크라인
   - 3.7 상태 바
4. [Cache Hit Rate 상세](#4-cache-hit-rate-상세)
5. [모델별 컨텍스트 및 단가](#5-모델별-컨텍스트-및-단가)
6. [색상 체계](#6-색상-체계)
7. [키 바인딩](#7-키-바인딩)
8. [구현 스택 제안](#8-구현-스택-제안)
9. [데이터 갱신 주기](#9-데이터-갱신-주기)

---

## 1. 전체 레이아웃 구조

```
┌─────────────────────────────────────────────────┐
│ 타이틀 바 (터미널 chrome)                          │
├─────────────────────────────────────────────────┤
│ 헤더 바 (앱 이름, 버전, 현재 시각)                  │
├─────────────────────────────────────────────────┤
│ 요약 헤더 바 (일/월 비용, 캐시 절감액, 세션 수)       │
├─────────────────────────────────────────────────┤
│ LOCAL SESSIONS 섹션 헤더                          │
├─────────────────────────────────────────────────┤
│ 컬럼 헤더                                         │
│ 세션 행 × N (로컬)                                │
├─────────────────────────────────────────────────┤
│ REMOTE SESSIONS 섹션 헤더                         │
├─────────────────────────────────────────────────┤
│ 컬럼 헤더                                         │
│ 세션 행 × M (원격)                                │
├─────────────────────────────────────────────────┤
│ 세션 상세 패널 (선택된 세션, 좌/우 분할)             │
├─────────────────────────────────────────────────┤
│ 토큰 사용량 스파크라인 (12h)                        │
├─────────────────────────────────────────────────┤
│ 상태 바 (키 바인딩 힌트, 갱신 주기, live 인디케이터) │
└─────────────────────────────────────────────────┘
```

전체 터미널 너비 기준 **80컬럼 이상** 권장. 배경색 `#0d1117`.

---

## 2. 데이터 소스 및 수집 방법

### 2.1 세션 탐지

Claude Code는 실행 시 특정 프로세스를 생성한다.

```bash
# 실행 중인 Claude Code 프로세스 탐지
ps aux | grep 'claude'

# 또는 socket/pid 파일 감시
~/.claude/sessions/*.json   # 세션 메타데이터 (추정 경로)
```

각 세션 디렉토리(`$CLAUDE_PROJECT_DIR`)에서 다음 정보를 수집:
- PID
- 프로젝트 경로
- 사용 모델
- 상태 (running / waiting / idle / error)

### 2.2 API 응답 후킹

Claude Code가 Anthropic API를 호출할 때 응답 헤더 또는 응답 본문에서 토큰 사용량을 추출한다.

```json
// Anthropic API 응답 usage 필드
{
  "usage": {
    "input_tokens": 248901,
    "output_tokens": 35201,
    "cache_creation_input_tokens": 62310,
    "cache_read_input_tokens": 169252
  }
}
```

Claude Code 내부 로그 파일 또는 프록시 방식으로 수집 가능:
- `~/.claude/logs/` 디렉토리 감시 (파일 테일링)
- 로컬 HTTP 프록시 (mitmproxy 등)로 트래픽 인터셉트
- Claude Code 자체 `--debug` 플래그 활용 (지원 시)

### 2.3 원격 세션 탐지

```bash
# SSH 연결 중인 원격 Claude Code 세션
ssh <host> "ps aux | grep claude"

# 또는 원격 호스트의 세션 파일 폴링
ssh <host> "cat ~/.claude/sessions/*.json"
```

원격 세션은 `HOST:PATH` 형태로 표시한다.

---

## 3. 영역별 상세 스펙

### 3.1 타이틀 바

| 항목 | 내용 |
|------|------|
| 배경색 | `#161b22` |
| 내용 | macOS-style 트래픽 라이트 버튼 (red/yellow/green), 중앙에 터미널 제목 |
| 터미널 제목 형식 | `claude-code-monitor — 80×24` |

### 3.2 요약 헤더 바

배경 `#0d1117`, 높이 약 52px (터미널 2행).

| 컬럼 | 내용 | 표시 형식 |
|------|------|-----------|
| Daily cost | 오늘 누적 비용 / 일일 예산 | `$3.42 / $20.00` + 진행 바 |
| Monthly cost | 이번 달 누적 비용 / 월 예산 | `$47.80 / $200` + 진행 바 |
| Cache saved today | 캐시 덕분에 절감된 비용 | `$1.28` (초록색) + `avg hit rate 64%` |
| Sessions | 로컬/원격 세션 수 + 활성 수 | `4 local  2 remote` + `● 4 active` |

**진행 바 색상:**
- 50% 미만 → `#3fb950` (초록)
- 50~80% → `#d29922` (노란)
- 80% 초과 → `#f78166` (빨간)

### 3.3 로컬 세션 목록

섹션 헤더 배경 `#161b22`, 텍스트 `#58a6ff` (파란색).

#### 컬럼 정의

| 컬럼명 | 내용 | 너비 |
|--------|------|------|
| `PID` | 프로세스 ID | 6ch |
| `PROJECT` | 프로젝트 경로 (`~/` 약식) | 18ch |
| `MODEL` | 모델명 단축 표기 | 12ch |
| `STATUS` | 상태 인디케이터 | 10ch |
| `CTX USED / MAX` | 사용 토큰 / 최대 컨텍스트 + 미니 진행 바 | 18ch |
| `CACHE` | 캐시 히트율 % | 6ch |
| `COST` | 세션 누적 비용 | 8ch |
| `DURATION` | 세션 경과 시간 | 9ch |

#### STATUS 값

| 값 | 표시 | 색상 |
|----|------|------|
| running | `● running` | `#3fb950` |
| waiting | `⏸ waiting` | `#d29922` |
| idle | `○ idle` | `#6e7681` |
| error | `✕ error` | `#f85149` |

#### 행 색상

- **선택된 행**: 배경 `#1c2b3a`
- 짝수 행: 배경 `#0d1117`
- idle/error 행: 전체 텍스트 `#6e7681` (흐리게)

#### 미니 컨텍스트 진행 바

CTX 컬럼 내 텍스트 옆에 36px 너비의 인라인 바를 표시한다.

```
141k / 200k  [███████░░░]  70%
```

- 70% 미만 → `#58a6ff`
- 70~90% → `#d29922`
- 90% 초과 → `#f78166`

### 3.4 원격 세션 목록

섹션 헤더 배경 `#161b22`, 텍스트 `#d29922` (노란색으로 로컬과 구분).

컬럼 구조는 로컬과 동일하되, `PROJECT` 컬럼이 `HOST / PROJECT` 형식으로 변경:

```
prod-server:~/deploy
ci-runner:~/tests
```

호스트명은 `#d29922`, 경로는 기본색으로 표시하거나 같은 색으로 통일.

### 3.5 세션 상세 패널

선택된 세션(하이라이트 행)의 상세 정보를 표시한다. 패널은 좌/우 2분할.

#### 좌측: 세션 상세

```
▶ session 8821  ~/work/api-refactor  [local]

model          claude-sonnet-4-6
               input:  $3.00/Mtok    output: $15.00/Mtok

ctx window     [████████████░░░░░░░] 70%  (141k / 200k)

input tokens   248,901   → $0.75
output tokens   35,201   → $0.53

cache reads    169,252  (68%)   → $0.05  (saved ~$0.46)
cache writes    62,310          → $0.23  (1.25× write fee)

total cost     $0.89   (without cache: ~$1.35)

hit rate bar   [████████████░░░░░░░] 68%
```

**계산 공식:**
```
input_cost  = input_tokens / 1_000_000 * input_price
output_cost = output_tokens / 1_000_000 * output_price
cache_read_cost  = cache_read_tokens / 1_000_000 * input_price * 0.1
cache_write_cost = cache_write_tokens / 1_000_000 * input_price * 1.25
total_cost  = input_cost + output_cost + cache_read_cost + cache_write_cost

without_cache_cost = (input_tokens + cache_read_tokens) / 1_000_000 * input_price
                   + output_cost
saved = without_cache_cost - total_cost
```

#### 우측: 툴 활동 로그

최근 N개의 툴 호출 이력을 시간순으로 표시한다.

```
tool activity log

14:08   read_file   src/routes/api.ts
14:11   edit_file   src/routes/api.ts
14:15   bash        npm test
14:19   bash        ✓ 14 passed        ← 성공은 #3fb950
14:22   read_file   src/middleware/auth.ts
14:27   edit_file   src/middleware/auth.ts
14:30   bash        ● running...       ← 실행 중은 #d29922
```

툴 종류별 색상:
- 성공 (`✓`) → `#3fb950`
- 실행 중 (`●`) → `#d29922`
- 실패/오류 (`✕`) → `#f85149`
- 일반 → `#8b949e`

#### 우측 하단: 캐시 설명 박스

처음 보는 사용자를 위한 간략 설명 박스 (토글 가능).

```
┌─ cache hit rate: 68% ──────────────────┐
│ 시스템 프롬프트 + 파일 컨텍스트가         │
│ 반복 전송될 때 캐시에서 로드됨.           │
│                                         │
│ read  0.10× 단가  →  90% 절감           │
│ write 1.25× 단가  →  첫 저장 비용        │
│                                         │
│ 이 세션 절감액  $0.46                   │
└─────────────────────────────────────────┘
```

배경 `#161b22`, 테두리 `#21262d`.

### 3.6 토큰 사용량 스파크라인

12시간 구간의 전체 세션 합산 컨텍스트 토큰 사용량을 막대 그래프로 표시.

| 항목 | 내용 |
|------|------|
| X축 | 시간 (2시간 간격 레이블) |
| Y축 | 토큰 수 (0 / 500k / 1M) |
| 막대 색상 | 사용량 기준 (아래 표 참고) |
| 갱신 | 2초마다 우측에 새 막대 추가 (스크롤) |

**막대 색상 기준 (전체 세션 합산 기준):**

| 구간 | 색상 | 의미 |
|------|------|------|
| ~200k | `#58a6ff` (파랑) | low |
| 200k~500k | `#3fb950` (초록) | mid |
| 500k~800k | `#f78166` (주황) | high |
| 800k+ | `#f85149` (빨강) | peak |

### 3.7 상태 바

화면 최하단, 배경 `#1f2937`.

```
F1:help  F2:sort  F3:search  F5:refresh  F9:kill  q:quit    auto-refresh: 2s  ● live
```

- `● live` 인디케이터: 연결 정상 시 `#3fb950`, 끊김 시 `#f85149`

---

## 4. Cache Hit Rate 상세

Anthropic API는 **Prompt Caching** 기능을 통해 동일한 prefix를 캐시한다.

### 4.1 작동 원리

```
첫 번째 호출:
  [시스템 프롬프트 + 파일 컨텍스트]  →  cache_creation_input_tokens
  [새로운 사용자 메시지]             →  input_tokens

두 번째 호출 (동일 prefix):
  [시스템 프롬프트 + 파일 컨텍스트]  →  cache_read_input_tokens (0.1× 단가)
  [새로운 사용자 메시지]             →  input_tokens
```

Claude Code는 각 턴마다 전체 파일 컨텍스트를 재전송하므로 캐시 히트율이 높다.

### 4.2 히트율 계산

```
hit_rate = cache_read_input_tokens
         / (cache_read_input_tokens + cache_creation_input_tokens + input_tokens)
         * 100
```

### 4.3 비용 비교

| 토큰 종류 | 단가 배율 | sonnet-4-6 예시 |
|-----------|-----------|-----------------|
| input (일반) | 1× | $3.00/Mtok |
| cache write | 1.25× | $3.75/Mtok |
| cache read | 0.1× | $0.30/Mtok |
| output | 5× | $15.00/Mtok |

### 4.4 색상 기준

| 히트율 | 색상 | 판단 |
|--------|------|------|
| 60% 이상 | `#3fb950` (초록) | 양호 |
| 30~60% | `#d29922` (노란) | 보통 |
| 30% 미만 | `#f85149` (빨강) | 낮음 |

---

## 5. 모델별 컨텍스트 및 단가

### 5.1 컨텍스트 윈도우

| 모델 | 최대 컨텍스트 | 표시 |
|------|--------------|------|
| claude-opus-4-6 | 1,000,000 | `1M` |
| claude-sonnet-4-6 | 200,000 | `200k` |
| claude-haiku-4-5 | 200,000 | `200k` |

CTX USED / MAX 컬럼에서 모델에 맞는 MAX 값을 자동 적용.

### 5.2 모델 단가 (참고값, 변경될 수 있음)

| 모델 | Input ($/Mtok) | Output ($/Mtok) |
|------|---------------|-----------------|
| opus-4-6 | $15.00 | $75.00 |
| sonnet-4-6 | $3.00 | $15.00 |
| haiku-4-5 | $0.80 | $4.00 |

세션 상세 패널에서 선택된 세션의 모델 단가를 함께 표시.

### 5.3 모델명 단축 표기 (세션 목록)

| 전체 모델명 | 목록 표시 |
|-------------|-----------|
| claude-opus-4-6 | `opus-4-6` |
| claude-sonnet-4-6 | `sonnet-4-6` |
| claude-haiku-4-5-20251001 | `haiku-4-5` |

---

## 6. 색상 체계

전체 배경: `#0d1117` (GitHub Dark 계열)

### 기본 팔레트

| 용도 | 색상 코드 | 사용처 |
|------|-----------|--------|
| 강조 텍스트 | `#e6edf3` | 주요 수치, 세션 PID |
| 보조 텍스트 | `#8b949e` | 레이블, 컬럼 헤더값 |
| 흐린 텍스트 | `#6e7681` | idle/error 세션 전체 |
| 파랑 | `#58a6ff` | 컬럼 헤더, 프로젝트 경로, 선택 패널 제목 |
| 초록 | `#3fb950` | running 상태, 성공, 낮은 비용, 캐시 절감 |
| 노랑 | `#d29922` | waiting 상태, 원격 세션 헤더, 보통 캐시율 |
| 주황 | `#f78166` | 높은 비용, CTX 70~90%, high 사용량 |
| 빨강 | `#f85149` | error 상태, CTX 90%+, peak 사용량 |

### 배경 팔레트

| 용도 | 색상 코드 |
|------|-----------|
| 메인 배경 | `#0d1117` |
| 섹션 헤더 / 타이틀 | `#161b22` |
| 헤더 바 / 상태 바 | `#1f2937` |
| 선택된 행 | `#1c2b3a` |
| 비활성 진행 바 배경 | `#21262d` |
| 박스 테두리 | `#21262d` |
| 터미널 테두리 | `#30363d` |

---

## 7. 키 바인딩

| 키 | 동작 |
|----|------|
| `↑` / `↓` | 세션 선택 이동 |
| `Tab` | 로컬 ↔ 원격 섹션 전환 |
| `F1` | 도움말 오버레이 |
| `F2` | 정렬 기준 변경 (PID / COST / CTX / DURATION) |
| `F3` | 프로젝트 경로 검색 |
| `F5` | 수동 갱신 |
| `F9` | 선택된 세션 종료 (kill) |
| `q` / `F10` | 모니터 종료 |
| `c` | 캐시 설명 박스 토글 |
| `+` / `-` | 스파크라인 시간 범위 조절 (1h ~ 24h) |

---

## 8. 구현 스택 제안

### 옵션 A: Python (권장)

```
blessed / urwid       — 터미널 UI 레이아웃
psutil                — 프로세스 탐지 (PID, 경로)
watchdog              — 로그 파일 감시
httpx / mitmproxy     — API 응답 인터셉트 또는 로그 파싱
```

```python
# 기본 구조
import blessed
import psutil
import asyncio

term = blessed.Terminal()

async def main():
    sessions = detect_sessions()
    while True:
        render(term, sessions)
        await asyncio.sleep(2)
```

### 옵션 B: Node.js

```
blessed-contrib         — htop 스타일 위젯 (sparkline, table 내장)
ink                     — React 기반 터미널 UI
@anthropic-ai/sdk       — 토큰 사용량 후킹 가능
```

### 옵션 C: Go

```
tview / bubbletea       — TUI 프레임워크
gopsutil                — 프로세스 정보
```

bubbletea는 htop 스타일 구현에 잘 맞고, 바이너리 배포가 쉽다는 장점이 있다.

### 데이터 파이프라인

```
Claude Code 프로세스
        │
        ├─ ps / proc 파일시스템  →  PID, 경로, 상태
        │
        └─ API 로그 파일 tail  →  토큰 수 (input/output/cache)
                │
                ▼
          세션 상태 집계
                │
                ▼
          TUI 렌더러 (2초 갱신)
```

---

## 9. 데이터 갱신 주기

| 데이터 | 갱신 주기 | 방법 |
|--------|-----------|------|
| 세션 목록 (PID, 상태) | 2초 | `ps` 폴링 또는 inotify |
| 토큰 / 비용 | API 응답 시 즉시 | 로그 파일 tail |
| 캐시 히트율 | API 응답 시 즉시 | 누적 계산 |
| 스파크라인 데이터 | 2초 (집계) | 내부 ring buffer |
| 원격 세션 | 5초 | SSH 폴링 |

스파크라인은 **ring buffer** 자료구조로 최근 N개 데이터포인트를 유지하고, 화면 너비에 맞게 막대 수를 조절한다.

---

*생성일: 2026-03-19*
*버전: v0.4.0 기준*
