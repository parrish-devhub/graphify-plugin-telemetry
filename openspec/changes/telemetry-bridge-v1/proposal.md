# Proposal — graphify-plugin-telemetry（telemetry-bridge-v1）

> 內容依 Architecture & Technical Specification v1.0.0（Status: Approved）
> 重建；原貼文已不在工作上下文，逐字版待原作者重貼後替換。

## 背景

Graphify 生態需要一條 Observability 橋接：把上游 Draco MCP（System User
域，抓取 Prometheus / OTEL / Jaeger / pprof）的原始觀測數據，升維對齊到
Graphify Core 的 AST symbol 圖譜，並把 hotspot 語意回饋給 Coding Agent。
本 plugin 即該橋接 — **Pure Bridge（Draco MCP First）, Zero Heavy OTLP
Engine, Symbol-Native Hotspot Binding**。

## 問題

- 觀測點位以 `file_path + line_number` 描述，隨 AST 重建/行號位移失效，
  無法跨 session 穩定引用。
- Draco 抓到的數據需要「哪個 symbol 是 hotspot、影響哪些 caller」的語意
  判定，而非原始數字轉貼。
- 既有 plugin 生態（handoff / opendoc / review）已定義 GraphifyPlugin
  trait 契約與 `graphify.db` 共用慣例，telemetry 須以相同契約對齊。

## 目標（非目標）

**In-Scope**：Draco MCP 介面對接（輪詢/檔案 ingest）、line→symbol 升維、
`telemetry_bindings` 表（workspace_key 隔離）、hotspot 門檻判定、
.toon 語意合成、graphify-mcp 自動註冊 telemetry* 工具。

**Out-of-Scope（硬性禁止）**：不自建 OTLP/Prometheus 服務端（不監聽
4317/4318 埠）、不執行主動 Profiling（不注入 eBPF/pprof agent）、不把
Telemetry 節點塞進 Core AST 圖譜、不做程序級環境變數秘密持久化。

## 採用方案

以 Graphify 內嵌型 Rust crate（實作 `GraphifyPlugin` trait）落地，tool
註冊由 graphify-mcp 統一處理；資料流：

```
Draco MCP ── MCP Tool Call / JSON file ──> plugin（resolver → registry → hotspot 判定）
    ──> graphify-mcp（telemetry_ingest / telemetry_get_context 自動註冊）
    ──> .toon 合成區塊 ──> Coding Agent
```

## 驗收

1. `telemetry_ingest` / `telemetry_get_context` 由 graphify-mcp 自動註冊。
2. Ingest 後 `telemetry_bindings` 依 schema 落庫，hotspot 旗標正確。
3. `sync_toon` packet 內含 hotspot 可觀測性區塊（§6 格式）。
4. `cargo test -p graphify-plugin-telemetry` 全綠。
