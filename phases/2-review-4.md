# `Phase 2` Review 4

## Status

本轮对 `phases/2-api-service.md`、历次 `Phase 2` review、现有代码、`Phase 1b` 验证用例，以及 `DDS` 文档和相关源码进行了非增量审阅。

第三轮 review 中的问题已经基本收束，但当前计划仍存在若干会影响 `mid-trick` 求解正确性、公开契约和阶段实施边界的问题。最重要的是明确用户快照到 `DDS` 输入的完整转换规则，并用精确结果测试锁定 `SolveBoardPBN` 语义。

审阅过程中未修改代码或 `phases/2-api-service.md`。

## Verified

- `cargo test --test dds_integration -- --test-threads=1 --nocapture` 通过，共 `15` 个集成测试。
- 现有 `mid-trick` 集成测试只验证返回牌的花色，没有验证受 `current_trick` 影响的精确分数。
- `DDS` 将 `dealPBN.first` 定义为当前牌墩的首攻者。
- `DDS` 根据 `dealPBN.first`、`currentTrick` 长度和剩余牌数推导实际下一位出牌者。
- `SolveBoardPBN` 的分数表示实际 `side to play` 的后续可得墩数。
- 当前 solver 同时将 `current_trick` 牌保留在 `remainCards`，并将 `dealPBN.first` 设置为 `next_to_act`。这会令 `DDS` 将 `mid-trick` 位置误判为干净牌墩边界并忽略 `currentTrick`；由于推导出的实际出牌者仍可能是 `next_to_act`，现有只检查返回牌花色的测试不会发现该问题。
- `git diff --check -- phases/2-api-service.md phases/2-review-4.md` 通过。

## Findings

### 1. 必须明确并测试用户快照到 `dealPBN` 的完整转换

计划已经要求从 `hands` 中移除 `current_trick` 牌，但没有明确 `dealPBN.first` 的映射。

正确转换必须同时满足：

- 从对应玩家的 `hands` 中移除所有 `current_trick` 牌。
- `dealPBN.first = trick_leader`。
- `dealPBN.currentTrickSuit` 和 `dealPBN.currentTrickRank` 按出牌顺序填充。
- `dealPBN.remainCards` 只包含尚未打出的牌。
- `DDS` 根据 `trick_leader` 和 `current_trick.len()` 推导出的实际下一位玩家必须等于 `next_to_act`。

现有实现中的两个确定错误会互相掩盖：

1. `current_trick` 牌仍被保留在 `remainCards` 中。
2. `dealPBN.first` 被错误地设置为 `next_to_act`，而不是 `trick_leader`。

由于四手仍然等长，`DDS` 会根据剩余牌数把当前位置推断为干净牌墩边界，从而忽略传入的 `currentTrick`。此时错误设置的 `first = next_to_act` 又恰好使 `DDS` 返回该玩家持有的合法牌，因此现有 CLI 输出和只检查返回牌花色的测试看起来正常，但求解实际没有纳入当前墩牌，精确分数和最佳牌可能错误。

这不是仅需在新 application layer 中注意的设计事项，而是 `Phase 2a` 必须修复的现有正确性问题。需要增加：

- 转换层 unit test：断言 `dealPBN.first = trick_leader`，并断言 `current_trick` 牌不在 `remainCards` 中。
- `DDS` integration test：使用当前墩牌会改变求解结果的牌例，断言精确分数和最佳牌，而不只是检查返回牌属于 `next_to_act`。
- CLI regression test：证明修复后的 CLI continuation 结果来自考虑了当前墩的正确位置。

### 2. `Continuation` 的分数视角应为实际 `side to play`

`SolveBoardPBN` 的结果是实际 `side to play` 的后续可得墩数。完成正确转换后，实际 `side to play` 就是 `next_to_act` 所在一方。

因此 continuation response 应定义为：

- `score_side` 是 `next_to_act` 所在一方。
- 建议牌字段可以使用 `tricks_for_score_side`，因为 response 已显式包含 `score_side`。
- 内部类型不应继续使用会暗示错误含义的名称或注释。

计划当前将 `score_side` 定义为 `trick_leader` 所在一方，需要修正。

现有 `phases/1b-verification.md` 已包含 `trick_leader` 与 `next_to_act` 分属对手的手工场景，但只验证输出标签和可出牌，未验证正确转换后的精确分数。需要增加自动化测试。

其中 case 8 本身还包含未被现有 parser 拒绝的玩家顺序错误：

