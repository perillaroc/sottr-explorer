# 研究基线复现与封存（SX-001）

- 日期：2026-10-13
- 状态：已完成
- 研究包：SOTTR_Save_Research v0.2（交接包，保持原样未修改）
- 私有输出：开发工作区中的独立目录（不随仓库或站点分发）

::: tip 公开整理版
本文删除了开发工作区的私有路径，命令中的目录以占位符表示；结果与结论未改动。
:::

## 环境

| 项目 | 值 |
| --- | --- |
| 操作系统 | Microsoft Windows 11 企业版，Build 26200 |
| Python | 3.10.5（MSC v.1929，64-bit），命令 `py -3` |
| 字节码控制 | 所有命令设置 `PYTHONDONTWRITEBYTECODE=1`，避免在研究包内产生 `__pycache__` |
| 夹具 | 研究包 `samples/` 五文件：profile.dat、save1.dat、save202.dat、save204.dat、save206.dat |

命令中的两个占位符：

```powershell
$PACKAGE = "<研究包目录>"      # 交接包根目录
$PRIVATE = "<私有输出目录>"    # 仓库外，含绝对路径与存档字符串，不得外发
```

## 1. 包校验（分析前后各一次）

```powershell
py -3 $PRIVATE/records/verify_package.py --package $PACKAGE --out $PRIVATE/records/package_verification_before.json
py -3 $PRIVATE/records/verify_package.py --package $PACKAGE --out $PRIVATE/records/package_verification_after.json
```

结果：两次均为 `PACKAGE_SHA256.json` 59/59 项一致、`original_hashes.json` 5/5 样本一致
（尺寸 + SHA-256），无 mismatch、无 missing。

含义：测试与复跑过程未改动研究包任何文件；样本哈希与交付清单一致，可作为后续 Rust 差分回归的输入基线。

备注：`PACKAGE_SHA256.json` 不含自身；`tool/README.md` 与 `tool/README_zh.md` 内容相同
（哈希一致），属包内已知状态。

## 2. 测试复现

```powershell
cd $PACKAGE
$env:SOTTR_SAMPLE_DIR = (Resolve-Path "samples").Path
$env:PYTHONDONTWRITEBYTECODE = "1"
py -3 -m unittest discover -s tool\tests -v
```

结果：`Ran 29 tests in 4.042s`，`OK (skipped=1)` → 28 通过、1 跳过、0 失败。
跳过项 `test_public_provenance_samples`（公开夹具经 `SOTTR_PUBLIC_DIR` opt-in，未随包分发）。

与历史记录一致性：`tool/test_results.txt` 为 29 项全通过（当时启用公开夹具）；
2026-10-11 复测为 28 通过 + 1 跳过；本次复跑与 10-11 记录一致。

私人夹具测试 `test_private_real_fixtures` 通过，确认：profile.dat 解压 25,600 字节；
4 个槽位均 4,194,304 字节且 zlib EOF、无尾随；save204/save206 压缩哈希相同；
save1/save202 解压后仅 9 字节不同。

## 3. 五文件分析复跑

```powershell
py -3 $PACKAGE/tool/sottr_inspect.py --input samples --output $PRIVATE/analysis01 `
    --catalog $PACKAGE/tool/data/collectible_id_candidates.json
```

结果：退出码 0；输出到全新目录（不覆盖包内 `tool/release_original_report`）。产物：
report.json、present_item_ids.json/.csv、reference_presence_differences.json/.csv、
summary.txt、research.log、invocation.json。

关键业务值（community_correlated 候选层）：

| 文件 | 整体候选（float32 原值） | 摘要显示 | 记录数（前缀+溢出） | 唯一 ID | 重复记录 |
| --- | --- | --- | --- | --- | --- |
| save1.dat | 66.20481872558594 | 66.204819% | 512 + 5 = 517 | 517 | 0 |
| save202.dat | 66.20481872558594 | 66.204819% | 512 + 5 = 517 | 517 | 0 |
| save204.dat | 65.96385192871094 | 65.963852% | 512 + 4 = 516 | 516 | 0 |
| save206.dat | 65.96385192871094 | 65.963852% | 512 + 4 = 516 | 516 | 0 |

- profile.dat 仅容器观察（expanded 25,600 字节，valid = true），不进入战役语义；
- 参考差集（四个内部类别合计）：save1/save202 为 12 项（SurvivalCache 9 + RareResourceCache 1 +
  Monolith 1 + CryptTreasure 1）；save204/save206 为 13 项（SurvivalCache 10 + 其余相同）；
- 区域计数：43 项稀疏映射全部产出，无超源目标异常；
- `integrity.all_input_hashes_unchanged = true`；两两比较 6/6；重复压缩组：save204.dat 与 save206.dat。

## 4. 与包内历史输出对比

```powershell
py -3 $PRIVATE/records/compare_with_release.py --rerun $PRIVATE/analysis01 `
    --release $PACKAGE/tool/release_original_report --out $PRIVATE/records/release_comparison.json
```

结果：业务差异 0。

- report.json 业务指纹（解压状态、压缩/解压哈希、整体候选、517/516 条记录序列、43 项区域计数、
  参考差集 ID 集合、重复组、6 对比较）零差异；
- present_item_ids 与 reference_presence_differences（JSON/CSV）按文件名规范化后一致；
- summary.txt 与 research.log 按行尾规范化后一致；
- 预期差异仅：绝对路径字段、invocation.json 的 inputs/output、Windows CRLF 与历史 LF 行尾。

## 5. review/ 独立测试检查（未复跑）

检查研究包中的独立测试：模块级 `ROOT = parents[3]`，工具路径指向原研究工作区布局；corpus 测试
硬性要求 24 个战役文件（含 20 个未分发公开样本）；mirror 测试需要公开 Cozumel 样本。

结论：依赖原研究工作区布局与公开夹具，不可开箱运行；本任务不声称复跑独立测试。
历史记录（2026-10-10）：13 项独立测试通过。

## 6. 私有产物（不随仓库或站点分发）

复跑过程在开发工作区产生了以下私有产物，均含绝对路径或存档字符串，禁止外发：

- 五文件复跑输出（report.json、ID 清单、参考差集、摘要、日志、调用参数）；
- 包校验前后记录、历史输出对比明细、产物清单与哈希；
- 包校验与对比脚本、测试与分析日志；
- 本记录的私有原件。

仓库与站点只保留方法、结果与结论。

## 7. 限制与未决项

- 公开 20 个样本未随包分发：公开夹具测试 skip；不得声称重现 24 份战役样本；
- 独立测试未复跑（依赖原路径与公开夹具，见上节）；
- Windows 存档目录发现逻辑历史上未经实机验证（Py v0.2 已知限制）；
- Py v0.2 只读保护为尽力而为；正式 Windows 实现按平台层处理稳定快照与竞态；
- 本记录不新增字段语义；517/516 为原始观察集合，不是收集品总数。

## 8. 结论

SX-001 完成条件满足：哈希核对无冲突（前后两次 59/59 + 5/5）；已有可运行测试无失败
（28 通过、1 明确 skip）；五文件分析复现，与历史输出业务零差异且全部差异有解释；原件未被修改。
