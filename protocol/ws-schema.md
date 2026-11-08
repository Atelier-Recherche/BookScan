# bookscan WebSocket schema (`/v1/ws`)

All messages are JSON objects with a required `type` string field.

## PC → APK

```json
{ "type": "capture_now" }
```

```json
{ "type": "set_mode", "mode": "auto" | "semi" | "manual" }
```

```json
{
  "type": "set_auto_params",
  "settle_ms": 600,
  "cooldown_ms": 1200,
  "motion_threshold": 12.0
}
```

## APK → PC

```json
{ "type": "hello", "device": "bookscan", "version": "0.1.0" }
```

```json
{ "type": "ready" }
```

```json
{ "type": "point_state", "state": "searching" | "stable" | "armed" }
```

```json
{ "type": "capturing" }
```

```json
{ "type": "uploaded", "page_id": "<uuid>" }
```

```json
{ "type": "error", "message": "..." }
```
