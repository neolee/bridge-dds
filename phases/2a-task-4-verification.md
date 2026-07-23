```bash
cd /Users/neo/Code/ML/bridge-dds
cargo build
```

## 1. 既有 full-deal 路径

```bash
cargo run --quiet -- solve --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Dealer "N"]
[Vulnerable "None"]
PBN
```

确认：

- `par.score` 为 `-110`
- `par.contracts` 为 `["2S-EW"]`
- 四个方向均包含五种 strain 的结果

## 2. Position matrix 路径

```bash
cargo run --quiet -- solve --matrix --format json <<'PBN'
[Position "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[First "N"]
PBN
```

确认：

- `row_semantics` 为 `next_to_act`
- `value_semantics` 为 `tricks_for_score_side`
- `values` 包含 `N/E/S/W`

## 3. Mid-trick continuation

```bash
cargo run --quiet -- solve --format json <<'PBN'
[Position "N:K9..Q8. T.T76.. 73.92.. AJ.J.6."]
[First "N"]
[Trump "D"]
[CurrentTrick "E:ST S:S3 W:SA"]
PBN
```

确认：

- `trump` 为 `D`
- `score_side` 为 `NS`
- `next_to_act` 为 `N`
- `current_trick` 为 `["ST","S3","SA"]`
- `S9` 为最优且得分 `3`
- `SK` 非最优且得分 `2`

## 4. Legacy `Play` 向后兼容

先执行带 prefix 的版本：

```bash
cargo run --quiet -- solve --trump S --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "E:S3=S5=S2=SQ"]
PBN
```

再执行无 prefix 的版本：

```bash
cargo run --quiet -- solve --trump S --declarer N --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "S3=S5=S2=SQ"]
PBN
```

确认两次 JSON 输出完全相同，并且：

- `next_to_act` 为 `N`
- `current_trick` 为空
- `score_side` 为 `NS`

## 5. 标准 `Play`、固定列和动态 leader

```bash
cargo run --quiet -- solve --trump S --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "E"]
S3 S5 S2 SQ
H7 H3 HA H2
- - C3 C8
PBN
```

这里固定列始终是 `E S W N`，但每墩 leader 会发生变化。

确认：

- 前两墩被正确推进
- 最终 `current_trick` 为 `["C3","C8"]`
- `next_to_act` 为 `E`
- `score_side` 为 `EW`

## 6. 不同 placeholder 旋转

```bash
cargo run --quiet -- solve --trump S --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "N"]
SQ S3 - -
PBN
```

确认：

- `current_trick` 为 `["SQ","S3"]`
- `next_to_act` 为 `S`

这可以确认 incomplete row 不要求 placeholder 固定出现在某一侧，而是根据实际出牌顺序解释。

## 7. `Contract` 和 `Declarer` fallback

不提供 `--trump` 或 `--declarer`：

```bash
cargo run --quiet -- solve --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Contract "4S"]
[Declarer "N"]
[Play "S3"]
PBN
```

确认：

- `trump` 为 `S`
- opening leader 从 declarer `N` 推导为 `E`
- `current_trick` 为 `["S3"]`
- `next_to_act` 为 `S`

## 8. 空 legacy `Play`

```bash
cargo run --quiet -- solve --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Contract "4S"]
[Declarer "N"]
[Play ""]
PBN
```

确认命令成功，并且：

- `current_trick` 为空
- `next_to_act` 为 `E`

## 9. 超过四张牌的连续 `=` sequence

```bash
cargo run --quiet -- solve --trump S --format json <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "E:S3=S5=S2=SQ=H2"]
PBN
```

确认分隔符没有被当作固定墩边界：

- 第一墩结束后由 `N` 获胜
- `H2` 成为下一墩第一张牌
- `current_trick` 为 `["H2"]`
- `next_to_act` 为 `E`

## 10. CLI 参数优先级

```bash
cargo run --quiet -- solve --trump S --declarer N <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Contract "4H"]
[Declarer "E"]
[Play "S3"]
PBN
```

确认 CLI 参数覆盖相应 PBN 字段：

- 输出 `Trump: S`
- opening leader 按 CLI declarer `N` 推导为 `E`
- `Current tricks: ES3`
- `Next to act: S`

## 11. Declarer/leader 冲突

```bash
cargo run --quiet -- solve --trump S <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Declarer "E"]
[Play "E:S3"]
PBN
```

应失败并输出：

```text
error: conflicting input: Play opening leader E does not follow declarer E
```

## 12. Case `10` parser-stage 顺序拒绝

```bash
cargo run --quiet -- solve --trump NT <<'PBN'
[Position "N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ"]
[First "S"]
[Trump "NT"]
[CurrentTrick "E:HA N:SA"]
PBN
```

应在 parser 阶段失败：

```text
error: invalid position: CurrentTrick: expected S as player 2, got N
```

## 13. 标准 `Play` 行形状错误

```bash
cargo run --quiet -- solve --trump S <<'PBN'
[Deal "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3"]
[Play "N"]
SA SK SQ
PBN
```

应失败，并指出标准 `Play` 行必须包含四个 token。

这套测试覆盖了 `Task 4` 的主要验收面。更完整的既有场景仍可参考 [1b-verification.md](/Users/neo/Code/ML/bridge-dds/phases/1b-verification.md)。
