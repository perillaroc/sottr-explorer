# 收集品元数据

`data/metadata/` 是版本化的收集品元数据，应用只读加载，**不执行其中任何内容**，也不自动联网更新。

## 文件

| 文件 | 内容 |
| --- | --- |
| `collectibles.json` | 866 个内部 ID 条目（候选名称全部保留为 `aliases`）与 9 个类别 |
| `region_counters.json` | 9 个区域与 43 项稀疏区域计数映射 |
| `reference_sets.json` | 参考集合与 3 个圣胡安攻略组 |
| `manifest.json` | 输入与输出内容哈希清单 |

三个数据文件作为**一组**加载：单独加载其中一个会因跨文件引用未解析而失败。

## 名称策略

- `aliases` 是内部候选名称，不是游戏显示名称；
- `display_name` 在无来源本地化名称时为 `null`；
- 人工简名以 `name_kind: "curated_short"` 标明，不伪装成游戏名称；
- 中文名未知就保持 `null`。

## 计数与参考

- 区域计数是 43 项稀疏映射，键为 `counter:<region>:<category>`；缺项不是 0；
- 偏移必须满足 `0x10BC + 行号 × 104 + 相对偏移` 恒等式；
- 计数可合并多个参考类别（如贮藏点 77 = 65 + 12），通过 `counts_toward` 声明；
- 参考集合来自两位作者标注 100% 的参考存档，记录压缩文件 SHA-256，存档字节不随产品分发。

## 来源

- 内部 ID 与候选名称：TrRebootModTools（MIT，固定提交）；
- 区域计数偏移与目标：ASL-SOTTR 的事实转录与独立比对；
- 参考集合与攻略组：Stella 攻略站的链接与事实，不打包内容。

详见[来源与许可](/data/sources)。

## 校验

```powershell
py -3 data/metadata/build_metadata.py          # 重新生成
py -3 data/metadata/build_metadata.py --check  # 校验现有文件与 manifest 一致
cargo test -p sottr-core --test metadata_dataset
```

加载器拒绝重复键、`NaN`/`Infinity`、非 http/https 链接、映射键与条目 ID 不一致、偏移恒等式
不符与跨文件冲突，并记录每个文件的 SHA-256。

完整说明见
[data/metadata/README.md](https://github.com/perillaroc/sottr-explorer/blob/main/data/metadata/README.md)。
