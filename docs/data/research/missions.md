# missions 语义（P3-3）

DTP 的区域数据里有一组 `missions`（20 条：Paititi 15、Kuwaq Yaku 2、Mission of San Juan 3），
本页记录 2026-11-18 的时间盒研究：它表示什么，为什么与外部“8 条支线任务”不一致。

::: warning 状态：语义已定，不进入产品
这 20 条是**任务目标（mission objective）级**条目，不是支线任务清单；
每条目标在存档里有独立状态（0/1/2）。名称无法从 32 位 ID 解析（见
[DTP 64 位文本键](/data/research/text-keys)），因此不在产品中展示。
:::

## 游戏数据的层级

`locals.bin` 里有两组键：

- `socialpuzzle_missionname_*`：支线任务名，11 个键 / 10 个不同任务
  （`armsdeal` 与 `armsdeal_alt` 是同一任务的两个结局名）。
- `mission_XX_obj_Y_(title|desc|sub|comp|new)_<guid>`：任务目标，
  22 个目标容器，分布在 7 个任务编号（01、02、03、05、06、07、09）。

例如：

```text
mission_01_obj_a_title = BLOOD CRIES OUT
mission_01_obj_a_desc  = Find smuggled goods
mission_01_obj_a_sub   = Break the generator / Interrupt the game / …
mission_01_obj_b_desc  = Find the smuggler's cache
mission_03_obj_c_title = HEARST AND MINDS
```

即一个任务有多个目标（obj a/b/c/…），每个目标又有多个子步骤（sub）。
22 个目标容器与 20 条区域 missions 量级一致（去掉 `mission_09_obj_new`
这一新版文本后为 21，仍与 20 差 1）。

## 存档里的目标状态

每个 ID 在解压存档中对应一条 56 字节记录：

```text
u32 id | u32 state | u32 count | f32 x | f32 y | …
```

`state` 分布与进度一致：

| 存档 | state 2 | state 1 | state 0 | 未找到 |
| --- | --- | --- | --- | --- |
| NG+ 100%（4 个） | 20 | 0 | 0 | 0 |
| NG+ 92.375% | 18 | 1 | 1 | 0 |
| 私人战役 66.2% | 14 | 0 | 6 | 0 |
| 100% 中配（无 DLC） | 13 | 0 | 5 | 2 |
| Cozumel 前段 | 0 | 0 | 14 | 6 |

因此 `state` 是目标状态：0 = 未开始，1 = 进行中，2 = 已完成。
100% 中配的 5 个未完成 + 2 个缺失，与“未持有对应 DLC 内容”一致
（同一批 ID 在 NG+ 全 DLC 存档里全部为 2）。

## 为什么是 20 而外部说 8

两者不在同一层级：

| 口径 | 计数 | 说明 |
| --- | --- | --- |
| 外部 100% 指南“8 条支线” | 8 | 任务级；不同指南对“哪些算支线”口径不一 |
| `socialpuzzle_missionname_*` | 10（11 键含结局别名） | 游戏内支线任务名 |
| `mission_XX_obj_*` | 22 个目标容器（21 去重） | 任务目标级 |
| `RegionData.missions` | 20 | 目标级地图标记，分布 15 / 2 / 3 |

20 与 8 不是矛盾，而是**目标级 vs 任务级**的差异；20 更接近 21 个目标容器。

## 产品口径

- 不把 `missions` 当支线任务清单，不进入缺失结论；
- 不用它计算 100% 或区域百分比；
- 逐条目标的名称需要 32 位 ID → 目标键的映射，目前受键映射障碍限制，无法展示。

## 复现

脚本留在开发工作区（`private/p3_3/missions_p33.py`），只读 gameflow DTP、
`locals_english.bin` 与存档副本，输出 20 条 ID、区域分布、支线名与各存档状态分布。
