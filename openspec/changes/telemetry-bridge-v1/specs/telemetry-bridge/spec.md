# Spec — telemetry-bridge

> 契約細節依 Architecture & Technical Specification v1.0.0（Approved）
> 重建；逐字版待原作者重貼後替換。

## 1. IngestPayload（telemetry JSON 檔案，schema 1.0）

```json
{
  "version": "1.0",
  "source": "draco-mcp",
  "workspace_key": "my-app-v1",
  "metrics": [
    {
      "metric_id": "tel-p99-001",
      "file_path": "src/db/query.rs",
      "function_name": "query_users",
      "line_number": 88,
      "p50_ms": 12.5,
      "p99_ms": 1250.0,
      "alloc_bytes_per_req": 10485760,
      "call_count_per_min": 5000,
      "environment": "production",
      "recorded_at": "2026-08-10T06:00:00Z"
    }
  ]
}
```

| 欄位 | 型別 | 說明 |
|------|------|------|
| version | string | 契約版本（"1.0"） |
| source | string | 資料源（"draco-mcp" 或 "file"） |
| workspace_key | string | 對齊 plugin 的 workspace 契約 |
| metric_id | string | 唯一識別（如 `tel-p99-001`） |
| file_path | string | 來源檔相對路徑 |
| function_name | string | 函式名（輔助驗證） |
| line_number | u32 | 觀測點位行號 |
| p50_ms / p99_ms | f64 | 延遲分位（毫秒） |
| alloc_bytes_per_req | i64 | 每請求配置位元組（預設 0） |
| call_count_per_min | i64 | 每分鐘呼叫數（預設 0） |
| environment | string | production / staging / local |
| recorded_at | string | RFC3339 時間戳 |

## 2. telemetry_bindings（graphify.db 共用表）

```sql
CREATE TABLE IF NOT EXISTS telemetry_bindings (
    id                  TEXT PRIMARY KEY,
    workspace_key       TEXT NOT NULL,
    canonical_node_id   TEXT NOT NULL,
    file_path           TEXT NOT NULL,
    function_name       TEXT,
    p50_ms              REAL NOT NULL DEFAULT 0,
    p99_ms              REAL NOT NULL DEFAULT 0,
    alloc_bytes         INTEGER NOT NULL DEFAULT 0,
    call_count          INTEGER NOT NULL DEFAULT 0,
    is_hotspot          BOOLEAN NOT NULL DEFAULT 0,
    environment         TEXT NOT NULL DEFAULT 'production',
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_telemetry_node    ON telemetry_bindings(workspace_key, canonical_node_id);
CREATE INDEX IF NOT EXISTS idx_telemetry_hotspot ON telemetry_bindings(workspace_key, is_hotspot);
```

- 註解門檻（schema 註解值）：`p99 > 1000ms` 或 high alloc → `is_hotspot`。
  實作採 `p99 > 1000.0ms || alloc > 5MB`；Slice 1 動態門檻時定案 500ms/5MB 之爭。
- 已採用 deviation（同 review plugin）：實際 composite PK 為
  `(workspace_key, id)`，`id` 不再單獨 PRIMARY KEY — 確保共用
  `graphify.db` 時不同 workspace 的同名 metric 不互踩。
- upsert：`ON CONFLICT(workspace_key, id) DO UPDATE`，保留 `created_at`。

## 3. MCP 工具

### telemetryIngest

```
input: {
  source: "file" | "draco-mcp",          // required
  path_or_draco_params: string | null    // source=file 時為 JSON 路徑
}
```

- `source=file`：讀本地 telemetry JSON，升維 + 落庫 + 門檻判定（Slice 0 已完成）。
- `source=draco-mcp`：`draco_client.fetch_top_hotspots()` 輪詢（Slice 1）。

### telemetryGetContext

```
input: {
  node: string,                    // canonical_node_id，required
  include_impact_radius: bool|null // 含 upstream callers 的 telemetry（Slice 2）
}
```

回傳該節點（+ 影響半徑內 caller）的 p99 / alloc / call_count 綁定。

## 4. .toon 合成（sync_toon）

packet `plugin_data.telemetry`：

```json
{
  "telemetry": {
    "workspace_key": "my-app-v1",
    "bindings": 2,
    "hotspots": 1,
    "plugin": "graphify-plugin-telemetry",
    "toon_block": "[src/db/query.rs:function:query_users (AST Node)]\n ├── ⚡ [Draco Telemetry] p99 Latency: 2,450ms (CRITICAL HOTSPOT)\n └── 📊 Alloc: 15.0MB | Calls: 5,000/min (prod)"
  }
}
```

## 5. 非契約行為（實作細節）

- 無圖快取時 ingest 全數 orphan（`canonical_node_id=""`），不丟數據。
- get_context 以 plugin 綁定的 workspace_key 查詢（與 review 同語意）。
- 工具名 camelCase、回應 `[telemetry] ...`（graphify-mcp 慣例）。
