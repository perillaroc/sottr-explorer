---
layout: home

hero:
  name: SOTTR Explorer
  text: 《古墓丽影：暗影》本地只读存档分析
  tagline: 打开你的存档，看清还差哪些收集品、它们在哪、怎么拿。原文件不会被改动一个字节。
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: 开发文档
      link: /development/

features:
  - title: 只读分析
    details: 读取稳定快照，前后哈希一致才生成报告；不写回、不修复、不改名、不删除。
  - title: 全收集口径
    details: 按必收规则区分故事与非故事收集品、DLC 与挑战古墓；差集不会被当成全集结论。
  - title: 位置与地图
    details: 42 个子关卡配有底图；位置是关卡内相对位置，并明确区分已校准与近似。
  - title: 结论可追溯
    details: 每条判断都带规则、证据与限制；未知就写未知，不用 0 或空清单冒充结论。
---

## 适用版本

Windows Steam 版《古墓丽影：暗影》（含 Definitive Edition）。不支持其它平台或
非 Steam 安装；存档发现、稳定快照读取与路径策略都按 Windows 实现。

## 隐私与安全

- 不联网：不加载任何网页，不上传存档，无账号与遥测；外部攻略链接交给系统浏览器打开。
- 只读：不写回、不修复、不改名、不删除你的存档。
- 本地输入输出：拒绝 UNC 与设备路径；导出必须写到不存在的新目录，不覆盖已有文件。

## 从哪里开始

- 第一次使用：读[快速开始](/guide/getting-started)。
- 想了解界面：读[界面导览](/guide/interface)。
- 想改代码或复核数据：从[开发文档](/development/)或[数据与来源](/data/)进入。
- 接口字段与状态定义：见[契约 1.0.0](/contracts)。
