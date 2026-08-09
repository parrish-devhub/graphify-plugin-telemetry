# Design — graphify-plugin-telemetry（telemetry-bridge-v1）

> 依 Architecture & Technical Specification v1.0.0（Approved）重建。

## 架構鏈

```
Draco MCP（System User 域；Prometheus / OTEL / Jaeger / pprof）
    │  MCP Tool Call / 檔案型 JSON（pprof.json / otel-metrics.json）
    ▼
graphify-plugin-telemetry（本 repo）
    │  Line/Stack-to-Symbol Resolver → telemetry_bindings DAO
    │  Hotspot Impact & Threshold Guard → .toon 語意合成
    ▼
graphify-mcp（GraphifyRust/）— 啟動時自動註冊 telemetry* 工具
    │  telemetry_ingest / telemetry_get_context
    │  notifications/telemetry/hotspot_alert（Slice 2）
    ▼
Coding Agent（.toon Consumer）
```

## 目錄結構（與 review plugin 手寫內嵌 Pattern 對齊）

```
src/
├── lib.rs          # TelemetryPlugin + GraphifyPlugin trait
├── ingest.rs       # TelemetryIngestPayload JSON 解析與 Draco 轉譯
├── draco_client.rs # Draco MCP Client（MCP-over-HTTP）
├── resolver.rs     # Line/Stack-to-Symbol Resolver（對齊 GraphOutput）
├── registry.rs     # telemetry_bindings DAO（併入 graphify.db）
├── telemetry.rs    # telemetry_ingest / telemetry_get_context 業務邏輯 + 門檻判定
└── sync.rs         # .toon packet v1.0.0 合成 + hotspot 區塊
```

## TelemetryPlugin 狀態模型

```
TelemetryPlugin {
    service: TelemetryService {
        workspace_key: String,
        registry_path: Option<PathBuf>,      // 開路徑時惰性建立
        graph_cache: Arc<RwLock<Option<GraphOutput>>>,
        draco_client: DracoMcpClient,
    }
}
```

- 圖來源：不自己建圖 — `sync_toon` 被動快取 Core 的 `GraphOutput`；
  `on_graph_updated`（Slice 2）以 `&GraphUpdateEvent` 增量更新快取。
- `workspace_key` 由 `WorkspaceContext.bind` 取得一次（常駐記憶體快取，
  不重複 walk-up）；plugin 間以 workspace_key 對齊，不各自 walk-up。

## 圖譜契約（graphify-core v1）

- `GraphifyPlugin` trait：`get_id` / `bind` / `get_workspace_key` /
  `sync_toon(Option<Vec<u8>>) -> Vec<u8>` / `on_graph_updated(&mut self,
  &GraphUpdateEvent)`。
- 升維格式：`{file_path}:{kind}:{name}`（e.g. `src/db/query.rs:function:query_users`）。
- resolver 對齊 `GraphOutput` 的 node `line_start/line_end` 區間與 AST
  巢狀 parent 鏈，從最深層開始匹配行號。

## MCP 整合（graphify-mcp）

- 工具：`telemetryIngest` / `telemetryGetContext`，由 graphify-mcp 啟動時
  自動註冊（tools/list + dispatch arm），plugin 不寫 MCP Protocol Server。
- 工具名採 camelCase（graphify-mcp 慣例）；錯誤訊息格式 `[telemetry] ...`。

## 資料流（telemetry_ingest）

1. 讀取檔案型 JSON（`source=file`，Slice 0）或 Draco MCP 輪詢
   （`source=draco-mcp`，Slice 1）。
2. 每筆 metric：resolver 升維 line→symbol → canonical_node_id。
3. `telemetry_bindings` upsert（composite PK `workspace_key + id`）。
4. 門檻判定（`p99 > 1000ms || alloc > 5MB`）→ `is_hotspot`。
5. 回報 IngestReport（total / bound / orphan / hotspots）。

## 安全與隱私

- 版本控制檔案禁止：本地網路拓撲、私有主機名、本地 IP、機器絕對路徑。
- 秘密一律動態（環境變數/檔案/記憶體綁定），拒絕程序級持久化。