```pbn
[First "S"]
[CurrentTrick "E:HA N:SA"]
```

如果 `E` 是 `trick_leader`，按顺时针顺序第二张牌应由 `S` 打出，而不是 `N`；两张牌后也不应轮到 `S`。这说明当前 parser 和验证只检查了牌张归属，没有完整验证 `CurrentTrick` 玩家顺序及其与 `First`/`next_to_act` 的一致性。`Phase 2a` 应修正该 verification case，并增加对应的拒绝测试。

### 3. 用户快照与内部运行时状态需要显式类型边界

公开用户快照要求：

- `hands` 包含 `current_trick` 牌。
- 四手张数相同。

内部 play advancement 和 solver state 则需要：

- 已打出的牌从 tracking hands 中移除。
- 未完成牌墩中四手张数可能暂时不同。

计划同时要求单一用户 `Position` 模型和从 tracking hands 删除牌，但没有明确两种状态如何隔离。建议定义独立类型，例如：

- `PositionSnapshot`：公开输入和输出模型。
- `PlayState`：内部状态推进模型。
- `SolverPosition`：已完成 `DDS` 转换的模型。

至少必须提供有明确不变量的显式转换函数，不能继续让同一个类型在不同路径中代表不同语义。

### 4. 建立并复用单一通用 `PBN` parser

当前存在多条相互独立的 `PBN` 处理路径：

- `parse_record()` 解析完整牌局的 `Deal`、`Dealer` 和 `Vulnerable`。
- `parse_residual_record()` 解析 residual position 的 `Position`、`First`、`Trump` 和 `CurrentTrick`。
- CLI play-trace 路径通过独立的字符串扫描提取 `Play` tag。

这种结构会使 tag 语法、重复 tag、未知 tag、partial record 和错误行为在不同功能间持续漂移。

`Phase 2a` 应建立一个通用的单 record `PBN` parser：

- 统一完成 tag line parsing、重复 tag 检测、未知 tag 保留或忽略策略，以及已支持 tag 的语法解析。
- 输出一个共享的 parsed record 类型，其中各字段可以缺省。
- endpoint、CLI 和后续 frontend import 都必须复用该 parser。
- endpoint-specific 层只负责选择允许的 tag、应用来源覆盖规则，以及验证合并后的必填字段和语义约束。
- 禁止 CLI、application 或 HTTP handler 再自行扫描或解析 tag 字符串。

通用 parser 不等于立即支持完整 `PBN` 标准；它仍然只实现项目明确支持的 subset，但所有入口必须共享同一个实现和同一套语义。

### 5. `/analyze/play` 的 `PBN` 支持范围需要明确

当前 `/analyze/play` contract 接受 `Deal`、`Dealer`、`Vulnerable` 和项目自定义的扁平 `Play` tag，但没有接受可推导 `trump` 和 `declarer` 的标准 `Contract`、`Declarer` 或标准 play section。现有 `CLI` 的 play-trace 路径也要求通过 `--trump` 提供将牌。

需要明确“完整 `PBN` string”具体表示：

1. 支持标准完整 play record，包括 `Contract`、`Declarer` 和 play section；或者
2. 仅支持项目定义的 partial/custom `PBN` subset，并要求通过 body/query 补充缺失字段。

如果目标是让用户直接粘贴常见完整 `PBN` 记录进行 play analysis，建议选择方案 `1`。

无论选择哪一种范围，相关 tag 都必须通过上一项定义的通用 `PBN` parser 解析。

### 6. `mpsc` worker 与 `spawn_blocking()` 的运行模型需要修正

计划称单一 solver worker 自身通过 `spawn_blocking()` 运行，但该 worker 还需要异步等待 `mpsc::Receiver`。

建议明确采用：

- 一个异步 worker 持有 `mpsc::Receiver`。
- worker 每次收到请求后，对该次同步 application/`DDS` 调用执行一次 `spawn_blocking()`。
- 调用完成后通过 `oneshot` 返回结果。
- worker 开始执行排队请求前，如果对应 `oneshot` receiver 已关闭，则跳过该请求。
- 已经开始的 `DDS` 调用无法因请求超时而取消。

同时明确队列容量、超时时长、worker 关闭和异常行为。

### 7. `final_position` 示例违反公开快照不变量

示例中的 `final_position.current_trick` 为空，但四手牌张数不一致。

最终位置转换应明确：

