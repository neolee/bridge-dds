# `Phase 2` Review 3

## Status

更新后的 `phases/2-api-service.md` 已解决 `phases/2-review-2.md` 中的大部分问题：

- 四个 endpoint 已分别定义请求、响应、允许的 `URL query` 字段和验证规则。
- `CLI --format json` 已明确在 `Phase 2a` 中复用公开 response `DTO`。
- play analysis 已明确 declarer side 和 defender side 的 mistake 判定规则。
- `/api/v1/analyze/play` 的主要输入来源和覆盖关系已定义。
- `Phase 2a` 与 `Phase 2b` 的 play analysis 职责边界已进一步拆分。
- `PLAN.md` 和 `phases/pbn-input-contract.md` 已加入 `Phase 2a Task 1`。
- `mode` query 已移除。
- timeout 不取消 `spawn_blocking()` 任务的限制已明确。
- reference 和文档格式已明显完善。

计划已经接近可批准状态。当前主要剩余问题集中在 position matrix、residual `PBN` 映射、continuation trick count 语义、play response 契约、输入覆盖规则和 admission control 实现。

## Verified

本轮对照检查包括：

- `phases/2-api-service.md`。
- `phases/2-review-1.md` 和 `phases/2-review-2.md`。
- 当前 residual `PBN` parser、`CLI` residual 路径、position solver 和 `CLI JSON` 输出。
- `DDS` 的 `SolveBoardPBN` 和 `AnalysePlayPBN` 语义。

文档检查结果：

- `git diff --check -- phases/2-api-service.md phases/2-review-1.md phases/2-review-2.md` 通过。
- 本轮审阅未修改现有代码或计划文档。

## Findings

### 1. Position Matrix 不应要求 `first`

当前 position matrix contract 要求合并后提供 `hands` 和 `first`，normalized command 也包含 `first`：

```rust
PositionMatrixAnalysis {
    hands: [Hand; 4],
    first: Direction,
}
```

这与已确认的 position matrix 输入规格冲突。position matrix 会分别计算四个 `next_to_act` 与五个 `strain` 的组合，初始 `first` 不参与结果计算，也没有领域意义。

建议：

- position matrix 的 `PBN` 只要求 `Position` tag。
- 不接受或要求 `First` tag。
- 从 `JSON body` 中移除 `first`。
- normalized command 改为：

```rust
PositionMatrixAnalysis {
    hands: [Hand; 4],
}
```

- position matrix application use case 内部为每一行设置对应的 `next_to_act`。

### 2. Continuation 的 residual `PBN` 映射与现有语义冲突

当前计划将 residual `PBN` 的 `[First]` 映射为 `trick_leader`。

现有 `Phase 1b` contract 和 `CLI` 中，[`First`] 表示 `next_to_act`。例如：

```pbn
[First "E"]
[CurrentTrick "N:SA"]
```

表示 `N` 首攻 `SA`，当前轮到 `E` 出牌。

如果将 `[First]` 改为 `trick_leader`，会破坏现有 `CLI` 输入兼容性，并使 mid-trick 位置无法仅通过现有 tags 正确表示。

建议保持现有语义：

- `[First]` 映射为 `next_to_act`。
- 非空 `[CurrentTrick]` 的第一项玩家映射为 `trick_leader`。
- `[CurrentTrick]` 中后续玩家必须按顺时针顺序排列。
- `[CurrentTrick]` 为空时，`trick_leader = next_to_act = First`。
- body 或 query 中显式提供的 `trick_leader`、`next_to_act` 按覆盖规则应用，并与最终 current trick 交叉验证。

### 3. Continuation Trick Count 字段语义自相矛盾

continuation response 同时定义：

- `score_side` 是 trick leader 所属一方。
- 每个建议牌的字段名是 `tricks_for_side_to_act`。

在 mid-trick 状态中，`next_to_act` 可能属于 trick leader 的对方，因此字段名与实际分数视角不一致。

建议将字段改为：

```json
{
  "tricks_for_score_side": 4
}
```

或者使用更明确但更长的：

```json
{
  "tricks_for_trick_leader_side": 4
}
```

推荐使用 `tricks_for_score_side`，因为 response 已明确包含 `score_side`。

应用层和 solver wrapper 中的 `CardResult.tricks_for_side_to_act` 也应同步改名，避免领域语义继续错误传播。

### 4. Play Response Contract 仍不完整且存在格式冲突

#### 4.1 缺少 `final_position`

play analysis 需要返回最终位置和最终 continuation。当前 response 只有 `trace` 和 `final_continuation`。

`final_continuation` 不包含剩余 `hands`，前端无法基于该结果继续出牌或重建完整状态。

建议增加：

```json
{
  "final_position": {
    "hands": {
      "N": ["..."],
      "E": ["..."],
      "S": ["..."],
      "W": ["..."]
    },
    "trick_leader": "N",
    "current_trick": [],
    "next_to_act": "N"
  }
}
```

`final_position` 必须使用已确认的用户快照语义，而不是 solver-facing remaining hands 语义。

#### 4.2 `trace` 数量描述与 `PlayEvaluation` 结构不兼容

计划称 `trace` 包含每张已出牌对应的一项，外加 initial state。

initial state 没有 `player` 和 `card`，无法使用当前 `PlayEvaluation` 结构表示。

建议使用以下契约：

- `trace` 恰好包含每张已出牌对应的一项。
- 第一项的 `tricks_before` 是 opening lead 前的初始 double-dummy 结果。
- 每项的 `tricks_after` 是该牌打出后的结果。

或者增加独立的 `initial_tricks` 字段。推荐第一种，结构更简单且不丢失信息。

#### 4.3 `current_trick` response 示例违反统一 `JSON` 表示

