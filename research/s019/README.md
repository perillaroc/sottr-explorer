# SX-019 研究记录：游戏资源、网络来源与本地化关联

游戏安装数据的本地只读提取与关联研究。产出为事实、假说与未解问题清单；未证实结论
不改产品状态，元数据并入按方案 G4 出口在 SX-020 完成后执行。

## 环境与输入

| 项 | 值 |
| --- | --- |
| 游戏 | Shadow of the Tomb Raider（Steam appid 750920，Definitive Edition，PC） |
| 安装目录 | `G:\SteamLibrary\steamapps\common\Shadow of the Tomb Raider` |
| 数据文件 | `bigfile.000-011.tiger`、`bigfile.update1.002.*`、`bigfile.update2.002.*`、`bigfile.update3.002.*`；各语言 `*_allvo_allplt`、`*_alltxt_*` 家族 |
| 关键文件 | `pcx64-w\globalcollectibleinfo.drm`、`pcx64-w\globalgameflow.drm`、`pcx64-w\local\locals.bin` |
| 观察日期 | 2026-10-17 |
| 只读性 | 全程只读打开；未修改任何游戏文件（未做前后哈希对比，因仅 `File.OpenRead` 语义） |

加载优先级（实测）：同名文件在 `bigfile` → `update1` → `update2` → `update3` 中按后者优先；
DRM 内的资源位置表带 `archiveId`，部分资源仍指回基础归档（1=bigfile、10=update1、
11=update2、12=update3）。**只读基础归档会得到过时内容**。

## 格式（独立实现）

`scripts/sottr_dtp.py` 为独立实现（未复制任何第三方源码），与两个公开工具的读取器交叉
核对过容器结构：

- TIGER v5：`TAFS` 头（version=5、numParts、numFiles、id、subId、platform[0x20]）+
  32 字节条目（u64 nameHash、u64 locale、u32 size1、u32 size2、i16 part、u8 archiveId、
  u8 archiveSubId、u32 offset）。文件路径哈希 = FNV-1a 64（与公开工具一致，本仓库实测命中）。
- CDRM 压缩容器：`CDRM` 魔数 + u32 type + u32 numChunks + u32 保留；块表
  `{u32 uncompressedSize<<8, u32 compressedSize}`；块数据按 16 字节对齐；
  `compressedSize == uncompressedSize` 为原样，否则跳过 2 字节后为 raw deflate。
- DRM v23（资源集合）：i32 version=23，头部 8×i32/u64（include/dependency 长度、size、
  flags、numResources、mainResourceIndex、locale）；随后 numResources×24 字节识别表
  （u32 bodySize、u8 type、u8 flags、i16 pad、u32 subTypeAndRefDefs、i32 id、u64 locale）、
  依赖/包含字符串、numResources×24 字节位置表（i32 uniqueKey=(type<<24|id)、i32 pad、
  i16 part、u8 archiveId、u8 archiveSubId、u32 offset、u32 size、u32 offsetInBatch）。
  资源负载长度 = refDefinitionsSize + bodySize；等于 size 为原样存储，否则为 CDRM。
- DTP 对象图：RefDefinitions 头（5×i32 计数 + 内部/宽外部引用表、补丁表、压缩外部引用表），
  主体内的指针按引用表解析（内部引用 targetOffset 相对主体；宽/压缩外部引用 refOffset
  以 32 位字为单位）。字符串为 NUL 结尾。
- `locals.bin`：i32 languageId、i32 numStrings、numStrings×8 字节键偏移表；
  条目为 `键 文本\0`（UTF-8）。键命名如 `regionname_*`、`<名称>_title_<guid>`。

对象布局按公开二进制模板（`tr11globalcollectibleinfo`、`tr11globalgameflow`、
`trobjectcommon`）实现，并用 DTP 内部引用表反推校验：
RegionData 步长 256、MapData 272、CollectibleStory 168、StoryCollectibleItem 496、
NonStoryCollectibleItem 240。注意：**更新后的 DTP 与模板版本存在差异**
（MapData 尾部无 FSkip 等）；以引用表实测为准。

## 脚本

| 脚本 | 作用 |
| --- | --- |
| `scripts/sottr_dtp.py` | 独立容器/对象图读取库（TIGER/CDRM/DRM/DTP、FNV-1a 64、归档注册表） |
| `scripts/extract_collectibleinfo.py` | 提取四个归档家族中的 `globalcollectibleinfo.drm`，逐家族对比 DTP 负载摘要 |
| `scripts/parse_collectibleinfo.py` | 解析 GlobalCollectibleInfo → 非故事收集品（组/条目全字段） |
| `scripts/extract_gameflow_locals.py` | 提取 `globalgameflow.drm`（26 区域、94 单位地图）与 `locals.bin`（英/简中） |
| `scripts/crosscheck_saves.py` | 用 CLI 分析输出（观察 ID）与 DTP 全集、866 名称字典交叉对账 |

运行示例（输出目录应在仓库外，如 `private/s019_work`）：

```powershell
py -3 scripts/extract_collectibleinfo.py "<游戏目录>" "<工作目录>/extracted"
py -3 scripts/parse_collectibleinfo.py "<工作目录>/extracted/dtp_0360E8.bin" "<工作目录>/extracted/collectibleinfo_update3.json"
py -3 scripts/extract_gameflow_locals.py "<游戏目录>" "<工作目录>/extracted"
& repo\sottr-explorer\target\debug\sottr-cli.exe analyze --input "<存档目录>" --context campaign `
    --output "<工作目录>/cli_analyze" --metadata repo\sottr-explorer\data\metadata
py -3 scripts/crosscheck_saves.py "<工作目录>/cli_analyze/analysis.json" "<工作目录>/extracted/collectibleinfo_update3.json"
```

## 来源与许可（本条研究）

| 来源 | 用途 | 许可状态 |
| --- | --- | --- |
| [TrRebootModTools](https://github.com/arcusmaximus/TrRebootModTools) `a79c48b75f1ffaaf623dc73a625211bf98ff0504`（2026-06-25） | 二进制模板（对象布局参考）、`tr11zones`/`tr11collectibles` 列表模板、文档 | MIT；模板为格式事实参考，未复制源码 |
| [CDCE.TIGER.Tool](https://github.com/Ekey/CDCE.TIGER.Tool) `4c02ae12d83894b348f49d094bbdee7fbf45526d`（2025-08-07） | 容器格式读取器交叉核对（未能构建，仅源码阅读） | **未声明许可**；仅本地研究参考，未复制任何源码进入本仓库或产品 |
| 本地游戏资源（tiger/drm/locals.bin） | 提取事实性映射 | 商业资源；原始素材仅在本地读取、不随产品分发；事实性映射的发布按 G4 出口门禁逐类复核 |

## 结果去向

- 可复现脚本与结论：本目录（仓库内，无商业数据转储）。
- 提取数据、映射候选表、验证日志：`private/s019_work/`、`private/s019_verification.log`（仓库外，按门禁未并入产品）。
- 产品元数据并入：按方案 G4 出口，在 SX-020 完成后统一执行。
