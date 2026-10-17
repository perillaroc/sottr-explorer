// SX-021 阶段 C2b：全收集（100%）页面渲染与保护性规则。
import { describe, expect, it } from "vitest";

import FullCollectionPage from "../src/components/FullCollectionPage.vue";
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
    expect(backpacks.text()).toContain("可选，不进入缺失清单");

    // 未闭合类别：给出观察计数但不显示缺失数或清单。
    const tombs = wrapper.find('[data-testid="category-challenge_tombs"]');
    expect(tombs.exists()).toBe(true);
    expect(tombs.text()).toContain("数据未闭合：不显示缺失数或清单");
    expect(wrapper.find('[data-testid="missing-toggle-challenge_tombs"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("未分析时显示引导空状态", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(FullCollectionPage, store);
    expect(wrapper.text()).toContain("先分析一个存档文件");
    wrapper.unmount();
  });
});
