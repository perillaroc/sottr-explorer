# 真实地图基底层（`data/map/basemaps.json`）

子关卡底图与「图鉴归一化坐标 → 图片像素」的变换。界面在详情「同一子关卡内的相对位置」段落里
用它画底图与点位；没有可用底图时回退到原有散点。

- 契约：`schemas/basemap.schema.json`（draft 2020-12）
- 加载器：`crates/sottr-core/src/basemap.rs`（严格模式、稳定错误码）
- 前端类型：`apps/desktop/src/contract/basemap.d.ts`（由 `pnpm contract:types` 生成）
- 界面组件：`apps/desktop/src/components/MapPanel.vue`

## 坐标与变换

图鉴 `position` 是**每单位独立**的归一化空间：原点在底图取景框西北角，**y 向下（向南）增大**。
首版变换是纯比例映射，不含平移、旋转与镜像：

```text
u = x · W        v = y · H
```

因此 `transform.matrix` 恒为 `[W, 0, 0, 0, H, 0]`，`coordinate_y_axis` 为 `down`。
不同子关卡的 `[0,1]` 空间互不相通，不存在全局地图坐标。

变换不是拟合出来的：gameflow 的 `MapData` 自带西北/东南两个标记摆件，`Campsite` 又同时带
归一化坐标与世界摆件，两侧配对求解得到零残差映射。推导见
`doc/2026-10-25-地图纹理逆向/几何标定执行.md` 的 GB-09。

## 状态与精度

| `status` | 条件 |
| --- | --- |
| `unverified` | 三个误差字段必须为 `null` |
| `approximate` | 中位误差 ≤ 2% 图片对角线 |
| `calibrated` | 中位 ≤ 0.5% **且** P95 ≤ 1% 图片对角线 |

加载器按状态强制校验门槛，不达标直接拒绝，不做静默降级。当前 42 条中 **7 条为 `calibrated`**
（241、243、318、682、2287、2306、4026，共覆盖 212 条带位置条目），其余 35 条为
`approximate`——变换同样是解析恒等映射，只是**条目数不足以做统计验证**。

误差口径是中位/P95/最大「图鉴条目到最近摆件网格中心」的像素距离，属**上界**，不是拟合残差。

## 图片位置

图片**随包分发**（D4 = A）：处理后放在 `data/map/maps/*.webp`，共 42 张、约 48 MB。

- `image_root.kind = bundled`，`image_root.path = data/map`；`image` 是相对它的 `maps/<名称>.webp`；
- 开发期由 Vite 中间件把 `/basemaps/maps/*` 映射到该目录，构建期作为 asset 输出到 `dist/basemaps/maps/`；
- 设置 `BASEMAP_IMAGE_ROOT` 可在本地临时改用其它目录（用于验证尚未入库的素材）；
- 目录不存在时前端拿不到图片地址，自动回退散点。

## 覆盖

- 42 个单位有底图，**每个单位恰好一张图，没有「一图多单位」**；
- 291 条带位置条目中 **277 条（95.2%）** 落在底图不透明区域；
- 未覆盖的单位（如 `croft_manor`、多数挑战古墓与特殊区域）保持散点。

逐单位数字见 `doc/2026-10-19-真实地图/覆盖与误差表.md`（由
`private/map-contact/gen_coverage_doc.py` 生成）。

## 生成与校验

生成脚本在开发工作区（`sottr-explorer-devel/private/map-contact/`），不在本仓库：

```powershell
# 重新生成数据集与处理后的图片（图片写在仓库外）
private/venvs/p1/Scripts/python.exe private/map-contact/build_basemaps.py

# 校验数据集与图片（存在性、字节数、SHA-256、WebP 实际尺寸）
cd apps/desktop; pnpm contract:images

# 契约校验（schema + 真实数据集 + 夹具）与前端类型检查
cd apps/desktop; pnpm contract:check
```

`contract:images` 单独成命令、不并入 `contract:check`：外置图片可能不在本机。

## 界面措辞边界

- `calibrated` 才可以说「已校准」，并同时给出中位误差与 P95；
- `approximate` 一律写「近似位置」，`error_source = global_estimate` 时追加「全局估计」；
- 回退散点时保留「坐标未校准，不是导航点位」；
- 任何状态都不把点位表述成导航路线。

## 来源与许可

底图来自 spriters-resource 的三份投稿，作者 **vi7ual**（2025-12-18 提交）：

| 投稿 | 标题 | 覆盖 |
| --- | --- | --- |
| 502127 | Map (1/3) | 24590–24607 |
| 502132 | Map (2/3) | 24608–24622（标题按同批投稿推断） |
| 502133 | Map (3/3) | 24623–24637（标题按同批投稿推断） |

站点 About 页写明素材供「personal projects and non-commercial work」，本项目以**非商业**方式
随包分发，因此 `sources[*].redistributable` 为 `allowed`。素材版权仍归原权利人；
商业用途需另行取得许可。完整记录见 `THIRD_PARTY_NOTICES.md` 第 5 节。