- 完整牌墩结束后，直接使用四手等长的剩余牌。
- 停在未完成牌墩时，将当前墩已经打出的牌加回对应玩家手中，以生成公开用户快照。
- 输出的四手牌张数始终一致。

### 8. `play` 输入验证契约不完整

公开契约还应明确：

- 强制遵守跟牌规则。
- 拒绝重复或再次打出同一张牌。
- `play` 长度范围为 `0..=52`，或者明确其他限制。
- 是否允许空 `play`。
- 完整 `52` 张牌后没有 continuation 时，response 如何表示。

### 9. `CLI JSON` 与 API 必须使用同一共享业务结果类型

`CLI JSON` 的目的就是提供与 API 一致的机器可读输出，因此不应保留独立的 JSON shape 或独立序列化实现。

共享 application use case 应返回同一个强类型业务结果，例如 deal、matrix、continuation 和 play analysis 的结果类型。CLI 和 HTTP adapter 都直接拿到该结果：

- CLI text formatter 根据该业务结果生成独立文本输出。
- CLI JSON 直接序列化与 API 相同的 response DTO 或同一个可序列化结果类型。
- HTTP handler 将同一结果作为 JSON response 返回。
- CLI 和 HTTP 不得分别重新拼装 JSON 字段或使用独立 JSON result 类型。

现有 CLI JSON 与计划中的 API JSON shape 不一致。例如 API matrix response 包含 `"matrix"` 外层字段，现有 CLI matrix JSON 没有该外层字段；continuation 也存在相同问题。因此 Phase 2a 应有意将 CLI JSON 迁移到最终 API spec，并更新旧 fixture。

验证要求应明确：

- 对同一规范化输入，CLI JSON 与 API response body 反序列化后必须完全相等。
- CLI JSON 和 API response 必须由同一 Rust 类型序列化产生。
- text fixture 在没有正确性修复时保持 byte-identical。
- 旧 CLI JSON fixture 不要求保持 byte-identical；应更新为最终 API spec。

### 10. 覆盖后仍必须执行最终语义一致性校验，并修正 declarer 推导

`declarer` 与 `opening_leader` 不是同一个字段：

- `declarer` 是定约者。
- `opening_leader` 是首攻者。
- 两者在 play analysis 中必须满足 `opening_leader == declarer.next()`。
- 从 `opening_leader` 反推时必须使用 `declarer = opening_leader.previous()`，而不是再次调用 `next()`。

不同来源可能分别提供这两个字段。例如 body 提供 `declarer = N`，而低优先级 `PBN Play` prefix 提供 `opening_leader = W`。由于字段名不同，覆盖规则不会移除任何一个值，但最终结果语义冲突，必须返回 `400`。

计划应明确：先按字段逐一应用来源优先级，再对最终合并结果执行所有跨字段语义校验。

`Phase 2b Task 1` 当前写成 `declarer = opening_leader.next()`，这是现有计划中的确定错误，需要修正为逆时针方向的 `previous()`。这与 mid-trick 转换错误一样，属于必须在 Phase 2 中明确修复并增加 regression test 的正确性问题。

需要增加：

- 四个方向的 `declarer -> opening_leader` 和 `opening_leader -> declarer` 推导测试。
- `declarer` 与 `opening_leader` 来自同一来源但不一致的错误测试。
- 二者来自不同来源、完成字段级覆盖后仍不一致的错误测试。
- `Play` prefix 单独提供 `opening_leader` 时，正确推导 `declarer` 的测试。

### 11. `PBN` tag 的 endpoint-specific 行为仍不明确

需要区分：

- 真正未知的 `PBN` tag：忽略。
- 已知但不适用于当前 endpoint 的 tag：忽略或返回 `400`，必须明确选择。
- 会派生多个字段的 tag，例如 `Contract` 或带 prefix 的 `Play`：其派生字段如何参与逐字段覆盖。

建议已知但不适用于当前 endpoint 的 tag 返回 `400`，避免用户误以为输入生效。

### 12. `Phase 2a Task 6` 的冲突描述与已确认规则不一致

Task 6 当前仍要求“拒绝同层不同表示的相同 key 冲突”，但已确认的模型实际是：

- 按字段执行 `query > body > PBN` 覆盖。
- 原始单一表示中的重复 key 不保证检测。
- 最终合并后执行跨字段语义一致性校验。

建议直接用以上三条替换 Task 6 当前的 conflict 描述，并为每条规则增加 transport tests。

### 13. `Phase 2a` 的 play 类型和验证范围仍存在阶段冲突

