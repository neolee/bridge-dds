# `Phase 2` Review 1

## Status

`Phase 2` 的总体方向已确认：先在 `Phase 2a` 中统一领域契约、建立共享应用层并定义公开协议，再在 `Phase 2b` 中实现完整的 `HTTP` 服务。

所有前置修正和共享逻辑工作都属于 `Phase 2a`，不新增其他阶段。`Phase 2b` 负责 `bridge-server`、完整的 `HTTP API`、`AnalysePlayPBN` 集成和服务验证。

当前 `phases/2-api-service.md` 需要根据本文已经确认的决策重新编写后，才能开始实施。

## Verified

审阅范围包括：

- `phases/2-api-service.md`。
- `phases/1a-full-deal-dds.md` 和 `phases/1b-mid-hand-analysis.md`。
- `PLAN.md`、`INIT.md` 和 `phases/pbn-input-contract.md`。
- `src/core/`、`src/dds/`、`src/cli/` 和现有测试。
- `DDS` 的 `SolveBoardPBN`、`AnalysePlayPBN` 和 `RETURN_PLAYED_CARD` 实现及文档。

当前验证结果：

- `cargo test -- --test-threads=1` 通过，共 `48` 个测试。
- `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- `cargo fmt --check` 通过。
- 审阅过程中未修改代码。

## Confirmed Decisions

### 1. User-Facing Position Semantics

用户输入的当前位置采用符合牌手习惯的快照模型：

- `hands` 表示当前墩开始前各玩家持有的牌。
- 四手 `hands` 的张数必须一致。
- `current_trick` 表示当前墩已经打出的 `0` 至 `3` 张牌。
- `current_trick` 中的每张牌仍包含在对应玩家的 `hands` 中。
- continuation 输入同时提供 `trick_leader` 和 `next_to_act`。
- 服务必须验证 `current_trick` 的牌属于相应玩家，并验证出牌顺序、跟牌规则和 `next_to_act`。

字段名使用 `current_trick`。

`DDS` 的 `dealPBN.remainCards` 不允许包含已经打出的当前墩牌，否则会返回 `RETURN_PLAYED_CARD`。因此应用层在调用 `DDS` 前必须从 `hands` 中移除 `current_trick`，生成仅供求解器使用的 remaining hands。

用户输入模型与 `DDS` 输入模型必须通过明确的转换边界隔离。

### 2. Supported Request Sources And Override Rules

相关操作同时支持：

- 完整或部分 `PBN` 输入。
- `JSON body` 中的独立字段。
- `URL query` 中允许覆盖的简单字段。

覆盖优先级确定为：

```text
URL query > JSON body fields > PBN fields
```

同一层中重复或冲突的字段必须返回错误，不能静默选择。

适合放入 `URL query` 的简单字段包括：

- `mode`。
- `dealer`。
- `declarer`。
- `vulnerable`。
- `trump`。
- `trick_leader`。
- `next_to_act`。

复杂字段必须放入 `JSON body`：

- `pbn`。
- `deal`。
- `hands`。
- `current_trick`。
- `play`。

所有来源合并后，应用层对最终规范化输入执行完整校验。

### 3. Public Endpoints

公开接口使用统一的 `analyze` 命名体系：

```text
POST /api/v1/analyze/deal
POST /api/v1/analyze/position/matrix
POST /api/v1/analyze/position/continuation
POST /api/v1/analyze/play
```

不同功能使用独立 endpoint 和独立请求、响应类型，不使用单一 tagged request enum。

应用层内部可以使用 enum 统一分派，但该内部结构不应决定公开 `HTTP` 协议。

### 4. Deal Analysis Input

`POST /api/v1/analyze/deal` 支持：

- 包含 `Deal`、`Dealer` 和 `Vulnerable` 的完整 `PBN`。
- 独立的 `deal`、`dealer` 和 `vulnerable` 字段。
- `PBN` 与独立字段组合输入。
- `JSON body` 和 `URL query` 中的独立字段按已确认优先级覆盖 `PBN`。

输出包括完整双明手矩阵和 `DealerPar` 结果。

### 5. Position Matrix Input

`POST /api/v1/analyze/position/matrix` 只要求四手等长的 `hands`：

- 支持完整牌局和残局。
- 不需要 `trump`。
- 不需要 `trick_leader`。
- 不需要 `next_to_act`。
- 不允许非空 `current_trick`。

该操作计算不同首攻者和不同将牌组合的 position matrix。

### 6. Continuation Input

`POST /api/v1/analyze/position/continuation` 要求：

- `hands`。
- `trump`。
- `trick_leader`。
- `next_to_act`。
- 可选的 `current_trick`，长度为 `0` 至 `3`。

`current_trick` 使用按出牌顺序排列的牌字符串，不为每张牌重复提供玩家：

```json
{
  "hands": {
    "N": ["SA", "SK"],
    "E": ["HA", "HK"],
    "S": ["DA", "DK"],
    "W": ["CA", "CK"]
  },
  "trump": "NT",
  "trick_leader": "N",
  "current_trick": ["SA", "HA"],
  "next_to_act": "S"
}
```

每张牌的出牌者由 `trick_leader` 和顺时针顺序推导。服务必须验证推导出的下一位玩家等于 `next_to_act`。

continuation 以独立字段作为主要结构化输入方式，同时支持项目定义的 residual `PBN` tags。

### 7. Play Analysis Input And Output

`POST /api/v1/analyze/play` 支持：

- 包含完整牌局和 `Play` tag 的 `PBN`。
- 独立的完整牌局字段和 play sequence。
- `PBN` 与独立字段组合输入及覆盖。

该操作在 `Phase 2b` 中一次性实现完整功能：

- 使用 `AnalysePlayPBN` 返回初始位置及每张已出牌之后的双明手结果。
- 标识导致双明手可得墩数下降的出牌。
- 推进到最终位置。
- 使用 `SolveBoardPBN` 返回最终位置的 continuation 建议。

应用层应返回强类型结果，至少包含历史出牌评价和最终 continuation：

```rust
pub struct PlayAnalysis {
    pub trace: Vec<PlayEvaluation>,
    pub continuation: ContinuationAnalysis,
}
```

### 8. `JSON` Representation

公开 `JSON` 使用以下表示：

- `Card`：`"SA"`。
- `Direction`：`"N"`。
- `Strain`：`"S"`、`"H"`、`"D"`、`"C"` 或 `"NT"`。
- `Vulnerability`：`"None"`、`"NS"`、`"EW"` 或 `"All"`。
- `hands`：以方向为 key，以牌字符串数组为值。
- `current_trick`：按出牌顺序排列的牌字符串数组。

公开协议不得暴露 `Hand` 的内部 `u64` 位布局。

请求 `DTO` 必须拒绝未知 `JSON` 字段，并返回明确错误。该规则不改变现有 `PBN` 未知 tag 的处理策略。

### 9. Shared Application Layer

`CLI` 和 `HTTP API` 必须调用同一个应用层：

- 相同规范化输入必须产生相同领域结果。
- `CLI --format json` 应尽量复用公开 response `DTO`。
- 文本输出由 `CLI` 表现层独立格式化。
- 正确性修复允许改变原有错误结果、错误消息和不正确的 play-trace 结果。
- 所有预期变化必须更新并持续维护测试 fixtures。

应用层不得依赖 `axum`、`HTTP status`、`Json` 或 `serde_json::Value`。

### 10. Error Protocol

所有 `HTTP` 错误使用稳定的机器可读错误码：

```json
{
  "error": {
    "code": "invalid_position",
    "message": "current_trick has 4 cards, max 3"
  }
}
```

状态码至少包括：

- `400`：无效输入或冲突参数。
- `413`：请求 body 过大。
- `500`：内部错误。
- `503`：solver 队列过载。
- `504`：请求超时。

### 11. Server Runtime

`bridge-server` 是提供 `HTTP API` 的 server。

服务端实现采用：

- 服务启动时调用一次 `DdsSolver::init()`。
- 使用 `tokio::task::spawn_blocking()` 执行同步 `DDS` 调用。
- 使用 `Semaphore` 限制并发和排队。
- 对队列过载和请求超时返回明确错误。

开发阶段由独立的 `Vite` server 提供前端页面和热更新，`bridge-server` 提供 `HTTP API`，通过受限 `CORS` 通信。

发布阶段继续使用 `Vite` 构建产物，还是将 web assets 嵌入 `bridge-server`，留到后续阶段决定。

### 12. Dependencies

实施时采用最新稳定版本，不采用预发布版本：

- 最新稳定的 `axum`。
- 与 `axum` 兼容的最新稳定 `tower-http`。
- 最新稳定的 `Tokio` `1.x`。

仅启用实际需要的 features，不默认启用 `tokio/full`。

## Findings To Address

### 1. Existing `Position` Semantics Conflict With The Confirmed Model

当前 `Position` 注释、`Position::validate()`、residual `PBN` 路径和 play-trace 路径对 `hands` 与 `current_trick` 的关系并不一致。

需要建立明确的用户快照模型，并在调用 `DDS` 前转换为移除 `current_trick` 的 solver position。

### 2. Existing Play-Trace Advancement Is Incomplete

`Position::play_card()` 当前没有强制执行跟牌规则。现有 `cmd_play_trace()` 在完成一个或多个墩后还会使用原始完整 `hands` 构造最终位置，导致历史已打出的牌重新出现。

必须新增共享且完整的 play-trace advancement，并由 `CLI`、应用层和 `HTTP API` 共同使用。

### 3. Public Solver Boundaries Are Not Fully Validated

`DdsSolver::solve_position()` 当前不会执行完整 position 校验。畸形结构化请求可能绕过现有 `PBN` 校验，甚至在写入固定长度 `DDS` 数组时触发 panic。

所有公共应用层用例必须在调用 `DDS` 前完成完整校验。

### 4. The Existing Plan Mixes Application And Transport Concerns

当前方案将应用用例、`JSON DTO`、play-trace 推进和求解逻辑全部放入 `src/api.rs`，并使用多个 `Option` 字段表示不同响应。

应用层、transport `DTO` 和 `HTTP` 适配层必须分离。每个公开 endpoint 使用独立请求和响应类型。

### 5. Server Execution And Error Handling Need Full Design

同步且全局串行化的 `DDS` 调用不能直接在异步 handler 中执行。现有错误设计也没有覆盖无效 `JSON`、body 限制、任务失败、超时和过载。

`Phase 2b` 必须实现已确认的 blocking execution、admission control 和统一错误协议。

## Recommended `Phase 2a` Tasks

以下任务全部属于 `Phase 2a`，并应按顺序执行。

### Task 1: Define Public Contracts

根据已确认决策，在 `phases/2-api-service.md` 中定义：

- 四个公开 endpoint。
- 每个 endpoint 的 `PBN`、`JSON body` 和 `URL query` 输入。
- 覆盖优先级和冲突规则。
- 每个 endpoint 的独立请求和响应结构。
- `JSON` 表示、错误结构和完整示例。
- play analysis 的历史评价与最终 continuation 输出。

### Task 2: Unify Position Models And Conversion

建立符合用户习惯的 position input model：

- `hands` 包含 `current_trick` 中的牌。
- 四手张数一致。
- `trick_leader`、`current_trick` 和 `next_to_act` 可进行交叉验证。

建立调用 `DDS` 前的转换：

- 验证当前墩牌属于相应玩家。
- 验证顺序和跟牌规则。
- 从 `hands` 中移除 `current_trick`。
- 生成 `DDS` 所需的 remaining hands。

统一 `Position` 注释、校验、推进逻辑、residual `PBN` 路径和 `DDS` 转换。

### Task 3: Complete Shared Play Advancement

新增共享 play advancement，负责：

- 校验出牌者顺序。
- 校验持牌情况。
- 强制跟牌。
- 正确计算每墩赢家。
- 正确移除历史已打出的牌。
- 输出最终规范化位置。

`CLI` 不再自行实现该逻辑。

### Task 4: Harden Solver Boundaries

保证所有公共应用用例在调用 `DDS` 前完成校验：

- 完整牌局完整性和重复牌。
- 四手张数约束。
- `current_trick` 长度和持牌关系。
- 出牌顺序和跟牌规则。
- `trick_leader` 与 `next_to_act`。
- position matrix 与 continuation 的不同约束。

增加证明非法输入不会 panic、也不会依赖 `DDS` 拒绝的测试。

### Task 5: Add Shared Application Use Cases

新增独立应用层，封装：

- deal analysis。
- position matrix analysis。
- position continuation analysis。
- play analysis 的共享输入、推进和输出模型。

`Phase 2a` 定义 `PlayAnalysis` 和 `PlayEvaluation` 等强类型结果。`AnalysePlayPBN` 的 `FFI` 和实际历史评价计算在 `Phase 2b` 完成。

建议结构：

```text
src/
├── application/
│   ├── mod.rs
│   ├── deal.rs
│   ├── position.rs
│   └── play.rs
├── core/
└── dds/
```

### Task 6: Define Transport `DTO` And Input Merging

新增 transport `DTO` 和转换层：

- 拒绝未知 `JSON` 字段。
- 解析 `PBN`。
- 解析 `JSON body` 独立字段。
- 解析允许的 `URL query` 字段。
- 按 `query > body > PBN` 合并。
- 拒绝同层重复或冲突字段。
- 将最终输入转换为应用层命令。

该转换层不得公开 `Hand` 的内部位布局。

### Task 7: Refactor `CLI` To Use Application Layer

`CLI` 继续负责：

- 读取 `stdin`。
- 解析命令行 flags。
- 将输入转换为应用层命令。
- 调用应用层。
- 输出文本或 `JSON`。

移除 `CLI` 内重复的求解和 play advancement 逻辑。`CLI --format json` 尽量复用 response `DTO`。

### Task 8: Complete `Phase 2a` Verification

自动化检查：

- `cargo fmt --check`。
- `cargo clippy --all-targets --all-features -- -D warnings`。
- `cargo test -- --test-threads=1`。
- position input 与 solver conversion 测试。
- play advancement 测试。
- 应用层用例测试。
- transport `DTO`、输入合并、覆盖和冲突测试。
- `CLI` golden fixtures。

手工检查：

- 运行现有 full-deal、residual 和 play-trace `CLI` 示例。
- 检查多墩 play trace。
- 检查 continuation 和 position matrix 输入。
- 检查正确性修复后的文本和 `JSON` 输出。

只有 `Phase 2a` 完成并确认后，才开始 `Phase 2b`。

## Recommended `Phase 2b` Tasks

### Task 1: Add `AnalysePlayPBN`

- 增加 `AnalysePlayPBN`、`playTracePBN` 和 `solvedPlay` 的 `FFI`。
- 在安全 wrapper 中返回每张已出牌前后的双明手结果。
- 将结果转换为 `PlayEvaluation`。
- 标识导致双明手可得墩数下降的出牌。

### Task 2: Add Server Runtime And Router

- 增加 `bridge-server` binary。
- 实现四个已确认 endpoint。
- 服务启动时初始化 `DDS`。
- 使用最新稳定依赖和最小必要 features。

### Task 3: Add Blocking Execution And Admission Control

- 使用 `spawn_blocking()` 执行应用层用例。
- 使用 `Semaphore` 限制并发和排队。
- 定义请求超时和队列过载行为。

### Task 4: Add Unified HTTP Errors And Middleware

- 将领域、transport、extractor 和 runtime 错误转换为统一错误响应。
- 限制请求 body 大小。
- 增加请求 tracing。
- 为独立 `Vite` 开发服务配置受限 `CORS`。

### Task 5: Complete Play Analysis Endpoint

`POST /api/v1/analyze/play` 必须同时返回：

- `AnalysePlayPBN` 生成的逐张牌历史评价。
- 最终位置。
- 最终位置的 continuation 建议。

### Task 6: Complete `Phase 2b` Verification

自动化检查：

- `Router::oneshot()` 成功和错误测试。
- 四个 endpoint 的 `PBN`、body、query 和组合输入测试。
- 输入覆盖和冲突测试。
- `AnalysePlayPBN` 历史评价测试。
- 随机端口 smoke test。
- 并发、超时和过载测试。
- 所有 `HTTP` 错误的统一格式测试。

手工检查：

- 使用 `curl` 验证四个 endpoint。
- 验证 body 与 query 覆盖。
- 验证 play analysis 的错误牌识别和最终 continuation。
- 验证独立 `Vite` 开发服务通过受限 `CORS` 访问 API。

## Documentation Updates Required

修订 `phases/2-api-service.md` 时，还需要同步：

- 更新 `PLAN.md` 中的 `Phase 1b` 和 `Phase 2` 状态及 endpoint。
- 明确 `AnalysePlayPBN` 在 `Phase 2b` 中完成。
- 更新 `INIT.md` 中的服务和输入模型描述。
- 使用 `AGENTS.md` 要求的 Markdown inline code 格式。
- 增加 `axum`、`Tokio`、`tower-http` 和相关 `DDS API` 参考链接。
- 在验证章节中区分自动化检查和手工检查。

## Recommendation

根据本文已确认的决策重写 `phases/2-api-service.md`。修订后的计划应将领域模型修正、共享应用层、输入合并和 transport 协议全部放入顺序执行的 `Phase 2a` tasks，并在 `Phase 2a` 经验证确认后开始包含完整 play analysis 的 `Phase 2b`。