全局协议规定 `current_trick` 是牌字符串数组，例如：

```json
["SA"]
```

continuation response 示例却使用：

```json
["NSA"]
```

该值混入了玩家方向，与统一表示冲突。应改为纯牌字符串，并通过 `trick_leader` 推导玩家。

### 5. 输入覆盖规则仍存在逻辑矛盾

计划定义不同输入来源的优先级为：

```text
URL query > JSON body > PBN
```

但随后将 `JSON body` 中的 `dealer` 和 `PBN` 中的 `Dealer` 举例为同层冲突。

它们属于不同来源，应由 body 覆盖 `PBN`，而不是返回冲突错误。

建议明确区分：

- 不同来源中的同名字段：按优先级覆盖。
- 同一来源中的不同字段表达了相关语义但互相矛盾，例如 `declarer` 与 `opening_leader` 不一致：返回错误。
- 原始 `JSON` 或 query 中的重复 key：不保证检测，行为由底层 parser 决定。
- 不属于当前 endpoint 的 query 字段：返回 `400`，不能静默忽略。

### 6. 单个 `Semaphore` 无法同时实现单并发求解和有界等待队列

当前计划同时要求：

- `Semaphore` 使用 `1` 个 permit。
- 使用 `try_acquire()`。
- 支持有界等待队列。

单 permit `Semaphore` 配合 `try_acquire()` 时不存在等待队列。solver 忙碌时，新请求会立即返回 `503`。

需要明确选择 admission control 模型。

可选方案：

1. **无等待队列**：单 permit，solver 忙碌时立即返回 `503`。
2. **有界 worker 队列**：使用有界 `mpsc` queue 和单 solver worker。
3. **双层 semaphore**：一个 admission semaphore 限制 outstanding 请求数量，一个单 permit solver gate 串行执行。

推荐方案 `2`：

- 最符合当前全局串行 `DDS` 模型。
- 队列容量清晰。
- 队列满时可立即返回 `503`。
- 单 worker 明确保证一次只有一个 solver operation。
- 请求超时后可以丢弃 response receiver，但正在执行的任务仍会完成。

如果希望保持实现简单，也可以明确采用方案 `1`，但不能继续描述为有界等待队列。

### 7. `Phase 2a` 不应为 play response 返回虚假的空 `trace`

当前计划要求 `Phase 2a` 对 play 请求返回空 `Vec` 作为 `trace`，以保持 response shape。

空 `trace` 容易被调用方理解为 historical evaluation 已完成，但 play sequence 中没有牌，而不是该能力尚未实现。

建议：

- `Phase 2a` 实现和测试 play input normalization、advancement、final position 和 final continuation。
- 完整 `PlayAnalysis` response 和 `/api/v1/analyze/play` endpoint 在 `Phase 2b` 完成。
- `Phase 2a` 不返回伪造或不完整的公开 play response。

如果必须在 `Phase 2a` 暴露中间结构，应使用明确的内部类型，而不是公开 response `DTO`。

### 8. `null` 和未知 query 字段行为仍未定义

请求 schema 中所有字段均为 optional，但未定义显式 `null` 的行为：

```json
{
  "dealer": null
}
```

建议：

- 所有显式 `null` 返回 `400`。
- 仅通过省略字段表示该来源未提供值。
- 不允许使用 `null` 清除低优先级来源中的字段。
- 不属于当前 endpoint 的 query 字段返回 `400`。

这些行为应加入 transport `DTO` 和 input merging 测试。

### 9. 标记 `Phase 1b` 完成时必须明确重新划分 `AnalysePlayPBN`

计划要求在 `Phase 2a Task 1` 中将 `Phase 1b` 标记为完成。

当前 `PLAN.md` 仍将 `AnalysePlayPBN` 列为 `Phase 1b` 待完成任务，而新计划将其放入 `Phase 2b`。

更新 `PLAN.md` 时必须明确：

- `AnalysePlayPBN` historical evaluation 从 `Phase 1b` 重新划入 `Phase 2b`。
- `Phase 1b` 的完成范围是 position analysis、play trace parsing、合法性验证、状态推进和最终 continuation。
- 完成重新划分后，再标记 `Phase 1b` 完成。

## Recommended Revisions

建议按以下顺序修订 `phases/2-api-service.md`：

1. 从 position matrix contract 中移除 `first`。
2. 修正 continuation residual `PBN` 中 `[First]` 和 `[CurrentTrick]` 的映射。
3. 将 continuation trick count 字段改为与 `score_side` 一致的名称。
4. 为 play response 增加 `final_position`，修正 `trace` 数量定义和 `current_trick` 示例。
5. 修正输入覆盖与冲突规则。
6. 选择可实际实现的 admission control 模型。
7. 移除 `Phase 2a` 中虚假的空 `trace` 公开响应。
8. 定义显式 `null` 和未知 query 字段行为。
9. 明确 `Phase 1b` 与 `Phase 2b` 的 `AnalysePlayPBN` 范围重新划分。

## Approval Conditions

建议在满足以下条件后批准 `Phase 2a Task 1`：

- position matrix contract 只要求 `hands`。
- residual `PBN` 与现有 `[First]` 语义兼容。
- continuation 分数视角和字段名称一致。
- play response 包含可继续操作的 `final_position`，且 `trace` 契约无歧义。
- 输入覆盖、冲突、`null` 和未知 query 行为明确。
- admission control 模型可以按计划直接实现。
- `Phase 2a` 不产生不完整或误导性的公开 play response。

## Recommendation

计划已接近最终可执行状态。完成本文前六项主要修订，并明确后三项边界后，可以批准 `Phase 2a Task 1` 并开始实施。
