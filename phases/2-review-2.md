# `Phase 2` Review 2

## Status

更新后的 `phases/2-api-service.md` 已覆盖大部分在 `phases/2-review-1.md` 中确认的决策：

- 所有前置修正归入 `Phase 2a`。
- `Phase 2a` 和 `Phase 2b` 按顺序执行并分别验证。
- 用户位置快照中的 `hands` 包含 `current_trick` 牌。
- 调用 `DDS` 前移除 `current_trick` 牌。
- 输入覆盖优先级为 `URL query > JSON body > PBN`。
- 使用四个独立的 `analyze` endpoint。
- 使用共享应用层、统一错误结构、`spawn_blocking()`、`Semaphore` 和受限 `CORS`。
- 在 `Phase 2b` 中实现 `AnalysePlayPBN` 和完整 play analysis。

计划方向已经基本收束，但仍有若干会直接影响实现边界和公开协议的问题。建议解决本文前五项后，再批准 `Phase 2a Task 1` 并开始代码工作。

## Verified

本轮对照检查包括：

- `phases/2-api-service.md`。
- `phases/2-review-1.md`。
- `PLAN.md`、`INIT.md` 和 `phases/pbn-input-contract.md`。
- 当前 `Position`、play advancement、`DDS` wrapper 和 `CLI` 实现。
- `DDS` 的 `SolveBoardPBN`、`AnalysePlayPBN` 和 `RETURN_PLAYED_CARD` 语义。

文档检查结果：

- `git diff --check -- phases/2-api-service.md phases/2-review-1.md` 通过。
- 本轮审阅未修改现有代码或计划文档。

## Findings

### 1. `Phase 2a Task 1` 要求的公开协议仍未完整定义

`phases/2-api-service.md` 要求在 `Task 1` 中定义完整输入和输出契约，但当前文档仅给出了 continuation 请求示例。

仍缺少：

- 四个 endpoint 的完整 `JSON body` schema。
- 四个 endpoint 的完整 response schema。
- 独立 `deal` 字段的具体表示。
- 独立 `play` 字段的具体表示。
- 每个 endpoint 允许使用的 `URL query` 字段。
- `null` 是否表示未提供、显式清除，或直接无效。
- 合并后的必填字段规则。
- response 中 trick count 的精确视角和语义。

在这些契约缺失时，transport `DTO`、应用层命令和测试 fixture 都无法稳定设计。

建议为每个 endpoint 增加以下固定章节：

```text
Purpose
Accepted PBN tags
Allowed URL query fields
JSON request schema
Normalized application command
JSON response schema
Validation rules
Examples
```

### 2. 延期同步 `CLI --format json` 与共享契约目标冲突

当前计划将 `CLI --format json` 与公开 response `DTO` 的同步延期到 `Phase 2` 之后。

这与已确认的核心约束冲突：

- `CLI` 和 `HTTP API` 必须调用同一个应用层。
- 相同规范化输入必须产生相同领域结果。
- `CLI --format json` 应尽量复用公开 response `DTO`。

如果 `CLI JSON` 继续使用独立定义，应用层结果和公开协议仍可能发生漂移。

建议：

- `Phase 2a` 必须统一应用层强类型结果。
- `Phase 2a` 中让 `CLI --format json` 使用与 `HTTP API` 相同的 response `DTO`。
- `CLI` 文本输出继续独立格式化。
- 移除将该事项延期到 post-`Phase 2` cleanup 的决定。

当前验证要求还同时包含 byte-identical output 和正确性修复后的输出检查。这两项需要统一为：

- 未涉及正确性修复的 fixture 保持不变。
- 已批准的正确性修复允许改变输出，并更新对应 fixture。

### 3. `AnalysePlayPBN` 的错误牌判定规则不正确

当前计划要求标识导致 double-dummy trick count 下降的牌。

`AnalysePlayPBN` 的结果始终从 declarer 视角表示，因此错误判断取决于出牌者所属阵营：

- declarer side 出牌后 declarer trick count 下降，表示 declarer side 出错。
- defender side 出牌后 declarer trick count 上升，表示 defender side 出错。
- defender side 出牌后 declarer trick count 下降通常不是错误。

