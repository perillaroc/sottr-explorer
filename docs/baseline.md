# 研究基线复现与封存（SX-001）

- 日期：2026-10-13
- 状态：已完成
- 研究包：`plan/SOTTR_Save_Research`（对应交接包 SOTTR_Save_Research_v0.2，保持原样未修改）
- 私有输出：`private/baseline/`（仓库外；含绝对路径与存档字符串，不得外发）
- 路径说明：本文档所有路径均相对开发工作区根目录 `sottr-explorer-devel`。

## 环境

| 项目 | 值 |
| --- | --- |
| 操作系统 | Microsoft Windows 11 企业版，Build 26200 |
| Python | 3.10.5（MSC v.1929，64-bit），命令 `py -3` |
| 字节码控制 | 所有命令设置 `PYTHONDONTWRITEBYTECODE=1`，避免在研究包内产生 `__pycache__` |
| 夹具 | 研究包 `samples/` 五文件：profile.dat、save1.dat、save202.dat、save204.dat、save206.dat |

## 1. 包校验（分析前后各一次）

命令：

```powershell
py -3 private/baseline/records/verify_package.py --package plan/SOTTR_Save_Research --out private/baseline/records/package_verification_before.json
py -3 private/baseline/records/verify_package.py --package plan/SOTTR_Save_Research --out private/baseline/records/package_verification_after.json
```

结果：两次均为 `PACKAGE_SHA256.json` 59/59 项一致、`original_hashes.json` 5/5 样本一致（尺寸 + SHA-256），无 mismatch、无 missing。

含义：测试与复跑过程未改动研究包任何文件；样本哈希与交付清单一致，可作为后续 Rust 差分回归的输入基线。

备注：`PACKAGE_SHA256.json` 不含自身；`tool/README.md` 与 `tool/README_zh.md` 内容相同（哈希一致），属包内已知状态。

## 2. 测试复现

命令（在 `plan/SOTTR_Save_Research` 目录执行）：

```powershell
$env:SOTTR_SAMPLE_DIR = (Resolve-Path "samples").Path
$env:PYTHONDONTWRITEBYTECODE = "1"
py -3 -m unittest discover -s tool\tests -v
```

结果：`Ran 29 tests in 4.042s`，`OK (skipped=1)` → 28 通过、1 跳过、0 失败。跳过项 `test_public_provenance_samples`（公开夹具经 `SOTTR_PUBLIC_DIR` opt-in，未随包分发）。

日志：`private/baseline/logs/s001_tests_20261013.log`。

与历史记录一致性：`tool/test_results.txt` 为 29 项全通过（当时启用公开夹具）；2026-10-11 复测为 28 通过 + 1 跳过；本次复跑与 10-11 记录一致。

私人夹具测试 `test_private_real_fixtures` 通过，确认：profile.dat 解压 25,600 字节；4 个槽位均 4,194,304 字节且 zlib EOF、无尾随；save204/save206 压缩哈希相同；save1/save202 解压后仅 9 字节不同。

## 3. 五文件分析复跑

命令（在 `plan/SOTTR_Save_Research` 目录执行）：

```powershell
py -3 tool\sottr_inspect.py --input samples --output ..\..\private\baseline\analysis01 --catalog tool\data\collectible_id_candidates.json
```

结果：退出码 0；输出到全新目录 `private/baseline/analysis01`（不覆盖包内 `tool/release_original_report`）。产物：report.json、present_item_ids.json/.csv、reference_presence_differences.json/.csv、summary.txt、research.log、invocation.json。

关键业务值（community_correlated 候选层）：

| 文件 | 整体候选（float32 原值） | 摘要显示 | 记录数（前缀+溢出） | 唯一 ID | 重复记录 |
| --- | --- | --- | --- | --- | --- |
| save1.dat | 66.20481872558594 | 66.204819% | 512 + 5 = 517 | 517 | 0 |
| save202.dat | 66.20481872558594 | 66.204819% | 512 + 5 = 517 | 517 | 0 |
| save204.dat | 65.96385192871094 | 65.963852% | 512 + 4 = 516 | 516 | 0 |
| save206.dat | 65.96385192871094 | 65.963852% | 512 + 4 = 516 | 516 | 0 |

