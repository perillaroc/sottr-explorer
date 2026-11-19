# ADR-0011 Python v0.2 只作差分回归参照

> 状态：已采纳（长期有效）。来源：[接口契约](/contracts)（Python 适配与差分回归）、
> [测试](../testing.md)。

## 背景

项目起点是 Python v0.2 研究实现，它已经在已知输入上产出了可复核的观测。Rust 实现在
重写过程中需要一把“已知输入的尺子”，但旧实现的结论不能直接升级为新契约的结论。

## 选项

| 选项 | 说明 | 结果 |
| --- | --- | --- |
| 仅作差分回归参照 | 用它的原始输出对照核心观测层 | **采纳** |
| 移植为唯一实现 / 替换 | 放弃独立参照，回归失去对照物 | 不采纳 |
| 把 `candidate_*` 升级为 `validated` / `verified_*` | 旧结论未经游戏内验证 | 禁止 |

## 决定

保留 `research/save-inspector/` 作为参照副本；Python 输出只进入契约的观测层
（`evidence_level = community_correlated`），`candidate_*` 不得因改名升级。

## 后果

- `cargo test -p sottr-core --test parity` 对照容器、哈希、区段、记录与伴随字、43 项计数、
  整体候选（float32 位模式）、参考差集与跨文件重复组。
- 夹具缺失时安静跳过并记录覆盖缺口，不记为通过；未解释的差异使回归失败。
- 设计内的强化行为（如异常 type28 导致表不完整）以带 `adjudication` 的差异记录，
  不要求复制旧的不安全输出。

## 关联文档

- [接口契约](/contracts)
- [测试](../testing.md)
- [研究记录](/data/research/)
