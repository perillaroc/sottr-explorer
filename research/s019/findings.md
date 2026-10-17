# SX-019 研究发现

依据：本地游戏数据（2026-10-17，更新至 `bigfile.update3.002`）提取，脚本见
`scripts/`。完整提取数据与验证日志在仓库外 `private/s019_work/`。

## 事实（已复核）

1. **更新优先级生效**：`globalcollectibleinfo.drm` 在 `bigfile`、`update1`、`update2`、
   `update3` 各有副本；其主 DTP 负载四者互不相同（sha256 前 16 位：
   `7ded848b3ba76a19` / `75de39f60f82eff5` / `f36f441ef78fce07` / `44e224bc7f2128aa`），
   游戏加载 update3 版本。基础归档解析会得到过时内容。
2. **非故事收集品全集**（update3 DTP，id `0x360E8` 资源哈希 `0x000360E8`）：
   14 个组、173 个条目结构、**168 个唯一 ID**；一个 ID 出现 5 次（`0xC9D62DEE`）、
   一个出现 2 次（`0x98564B9D`）；无 0 值 ID。
3. **type 分布**：2:81、9:17、5:15、7:19、4:12、6:10、11:9、8:8、1:2。
   按 metricName 前缀可见：2=贮藏点（65 SurvivalCache + 16 RareResource/Gym）、
   4=石碑（12 Monolith）、5=探险者尸包（15 ExplorerSatchel）、6=墓穴宝藏（10 CryptTreasure）、
   7=藏宝图/档案图（19）、8=锁箱（8 lockpick/Strongbox）、9=挑战古墓/训练（17）、
   11=墓穴入口（9）、1=测试项（2 Document_Test）。类别命名属观察归纳，待 SX-020 语义确认。
4. **地图标记图标**（mapMarkerIcon）：SurvivalCache 81、DocumentChest 19、ExplorerCorpse 15、
   Codex 12、Monoliths 12、LostItem 10、CryptEntrance 9、Strongbox 8、Document 2。
   与 type 分布一致（81/19/15/12/12/10/9/8/2）。
5. **字段结构**：每个条目含 32 位 ID、metricName（字符串）、type、regionId（32 位）、
   unit（层级/关卡 ID）、placement（listType 167/168 为 `0xF787272F`=OBJECTREF 类哈希、
   zoneId=unit、id0 64 位对象哈希、id1 通常为 0；1 条为 `0xFFFFFFFF` 空 placement）、
   coordinateX/Y、instanceZ、elevation、hasAreaMarker、area* 字段组、mapReveal*、
   revealedObject*、selectedDescription、doNotShowOnMap。
6. **区域表**（globalgameflow，update3）：26 个区域；收集品引用的 regionId 9 个全部命中
   区域表（26694CD8、838BAE8C、8C0D7019、8D2E7B5C、B02CF2FD、B06345B4、C5B30CDB、
   C92877A7、D65AE5D8）；区域记录含 regionLocalName/regionOverrideName（64 位键，未解析，
   见未知项）、coordinateX/Y（样本 0.5/0.5）、uiUnitID、dlcHash、进出统计等。
   94 个单位地图（unitMapHashedString、uiUnitID、markerUnit、primaryRegion、secondary）。
7. **单位（unit）命名**：47 个不同 unit；经公开列表模板（MIT）解析出层级名，例如
   243=`pa_hub_paititi`、241=`pa_hub_paititi_return_02`、318=`sm_spanish_mission_village`、
   2468=`ct_serpents_hollow`、980=`ct_citadel_of_wind`、3417=`ct_tree_of_life`、
   4907=`gym_outcasts`、4908=`gym_social_hub`。
8. **存档交叉验证**（CLI 观察，5 个私人战役槽位）：
   save1 观察 517 个 ID；与 DTP 168 唯一 ID 的交集为 **129**；全部 5 个存档的并集同样为
   517（交集 129）。39 个 DTP ID 未在任一存档出现——含 2 个测试项（Document_Test）、
   训练场（gym_social_hub_codex×4、CT 相关若干）、DLC 图若干、San Juan 未收集若干。
   按 type：2:67/81、4:11/12、5:11/15、6:9/10、7:12/19、8:7/8、9:4/12、11:8/9（已观察/全集）。
9. **名称字典覆盖**：168/168 个 ID 均命中仓库既有 866 字典（MIT，
   `Templates/.../tr11collectibles.bt`，如 `0xF97006F6=Hidden_City_Riddle`）。
10. **本地化**：`locals.bin` 在 `*_allvo_allplt` 家族（非 alltxt）；英文/简中各约
    3.96 万条（解析出 38,234 / 37,625 条）。区域名键（42 个 `regionname_*`）与
    故事条目标题键（`<名称>_title_<guid>`）均在表中，且与条目文本一一对应；
    但 DTP 中 64 位键（如 `0xC53127D26394F448`）与这些键字符串的哈希关联**未闭合**。
11. **坐标分布**（168 条）：coordinateX 范围 0.0–9.19、coordinateY 0.0–0.85；
    161 条非零；instanceZ 160 条非零；hasAreaMarker 仅 12 条为 1（与石碑数一致，
    待语义确认）；doNotShowOnMap 全部为 0。

## 假说（待 SX-020 验证，不得直接进入产品图层）

- placement.id0（64 位）是对象/放置哈希，可与单位地图（primaryRegion/secondary）关联出
  更细的区域归属；尚待用可见对象反例核对。
- coordinateX/Y 非世界 XYZ：量级与分布不符合米制坐标，可能是地图/归一化空间；
  instanceZ 与 elevation 语义未定。坐标不得作为“已验证点位”展示。
- type 数字与游戏内类别（遗物/文献/贮藏点等）的对应可从 marker 图标与计数器口径交叉
  确认；当前只作为观察归纳。

## 未解问题

1. DTP 64 位文本键 → `locals.bin` 键的哈希/查表算法未闭合（已排除 FNV-1a/FNV-1/CRC/
   murmur 等十余种变体与常见键形式；可能经运行时表或额外资源中转）。区域显示名与
   条目标题文字因此暂不能从键直接解析。
2. 重复 ID（`0xC9D62DEE`×5、`0x98564B9D`×2）的组归属差异与语义。
3. 四个更新家族之间 DTP 内容差异的具体条目级差分未做（仅记录负载哈希不同）。
4. `area*` 字段组（含 areaMarkerScale 出现大负值）含义未验证。
5. 26 个区域中若干区域的显示名键与单位地图的关系未建立（94 个 map 记录未逐一核对）。

## 门禁分类（发布复核用）

- 事实性映射（ID→region/unit/placement/坐标）：来自本地商业资源的本地提取；
  是否随交付版发布按 G4 出口“证据与再分发门禁逐类复核”，当前保留在私有目录。
- 字典与 zone 命名：MIT（TrRebootModTools），可随产品分发并保留许可说明。
- CDCE.TIGER.Tool：未声明许可，仅作本地格式核对参考；未复制源码，不进入产品。
