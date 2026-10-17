// 概览测试（SX-022）：收集进度摘要、区域进度、状态提示与降级/空状态。
import { describe, expect, it } from "vitest";
import OverviewPage from "../src/components/OverviewPage.vue";
import { copy } from "../src/app/labels";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";

describe("概览", () => {
  it("分别标示游戏内完成度字段与工具自己的收集品计数，且不互相换算", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("66.20%");
    expect(wrapper.text()).toContain("推定");
    expect(wrapper.get('[data-testid="collection-summary"]').text()).toContain("246/370");
    expect(wrapper.get('[data-testid="uncollected-value"]').text()).toContain("124 项");
    expect(wrapper.text()).toContain("不能互相换算");
    expect(wrapper.text()).toContain("不是游戏内地图完成度百分比");
    expect(wrapper.get('[data-testid="coverage-banner"]').text()).toContain("收集状态口径");
    expect(wrapper.text()).toContain(copy.coverageScope);
  });

  it("区域进度列出全部主区域与特殊区域入口", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    const progress = wrapper.get('[data-testid="region-progress"]');
    expect(progress.text()).toContain("帕依提提");
    expect(progress.text()).toContain("圣胡安教会");
    expect(progress.text()).toContain("库阿克雅库");
    // 九个主区域全部列出（不含特殊区域）。
    expect(wrapper.findAll('[data-testid^="overview-region-"]').length).toBe(9);
    expect(wrapper.text()).toContain("特殊区域（3）");
  });

  it("点击区域进度进入区域并默认只看未收集", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    await wrapper.get('[data-testid="overview-region-paititi"]').trigger("click");
    expect(store.state.region).toBe("paititi");
    expect(store.state.filters.status).toBe("missing");
    expect(store.state.view).toBe("regions");
  });

  it("未收集入口：切换到全部区域并只看未收集", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    await wrapper.get('[data-testid="view-missing"]').trigger("click");
    expect(store.state.region).toBe("all");
    expect(store.state.filters.status).toBe("missing");
    expect(store.state.view).toBe("regions");
  });

  it("旧快照：常驻过期提示与快照时间，并提示刷新后核对", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    await store.actions.refresh(); // 第一次刷新模拟失败 → stale
    const wrapper = mountWithStore(OverviewPage, store);
    const staleNotice = wrapper.get('[data-testid="stale-banner"]');
    expect(staleNotice.text()).toContain(copy.stale);
    expect(staleNotice.text()).toContain("快照时间");
    expect(staleNotice.text()).toContain("INPUT_CHANGED");
    expect(staleNotice.text()).toContain("请刷新后再核对收集状态");
    expect(wrapper.get('[data-testid="coverage-banner"]').text()).toContain("收集状态口径");
  });

  it("资料文件：整体候选显示不可用与原因，收集品计数显示不适用", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "profile.dat");
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("不可用");
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("资料文件不做进度推定");
    expect(wrapper.get('[data-testid="collection-summary"]').text()).toContain("不适用");
    expect(wrapper.get('[data-testid="uncollected-value"]').text()).toContain("不适用");
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