`Phase 2a` 不实现完整 historical evaluation，却仍定义完整 `PlayAnalysis` 并要求四个完整 application use case tests。

建议：

- `Phase 2a` 定义和测试 play normalization、advancement、`final_position` 和 `final_continuation` 所需的内部结果。
- 完整公开 `PlayAnalysis` 和 `analyze_play()` use case 在 `Phase 2b` 完成。
- `Phase 2a` verification 改为三个完整 use case，加上 play advancement/final-state tests。

### 14. 稳定错误码契约尚未完整定义

计划只展示了 `invalid_position` 示例，没有定义稳定错误码集合，也没有定义 domain、transport、extractor、queue、timeout 和 internal error 到错误码的映射。

建议在 Task 1 定义最小稳定错误码集合，并在 `Phase 2b` 对每类映射增加测试。

### 15. 稳定输出顺序尚未定义

需要明确：

- `hands` 中牌张按何种顺序序列化。
- `suggested` 按 score、optimal 和 card 如何排序。
- equivalent cards 展开后的顺序。
- `par.contracts` 是否保留 `DDS` 顺序。

否则 API response、`CLI JSON` 和 golden fixtures 可能发生非语义性漂移。

### 16. `par.score` 的公开视角未定义

现有领域类型将 `par.score` 定义为 `NS` 视角：正数表示 `NS` 得分，负数表示 `EW` 得分。

API response 示例只给出数值，没有写明该符号语义。应在 deal response contract 中明确：

```text
par.score is from the NS perspective: positive means an NS gain, negative means an EW gain.
```

### 17. 公开字符串输入的严格程度未定义

需要明确 JSON/query 是否仅接受文档中的规范形式，还是同时接受别名和大小写变体，例如：

- `NT` 与 `nt`。
- `All` 与 `Both`。
- `None` 与 `Love`。
- 牌张字符串的小写形式。

建议公开 JSON/query 只接受规范形式；`PBN` parser 可以继续接受其契约明确列出的别名。

### 18. Matrix request schema 与 validation rule 不一致

Matrix request schema 不包含 `current_trick` 字段，但 validation rule 称其可以 absent 或 empty。

建议只允许 absent，并将任何 body `current_trick` 视为未知字段返回 `400`。Matrix application command 本身不应包含该字段。

### 19. Server 配置缺少可验证的默认值

计划要求测试 queue overload、timeout、body limit 和 restricted `CORS`，但没有定义默认值或配置入口。

建议在 Task 3/4 明确：

- 默认 queue capacity。
- 默认 request timeout。
- 默认 body size limit。
- 允许的开发期 `CORS` origin。
- 测试中如何覆盖这些配置。

### 20. Application command 与结果类型命名容易混淆

`FullDealAnalysis`、`ContinuationAnalysis` 和 `PlayAnalysis` 看起来更像结果类型。

建议 command 使用 `AnalyzeDeal`、`AnalyzeContinuation`、`AnalyzePlay`，或使用 `*Request` 后缀；结果类型保留 `*Analysis`。

### 21. Matrix request 示例不是合法 `JSON`

`hands` 字段后包含尾随逗号，应删除。

## Approval Conditions

建议至少完成以下事项后批准 `Phase 2a Task 1`：

1. 明确 `PositionSnapshot` 到 `SolverPosition` 的转换规则，包括 `dealPBN.first = trick_leader`。
2. 修正 continuation 分数视角，并增加受 `current_trick` 影响的精确结果测试。
3. 明确用户快照、内部推进状态和 solver 状态的类型边界。
4. 建立所有入口必须复用的单一通用 `PBN` parser，并决定 `/analyze/play` 对完整标准 `PBN` record 的支持范围。
5. 让 application、CLI JSON 和 API response 复用同一共享业务结果类型。
6. 修正 `declarer` 与 `opening_leader` 的反向推导错误并增加 regression tests。
7. 修正 worker/`spawn_blocking()` 运行模型。
8. 修正公开 response 示例、play validation、覆盖后语义校验和 Task 6 描述。
9. 明确 `Phase 2a` 与 `Phase 2b` 的 play result/use-case 边界。

## Recommendation

当前计划的总体架构仍然合理，但不应在完成上述正确性和契约修订前开始实现。特别是 `mid-trick` 转换与分数视角必须先由自动化精确结果测试锁定，避免现有两个互相掩盖的错误被带入共享 application layer 和公开 API。
