# DTP 64 位文本键（P3-2）

DTP 资源里有一些 64 位键（如 `0xC53127D26394F448`），本页记录 2026-11-18 的时间盒研究：
它们能否由 `locals.bin` 的键推导出来，以及为什么产品不依赖它们。

::: warning 状态：否证（未找到算法）
已排除常见哈希算法与键形式；结论收窄为“自定义/带种子的哈希”或“独立映射表”。
产品仍使用“DTP 明文 + locals 值匹配”路径，覆盖全部故事条目与已解析区域名。
:::

## 测试集

| 对象 | 对数 | 配对依据 |
| --- | --- | --- |
| 故事条目 `nameKey` ↔ locals 标题键 | 302 | DTP 明文与 locals 值完全一致 |
| 区域 `regionLocalNameKey` ↔ `regionname_*` 键 | 10 | 区域名值与 locals 值一致 |

示例：

```text
nameKey  C53127D26394F448
locals   hidden_city_riddle_title_38aa2bcd_d256_4726_8b69_36801ba34a76
metric   Hidden_City_Riddle
text     HIDDEN CITY RIDDLE
```

每个故事的标题、描述、短描述各有一个 64 位键，对应三个带不同 GUID 的 locals 键；
DTP 内既不含 GUID，也不含 locals 键原文。

## 已排除

对全部 312 对键逐一比较全 64 位与高/低 32 位：

| 类别 | 已测试 |
| --- | --- |
| 算法 | FNV-1/1a 64（标准与 0 偏移）、MurmurHash64A、MurmurHash3 x64 128、Murmur3 32、xxHash64/xxHash3/xxHash32、CityHash64、FarmHash64、CRC-64/ECMA、CRC32、djb2、sdbm、MD5/SHA1/SHA256/BLAKE2b/SHA3 截断 |
| 种子 | murmur3 / xxh64 0–20000；CityHash64WithSeed 0–2000 |
| 编码 | UTF-8、UTF-8+NUL、UTF-16LE/BE、长度前缀 |
| 键形式 | locals 键、metric、明文、GUID、资源路径、前缀/后缀切分、item ID 拼入 |
| 文本归一化 | 约 24 万种大小写/分隔符/前后缀组合 |
| 结构关系 | UUID3/5 派生、XOR 常量、加法常量、两个 CRC32 半字 |

结果：全 64 位、半字与常量偏移变体的匹配数均为 0。

引擎二进制中可以确认存在 FNV-1a 64（含一个大写转小写的变体）、Murmur3、CRC-64 与
xxHash 的实现，但这些标准实现作用于任何可推导字符串都不产生这些键。

## 结论

两种可能仍未排除：

1. 引擎使用自定义或带种子的哈希，种子/表不在已提取资源中；
2. 64 位键是作者侧的随机 ID，与 locals 键通过一张独立的“键 → 字符串”表关联，
   该表不在 `globalcollectibleinfo.drm`、`globalgameflow.drm` 或 `locals.bin` 中
   （DTP 无外部引用，DRM 无依赖）。

简单情形已经排除：键不是 locals 键的常见哈希，不是 UUID 派生，不是“哈希 + 固定常量”，
也不是文本或 metric 的常见归一化哈希。

## 产品口径

名称解析不依赖 64 位键：

| 对象 | 路径 | 覆盖 |
| --- | --- | --- |
| 故事条目名 | DTP 明文 ↔ locals 值 | 302/302 |
| 区域显示名 | `region_names.json` 的 `locale_key` + 值 | 10/10 |
| 非故事条目 | metric 名 + 人工补全表（locals 无对应键） | 不适用 |

22 条故事文本在 locals 中有多个同值键（重复文本），但“值 → 文本”唯一；
只有“键 → 值”方向存在歧义，产品不使用该方向。

## 下一步

- 在 bigfile 中搜索包含 8 字节键的“哈希 → 字符串”资源；
- 逆向本地化加载函数（`%slocal\locals.bin`、`Could Not Load Localization Table: %s`），
  确认建表时的哈希与大小写规则；
- 收集“只有键、没有明文”的字段样本，反向约束键空间。

脚本与完整测试矩阵留在开发工作区（`private/p3_2/`）。
