# 命令行

CLI 与桌面端共用同一套核心分析管线，不重复实现语义。

## analyze

批量或单文件分析，输出 `analysis.json`（`BatchReport`）。

```text
sottr-cli analyze --input <文件或目录> --context <campaign|unknown> \
    --output <新目录> --metadata <元数据目录>
```

- 目录输入收集 `*.dat`（≤128 个，按名称排序）；单文件等价于单文件批量；
- `profile.dat` 始终按资料文件处理，`--context` 不覆盖该排除；
- `--context campaign` 是用户声明，不是格式识别证明；
- 输出目录必须尚不存在；单文件输入级错误写入对应结果条目，不影响其他文件；
- 退出码：全部成功 `0`；至少一个输入级错误 `1`（批量 JSON 仍写出全部结果）；用法或配置错误 `2`。

## export

导出当前报告为 JSON、CSV 或两者。

```text
sottr-cli export --input <文件> --context <campaign|unknown> \
    --output <新目录> --metadata <元数据目录> \
    [--format json|csv|both] [--share] \
    [--snapshot-id <ID>] [--snapshot-time <RFC3339>] \
    [--stale --refresh-error <CODE>]
```

- JSON 是导出信封；CSV 每行一项评估，列固定；
- `--share` 去掉输入目录并替换路径样片段；
- `--stale` 必须携带 `--refresh-error`，两者进入 JSON 与 CSV 的明确列；
- 输出目录必须不存在且与输入不重叠；先写临时文件，全部成功后再提交。

## plan

按区域生成收集计划。

```text
sottr-cli plan --catalog data/catalog/collectibles_catalog.json \
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]
```

输出 `plan.json`：每个区域的收集品清单、已收集/未收集、所属子关卡、关卡内相对位置与获取方式。
特殊区域单独成组，不计入主区域合计。

## missing

按类别回答“100% 必收还差哪些”。

```text
sottr-cli missing --catalog data/catalog/collectibles_catalog.json \
    --analysis <分析目录>/analysis.json --output <新目录> [--input save1.dat]
```

## 独立复核

批量输出可用 Node 脚本按 schema 复核：

```powershell
node apps/desktop/scripts/contract/validate-document.mjs --schema batch --file <路径>
```
