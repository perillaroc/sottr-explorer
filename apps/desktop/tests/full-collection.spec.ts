// SX-021 阶段 C2b：全收集（100%）页面渲染与保护性规则。
import { describe, expect, it } from "vitest";

import FullCollectionPage from "../src/components/FullCollectionPage.vue";
import { copy } from "../src/app/labels";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

describe("FullCollectionPage", () => {
  it("必收/可选/未闭合分组与缺失清单", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    expect(store.report.value).not.toBeNull();

    const wrapper = mountWithStore(FullCollectionPage, store);
    expect(wrapper.find('[data-testid="full-collection"]').exists()).toBe(true);
    const text = wrapper.text();
    expect(text).toContain("必收类别");
    expect(text).toContain("可选类别");
    expect(text).toContain("未闭合类别");

    // 必收指引覆盖：352 条全部有地图或文字指引。
    const coverage = wrapper.get('[data-testid="guidance-coverage"]').text();
    expect(coverage).toContain("必收指引 352/352");
    expect(coverage).toContain("地图 243");
    expect(coverage).toContain("合集 32");
    expect(coverage).toContain("子关卡 77");

    // 必收类别：artifacts 按必收子集计 253，且有可展开缺失清单。
    const artifacts = wrapper.find('[data-testid="category-artifacts"]');
    expect(artifacts.exists()).toBe(true);
    expect(artifacts.text()).toContain("/253");
    const toggle = wrapper.find('[data-testid="missing-toggle-artifacts"]');
    expect(toggle.exists()).toBe(true);
    await toggle.find("summary").trigger("click");
    const items = artifacts.findAll('[data-testid="missing-item"]');
    expect(items.length).toBeGreaterThan(0);

    // 可选类别：只给计数，不进入缺失清单。
    const backpacks = wrapper.find('[data-testid="category-explorer_backpacks"]');
    expect(backpacks.exists()).toBe(true);
    expect(backpacks.text()).toContain("探险者背包");
    expect(backpacks.text()).toContain("参考数未知");
    expect(wrapper.find('[data-testid="missing-toggle-explorer_backpacks"]').exists()).toBe(false);

    // 保险箱有区域计数器对账，可以显示分母。
    const strongboxes = wrapper.find('[data-testid="category-strongboxes"]');
    expect(strongboxes.exists()).toBe(true);
    expect(strongboxes.text()).toContain("/8");
    expect(strongboxes.text()).toContain("区域计数器已对账");

    // 训练场等非收集品只展示，不计入必收。
    const gym = wrapper.find('[data-testid="category-gym_training"]');
    expect(gym.exists()).toBe(true);
    expect(gym.text()).toContain("仅展示");

    // DLC 故事条目：7 个包、合计 42 条，并明确不计入 100%。
    expect(text).toContain("DLC 故事条目（不计入 100%）");
    expect(text).toContain("/42");
    expect(wrapper.findAll('[data-testid^="dlc-group-"]')).toHaveLength(7);
    expect(wrapper.find('[data-testid="dlc-group-DLC01"]').exists()).toBe(true);

    // 未闭合类别：给出观察计数但不显示缺失数或清单。
    const tombs = wrapper.find('[data-testid="category-challenge_tombs"]');
    expect(tombs.exists()).toBe(true);
    expect(tombs.text()).toContain("挑战古墓");
    expect(wrapper.find('[data-testid="missing-toggle-challenge_tombs"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("未分析时显示引导空状态", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(FullCollectionPage, store);
    expect(wrapper.text()).toContain("先分析一个存档文件");
    wrapper.unmount();
  });

  it("资料文件显示无收集品清单，不把空表当作全缺失", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "profile.dat");
    const wrapper = mountWithStore(FullCollectionPage, store);
    expect(wrapper.text()).toContain("资料文件没有收集品清单");
    expect(wrapper.find('[data-testid="required-band"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it("旧快照保留提示与数据，不隐藏缺失口径", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    await store.actions.refresh(); // 第一次刷新模拟失败 → stale
    const wrapper = mountWithStore(FullCollectionPage, store);
    expect(wrapper.get('[data-testid="stale-banner"]').text()).toContain(copy.stale);
    expect(wrapper.find('[data-testid="required-band"]').exists()).toBe(true);
    wrapper.unmount();
  });
});