- profile.dat 仅容器观察（expanded 25,600 字节，valid = true），不进入战役语义
- 参考差集（四个内部类别合计）：save1/save202 为 12 项（SurvivalCache 9 + RareResourceCache 1 + Monolith 1 + CryptTreasure 1）；save204/save206 为 13 项（SurvivalCache 10 + RareResourceCache 1 + Monolith 1 + CryptTreasure 1）
- 区域计数：43 项稀疏映射全部产出，无超源目标异常
- `integrity.all_input_hashes_unchanged = true`；两两比较 6/6；重复压缩组：save204.dat 与 save206.dat

## 4. 与包内历史输出对比（tool/release_original_report）

命令：

```powershell
py -3 private/baseline/records/compare_with_release.py --rerun private/baseline/analysis01 --release plan/SOTTR_Save_Research/tool/release_original_report --out private/baseline/records/release_comparison.json
```

结果：业务差异 0。

- report.json 业务指纹（解压状态、压缩/解压哈希、整体候选、517/516 条记录序列、43 项区域计数、参考差集 ID 集合、重复组、6 对比较）零差异
- present_item_ids 与 reference_presence_differences（JSON/CSV）按文件名规范化后一致
- summary.txt 与 research.log 按行尾规范化后一致
- 预期差异仅：绝对路径字段、invocation.json 的 inputs/output、Windows CRLF 与历史 LF 行尾

## 5. review/ 独立测试检查（未复跑）

检查 `review/test_independent_review.py`：模块级 `ROOT = parents[3]`、`TOOL = ROOT/'sottr/phase2/tool_v02'`；corpus 测试硬性要求 24 个战役文件（含 20 个未分发公开样本）；mirror 测试需要公开 Cozumel 样本。

结论：依赖原研究工作区布局与公开夹具，不可开箱运行；本任务不声称复跑独立测试。历史记录（`review/independent_test_results.txt`，2026-10-10）：13 项独立测试通过。

## 6. 产物清单（私有）

| 路径 | 说明 |
| --- | --- |
| `private/baseline/analysis01/` | 五文件复跑输出；含绝对路径与存档字符串，禁止外发 |
| `private/baseline/records/verify_package.py` | 包校验脚本（仅标准库） |
| `private/baseline/records/package_verification_before.json` / `_after.json` | 分析前后两次校验记录 |
| `private/baseline/records/compare_with_release.py` | 历史输出业务对比脚本 |
| `private/baseline/records/release_comparison.json` | 对比明细 |
| `private/baseline/records/make_manifest.py` / `baseline_manifest.json` | 产物清单与哈希 |
| `private/baseline/logs/s001_*.log` | 包校验、测试、分析、对比日志 |
| `private/baseline/baseline.md` | 本记录的私有原件；仓库副本见 `docs/baseline.md` |

## 7. 限制与未决项

- 公开 20 个样本未随包分发：公开夹具测试 skip；不得声称重现 24 份战役样本
- review/ 13 项独立测试未复跑（依赖原路径与公开夹具，见上节）
- Windows 存档目录发现逻辑历史上未经实机验证（Py v0.2 已知限制）
- Py v0.2 只读保护为尽力而为；正式 Windows 实现按 SX-012/SX-013 处理稳定快照与竞态
- 本记录不新增字段语义；517/516 为原始观察集合，不是收集品总数

## 8. 结论

SX-001 完成条件满足：哈希核对无冲突（前后两次 59/59 + 5/5）；已有可运行测试无失败（28 通过、1 明确 skip）；五文件分析复现，与历史输出业务零差异且全部差异有解释；原件未被修改。
