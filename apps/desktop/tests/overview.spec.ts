// 概览测试（SX-015 验收：66.20% 与支持范围内 12 项分别标示、推定标签不丢失、
// 43 项稀疏计数不补零、coverage banner 始终可发现、降级/过期/错误状态）。
import { describe, expect, it } from "vitest";
import OverviewPage from "../src/components/OverviewPage.vue";
import { copy } from "../src/app/labels";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";

describe("概览", () => {
  it("分别标示 66.20% 与 12 项，保留推定标签，并避免混淆分母", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("66.20%");
    expect(wrapper.get('[data-testid="overall-summary"]').text()).toContain("推定");
    expect(wrapper.get('[data-testid="missing-value"]').text()).toContain("12 项");
    expect(wrapper.get('[data-testid="location-value"]').text()).toContain("已验证坐标 0 项");
    const text = wrapper.text();
    expect(text).toContain("不是“全游戏还差 N 项”");
    expect(text).toContain("未反映剩余比例构成");
    // coverage banner 在主流程始终可见（概览）。
    expect(wrapper.get('[data-testid="coverage-banner"]').text()).toContain(copy.coverage);
    expect(wrapper.text()).toContain(copy.coverageScope);
  });

  it("区域入口只显示已支持计数：缺读数显示未知、无映射显示暂不支持，不补 0", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    // 圣胡安：三个参考单元格都有读数（0/8、0/1、0/1）。
    expect(wrapper.get('[data-cell="mission_of_san_juan:survival_caches:value"]').text()).toContain("0/8");
    expect(wrapper.get('[data-cell="mission_of_san_juan:monoliths:value"]').text()).toContain("0/1");
    expect(wrapper.get('[data-cell="mission_of_san_juan:crypt_treasures:value"]').text()).toContain("0/1");
    // 科苏梅尔：贮藏点有映射但本次无读数 → 未知；石碑/墓穴宝藏在 43 项映射之外 → 暂不支持。
    expect(wrapper.get('[data-cell="cozumel:survival_caches:unknown"]').text()).toContain("未知");
    expect(wrapper.get('[data-cell="cozumel:monoliths:unsupported"]').text()).toContain("暂不支持");
    expect(wrapper.get('[data-cell="cozumel:crypt_treasures:unsupported"]').text()).toContain("暂不支持");    // 任何单元格都不会被补成 0/0 或伪造目标。
    expect(wrapper.text()).not.toContain("0/0");
  });

  it("降级报告：遗漏入口禁用并解释原因，仍可浏览受支持收集品", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save206.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    const button = wrapper.get('[data-testid="view-missing"]');
    expect(button.attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain(copy.missingUnavailable);
    expect(wrapper.text()).toContain(copy.missingUnavailableHelp);
    // 覆盖提示仍在，且浏览入口可用。
    expect(wrapper.get('[data-testid="coverage-banner"]').text()).toContain(copy.coverage);
    expect(wrapper.text()).toContain("浏览受支持的收集品");
  });

  it("旧快照：常驻过期提示与快照时间，不以当前遗漏展示", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    await store.actions.refresh(); // 第一次刷新模拟失败 → stale
    const wrapper = mountWithStore(OverviewPage, store);
    const staleNotice = wrapper.get('[data-testid="stale-banner"]');
    expect(staleNotice.text()).toContain(copy.stale);
    expect(staleNotice.text()).toContain("快照时间");
    expect(staleNotice.text()).toContain("INPUT_CHANGED");
    expect(staleNotice.text()).toContain("不能当作当前遗漏");
    // 覆盖提示与状态提示并存（不互相替代）。
    expect(wrapper.get('[data-testid="coverage-banner"]').text()).toContain(copy.coverage);
  });

  it("资料文件：整体候选显示不可用与原因，不显示 0%", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "profile.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("不可用");
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("资料文件不做进度推定");
    expect(wrapper.text()).not.toContain("0.00%");
  });

  it("未分析时显示说明与入口，不出现假进度", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.text()).toContain(copy.noReport);
    expect(wrapper.text()).toContain(copy.noReportHelp);
    expect(wrapper.text()).not.toContain("66.20%");
    expect(wrapper.text()).not.toContain("0%");
  });
});
