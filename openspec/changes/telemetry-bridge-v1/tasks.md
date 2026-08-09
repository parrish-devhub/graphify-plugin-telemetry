# Tasks — telemetry-bridge-v1（Slice 0-2 路線圖）

> 依 Architecture & Technical Specification v1.0.0（Approved）重建。

## Slice 0 — Bridge Core（✅ 已完成）

- [x] Task 0.1: crate setup + stub（Cargo.toml、lib.rs、空 graphify-mcp 驗證）
- [x] Task 0.2: resolver（line→symbol 升維，對齊 GraphOutput）
- [x] Task 0.3: registry（telemetry_bindings DAO + 索引 + upsert）
- [x] Task 0.4: ingest + draco_client（IngestPayload 解析、MCP-over-HTTP 骨架）
- [x] Task 0.5: sync.rs（.toon packet v1.0.0 合成 + hotspot 區塊）
- [x] Task 0.6: graphify-mcp 自動註冊驗證（tools/list 23 tools 含
  telemetryIngest / telemetryGetContext；端到端 ingest 2 bound 1 hotspot）

## Slice 1 — Draco Polling + 動態門檻（🔜 下一階段）

- [ ] Task 1.1: Draco MCP 連線實測 — probe 4 tools 清單與 base URL
      （目前 `fetch_top_hotspots` 為唯一確認名，port 佔位待修正）
- [ ] Task 1.2: `source=draco-mcp` 輪詢路徑實作（fetch_top_hotspots → ingest）
- [ ] Task 1.3: 動態門檻定案 — 定案 `p99 > 500ms || alloc > 5MB`（Slice 1
      範例值）與 schema 註解 1000ms 之爭；評估 per-workspace 設定
- [ ] Task 1.4: 一鍵同步 Top 10 熱點

## Slice 2 — Impact Radius + Hotspot Alert（🔜）

- [ ] Task 2.1: `on_graph_updated` 增量更新 graph_cache
- [ ] Task 2.2: BFS 衝擊半徑（upstream callers）— `telemetry_get_context`
      `include_impact_radius=true` 實作
- [ ] Task 2.3: `notifications/telemetry/hotspot_alert` 雙向廣播
