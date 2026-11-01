# 来源与许可

工程视角的来源矩阵；法律声明以
[THIRD_PARTY_NOTICES.md](https://github.com/perillaroc/sottr-explorer/blob/main/THIRD_PARTY_NOTICES.md)
为准，完整许可全文见
[THIRD_PARTY_LICENSES.txt](https://github.com/perillaroc/sottr-explorer/blob/main/THIRD_PARTY_LICENSES.txt)。
名称、区域、坐标与计数的提取与对账过程见[研究记录](/data/research/)。

| 来源 | 用到的内容 | 许可/定性 | 是否随产品分发 |
| --- | --- | --- | --- |
| TrRebootModTools | 内部 ID 与候选名称字典、zone 命名 | MIT | 是（名称），许可全文在 `licenses/` |
| Atorizil / ASL-SOTTR | 43 项区域计数偏移与来源目标 | 上游未声明许可证；仅转录事实，定性为事实性数据（2026-10-31 确认） | 是 |
| Stella 攻略站 | 参考存档哈希、攻略组与组级链接 | 仅引用链接与哈希；不打包攻略内容与存档字节 | 否 |
| spriters-resource / vi7ual | 42 张子关卡底图 | 站点声明供个人项目与非商业用途；本项目非商业分发并署名 | 是（非商业） |
| 游戏数据事实 | 名称、区域、单位、坐标、计数 | 项目所有者确认可随交付版发布（2026-10-17 / 2026-10-31） | 是 |
| Rust / JS 依赖 | 运行时与构建依赖 | 各依赖自身许可（全部有 license 字段） | 是（许可全文内嵌） |
| 研究交接包 SOTTR_Save_Research | Python v0.2 参照实现与字典 | 内部研究材料 | 否（仅结论与脚本引用） |
| 公开存档样本 | 20 个公开战役样本 | 仅本地只读比对，不再分发 | 否 |

## 数据集的分层声明

- 代码采用 MIT；
- `data/map/` 的地图底图、`data/catalog/` 与 `data/metadata/` 中的游戏数据事实**不在 MIT 覆盖范围内**，
  各自保留原权利；
- 文档站上的文字说明随仓库以 MIT 提供，除非页面另行标注；引用的游戏数据事实与地图素材除外。

## 非商业边界

地图素材的授权限定个人项目与非商业用途。项目不投放广告、不提供付费版本，文档站也不接入
商业赞助；商业用途需另行取得权利人许可。

「Shadow of the Tomb Raider」及相关商标归 Square Enix / Crystal Dynamics 所有；本项目是只读、
非官方工具，与上述公司无从属关系。