因此不能统一使用 trick count drop 判断错误。

建议 `PlayEvaluation` 至少包含：

```rust
pub struct PlayEvaluation {
    pub player: Direction,
    pub card: Card,
    pub tricks_before: u8,
    pub tricks_after: u8,
    pub delta_for_declarer: i8,
    pub is_mistake: bool,
}
```

计划必须明确：

- `tricks_before` 和 `tricks_after` 始终从 declarer 视角解释。
- `is_mistake` 根据出牌者属于 declarer side 或 defender side 进行判断。
- 没有改变 double-dummy 结果的牌不是 mistake，但可能只是多个等价选择之一。

### 4. `/api/v1/analyze/play` 的必填输入和覆盖规则未定义

完整 play analysis 使用 `AnalysePlayPBN` 时至少需要：

- 完整 `deal`。
- `trump`。
- opening leader，或可用于推导 opening leader 的 `declarer`。
- play sequence。

`dealer` 和 `vulnerable` 对 `AnalysePlayPBN` 本身不是必需输入。

当前计划只描述为完整牌局字段和 play sequence，不足以实现。需要明确：

- `Play` tag 带方向前缀时，该方向是否直接作为 opening leader。
- `Play` tag 不带方向前缀时，是否强制要求 `declarer`。
- 同时提供 Play prefix 和 `declarer` 时，是否交叉验证二者关系。
- `trump` 可以来自 `URL query`、`JSON body`、`Trump` tag，还是未来的 `Contract` tag。
- `AnalysePlayPBN` 结果中的 declarer 是 opening leader 的右手玩家，该关系必须如何验证。
- play sequence 的独立 `JSON` 表示和长度限制。

### 5. `Phase 2a` play use case 与 `Phase 2b` 实现存在任务冲突

当前计划要求 `Phase 2a`：

- 每个 application module 暴露一个调用 solver 的 use-case function。
- 为四个 operation 增加应用层 use-case 测试。

但完整 play analysis 所需的 `AnalysePlayPBN` 要到 `Phase 2b` 才实现。

建议明确任务边界：

- `Phase 2a` 定义 `PlayAnalysis` 和 `PlayEvaluation` 契约。
- `Phase 2a` 实现 play 输入规范化、合法性验证、状态推进和最终 continuation。
- `Phase 2a` 测试以上已实现部分和结果契约，不要求完整 historical evaluation。
- `Phase 2b` 增加 `AnalysePlayPBN`，补充 historical evaluation，并完成最终 `analyze_play()` 用例和 endpoint。

### 6. 部分 `PBN` 输入尚未进入项目 `PBN` 契约

计划允许完整或部分 `PBN`，再由 body 或 query 补充字段。

当前 `phases/pbn-input-contract.md` 仍要求完整牌局必须同时包含 `Deal`、`Dealer` 和 `Vulnerable`，并声明不支持 `Play` parsing。

这会导致以下行为不明确：

- `/api/v1/analyze/deal` 是否允许只有 `[Deal]`，然后由 query 提供 `dealer` 和 `vulnerable`。
- `/api/v1/analyze/position/matrix` 接受哪些 `PBN` tags。
- `/api/v1/analyze/position/continuation` 如何将 `[First]`、`[CurrentTrick]` 和新模型对应。
- `/api/v1/analyze/play` 接受哪些 tags。

建议将修订 `phases/pbn-input-contract.md` 加入 `Phase 2a Task 1`。该契约需要区分：

- 完整 record validation。
- endpoint-specific partial record parsing。
- 每个 endpoint 支持的 tags。
- 字段覆盖后的最终必填约束。

### 7. 同层重复字段检测需要明确实现策略

计划要求同一输入层中的重复或冲突字段返回错误。

默认 `serde` 和常见 `axum` extractor 不一定能可靠拒绝：

```json
{
  "dealer": "N",
  "dealer": "E"
}
```

重复 query key 也可能被默认处理逻辑静默选择。

建议二选一并写入计划：

