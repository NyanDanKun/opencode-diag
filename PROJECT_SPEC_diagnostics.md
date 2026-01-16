# OpenCode Diagnostics Tool

## Идея

Инструмент диагностики для определения причины ошибок "server at capacity" и других проблем с подключением к AI-сервисам.

## Название варианты

- `opencode-diag`
- `ai-health`
- `connection-doctor`
- `capacity-check`

## Что проверяем (цепочка)

```
[User PC] → [VPN?] → [Internet] → [Claude API] → [OpenCode] → [Agent]
```

### 1. Локальные ресурсы
- CPU usage (%)
- RAM usage (%)
- Disk I/O
- **Статус**: OK / WARNING / CRITICAL

### 2. Сеть / Интернет
- Ping to 8.8.8.8 (Google DNS)
- Ping to 1.1.1.1 (Cloudflare)
- Download speed test (optional)
- **Статус**: CONNECTED / SLOW / DISCONNECTED

### 3. VPN (если используется)
- Detect active VPN interfaces
- Ping through VPN
- **Статус**: ACTIVE / INACTIVE / BLOCKING

### 4. Claude/Anthropic API
- HTTPS request to api.anthropic.com
- Response time
- Status code (200, 429, 503, etc.)
- **Статус**: AVAILABLE / RATE_LIMITED / OVERLOADED / DOWN

### 5. OpenCode сервер
- Check opencode.ai endpoint
- **Статус**: AVAILABLE / DOWN

### 6. Локальный OpenCode процесс
- Check if opencode is running
- Memory usage
- **Статус**: RUNNING / NOT_RUNNING

## UI Layout

```
┌─────────────────────────────────────────────────┐
│ ■ OPENCODE DIAGNOSTICS                   [DARK] │
├─────────────────────────────────────────────────┤
│                                                 │
│ // SYSTEM CHECK                                 │
│                                                 │
│ ┃ LOCAL RESOURCES                        [ OK ] │
│ ┃ CPU: 24% :: RAM: 67% :: DISK: NORMAL          │
│                                                 │
│ ┃ INTERNET                               [ OK ] │
│ ┃ PING: 15ms :: google.com reachable            │
│                                                 │
│ ┃ VPN                                [INACTIVE] │
│ ┃ No VPN detected                               │
│                                                 │
│ ┃ CLAUDE API                         [OVERLOAD] │
│ ┃ api.anthropic.com :: 503 :: server at capacity│
│                                                 │
│ ┃ OPENCODE                               [ OK ] │
│ ┃ Process running :: PID 12345                  │
│                                                 │
├─────────────────────────────────────────────────┤
│ [  RUN DIAGNOSTICS  ]           [ COPY REPORT ] │
├─────────────────────────────────────────────────┤
│ SYS.STATUS: ISSUE FOUND → CLAUDE API   v0.1.0   │
└─────────────────────────────────────────────────┘
```

## Статусы и цвета

| Status | Light Theme | Dark Theme |
|--------|-------------|------------|
| OK | #2a2a2a (black) | #4caf50 (green) |
| WARNING | #ff9800 (orange) | #ff9800 |
| ERROR | #d32f2f (red) | #f44336 |
| UNKNOWN | #888888 (gray) | #5c5c5c |

## Функции

1. **RUN DIAGNOSTICS** — запустить все проверки
2. **COPY REPORT** — скопировать текстовый отчёт в буфер
3. **Auto-refresh** — опционально обновлять каждые 30 сек

## Зависимости Rust

```toml
[dependencies]
eframe = "0.29"
egui = "0.29"
sysinfo = "0.32"          # CPU, RAM, processes
reqwest = { version = "0.12", features = ["blocking"] }
ping = "0.5"              # ICMP ping (или surge-ping)
arboard = "3"             # Clipboard
```

## Выходные данные

### Консольный отчёт (для копирования)
```
=== OpenCode Diagnostics Report ===
Time: 2026-01-16 22:30:45

[OK] Local Resources
     CPU: 24% | RAM: 67% | Disk: Normal

[OK] Internet
     Ping: 15ms | google.com: reachable

[--] VPN
     No VPN detected

[!!] Claude API
     Status: 503 Service Unavailable
     Message: "server at capacity"

[OK] OpenCode
     Process: running (PID 12345)

DIAGNOSIS: Claude API is overloaded. Try again later.
```

## MVP Features (v0.1.0)

1. [ ] Basic UI with theme toggle
2. [ ] CPU/RAM check
3. [ ] Internet ping
4. [ ] Claude API check (HTTP request)
5. [ ] Copy report to clipboard

## Future (v0.2.0+)

- [ ] VPN detection
- [ ] OpenCode process check
- [ ] Historical data (graph)
- [ ] Auto-retry when API available
- [ ] Notifications