1. 保留严格规则，使用能够检测重复 key 的自定义 body 和 query 解析。
2. 将规则收窄为：同层不同表示之间产生冲突时返回错误，不保证检测原始 `JSON` 或 query 中的重复 key。

如果严格重复检测是公开协议的一部分，必须在测试计划中覆盖。

### 8. `mode` 已不再属于有效 query 字段

position matrix 和 continuation 已拆分为独立 endpoint，因此不再需要 `mode` query。

建议移除全局允许字段列表中的 `mode`，并为每个 endpoint 单独列出允许的 query 字段。无意义或不属于该 endpoint 的 query 字段应返回明确错误。

### 9. `Semaphore` 和过载行为需要更具体

当前 `DDS_LOCK` 会串行化所有 `DDS` 调用。如果 `Semaphore` 允许多个并发 permit，多余的 blocking task 只会占用 blocking thread，然后等待 `DDS_LOCK`。

建议初始服务配置明确为：

- solver concurrency 为 `1`。
- 使用有界等待队列。
- 使用 `try_acquire()` 或获取许可超时判断过载。
- 队列满时返回 `503`。

还需要明确：`spawn_blocking()` 任务启动后无法被强制取消。请求返回 `504` 后，底层求解任务可能继续运行并占用 solver。计划不能承诺实际取消正在执行的 `DDS` 调用。

### 10. `PLAN.md` 和 `PBN` 契约不应等到 `Phase 2` 完成后更新

当前计划将 `PLAN.md` 和 `INIT.md` 更新放在完成整个 `Phase 2` 之后。

但 `PLAN.md` 仍包含旧 endpoint 和旧 `Phase 1b` 状态，`phases/pbn-input-contract.md` 也与新的输入策略冲突。

建议：

- `Phase 2a Task 1` 同步更新 `PLAN.md` 和 `phases/pbn-input-contract.md`。
- 完成 `Phase 2` 后更新 `INIT.md` 和 `README.md`，描述最终实现和使用方式。

### 11. 文档规范和 reference 仍不完整

当前文档比上一版清晰，但仍有技术术语没有使用 Markdown inline code，例如多个英文 task heading、`endpoint`、`application layer` 和相关术语。

`Reference` 章节仍缺少：

- `axum` 官方文档链接。
- `Tokio` `spawn_blocking()` 和 `Semaphore` 官方文档链接。
- `tower-http` `CORS` 和 tracing 官方文档链接。
- 外部 `PBN` specification 链接。

建议在批准计划前完成文档格式和 reference 修订。

## Recommended Revisions

建议按以下顺序修订 `phases/2-api-service.md`：

1. 补齐四个 endpoint 的完整请求、响应和字段来源契约。
2. 将 `CLI --format json` 与 response `DTO` 同步保留在 `Phase 2a`。
3. 定义 `AnalysePlayPBN` 的 declarer/defender 错误判定语义。
4. 明确 `/api/v1/analyze/play` 的必填字段、来源和覆盖规则。
5. 解决 `Phase 2a` play use case 与 `Phase 2b` historical evaluation 的任务边界。
6. 将更新 `PLAN.md` 和 `phases/pbn-input-contract.md` 加入 `Phase 2a Task 1`。
7. 决定并记录重复字段检测策略。
8. 移除 `mode`，按 endpoint 定义允许的 query 字段。
9. 明确单 permit solver、有界队列和不可取消的超时行为。
10. 补齐文档格式和 reference。

## Approval Conditions

建议在满足以下条件后确认 `Phase 2a Task 1` 并开始代码工作：

- 四个 endpoint 的 request 和 response contract 已完整记录。
- play analysis 的输入、输出视角和 mistake 判定规则已明确。
- `Phase 2a` 与 `Phase 2b` 的 play analysis 职责边界无冲突。
- `PBN` partial record 和覆盖规则已进入项目输入契约。
- `CLI JSON` 与公开 response `DTO` 的同步不再延期。
- query 字段、重复字段和 solver admission control 策略已明确。

## Recommendation

更新后的计划已经接近可执行状态。完成本文列出的协议和任务边界修订后，可以确认 `Phase 2a Task 1`，随后按计划顺序开始实施。
