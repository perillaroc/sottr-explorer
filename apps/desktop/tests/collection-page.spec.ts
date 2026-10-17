// 区域与收集页测试（SX-022）：区域卡片、逐项清单、状态筛选、搜索、目录外记录。
import { describe, expect, it } from "vitest";
import CollectionPage from "../src/components/CollectionPage.vue";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";

async function collectionStore(observedIds?: () => string[]) {
  const { store } = await createTestStore({ observedIds });
  await analyzeCandidate(store, "save1.dat");
  store.actions.setView("regions");
  return store;
}

describe("区域与收集页", () => {
  it("全部区域视图列出 9 个主区域与 3 个特殊区域，并显示已收集/总数", async () => {
    const store = await collectionStore();
    const wrapper = mountWithStore(CollectionPage, store);
    const overview = wrapper.get('[data-testid="region-overview"]');
    expect(overview.text()).toContain("帕依提提");
    expect(overview.text()).toContain("圣胡安教会");
    expect(overview.text()).toContain("挑战古墓");
    expect(overview.text()).toContain("克劳馥庄园");
    expect(wrapper.findAll('[data-testid^="region-card-"]').length).toBe(12);
    // 帕依提提在 demo 观察集合下的进度来自计划，不是游戏内百分比。
    const card = wrapper.get('[data-testid="region-card-paititi"]');
    expect(card.text()).toMatch(/\d+\/159/);
    expect(wrapper.text()).toMatch(/主区域 \d+\/370 已收集/);
  });

  it("进入区域后按类别分组，默认只显示未收集", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    const wrapper = mountWithStore(CollectionPage, store);
    const text = wrapper.text();
    expect(text).toContain("区域与收集 · 圣胡安教会");
    expect(text).toContain("Document（文献）");
    expect(text).toContain("未收集");
    // 分组进度是该组真实总数（圣胡安文献共 27 项），不随筛选变化。
    const region = store.planRegion.value!;
    const documents = region.groups.find((group) => group.subcategory === "documents")!;
    expect(documents.total).toBe(27);
    expect(wrapper.get('[data-testid="region-list"]').text()).toContain("/27");
  });

  it("状态筛选可切到已收集，并显示实心中文的已收集徽标", async () => {
    const store = await collectionStore();
    store.actions.setRegion("paititi");
    const wrapper = mountWithStore(CollectionPage, store);
    store.actions.setStatusFilter("observed");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("已收集");
    expect(wrapper.findAll('[data-testid^="row-"]').length).toBeGreaterThan(0);
  });

  it("搜索匹配内部候选名与 ID；无结果时给出可恢复的空状态", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    const wrapper = mountWithStore(CollectionPage, store);
    // 先看全部（未收集视图会过滤掉已收集条目），再用内部候选名搜索。
    store.actions.setStatusFilter("all");
    await wrapper.vm.$nextTick();
    store.actions.setSearch("SpanishMission_03");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("SpanishMission_03");

    store.actions.setSearch("完全不存在的名字");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("没有匹配的收集品");
    expect(wrapper.text()).toContain("清空搜索");
  });

  it("点击行选择条目（保留稳定 ID），行是按钮", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("all");
    const wrapper = mountWithStore(CollectionPage, store);
    const row = wrapper.get('[data-testid="row-0x0F28B6EC"]');
    expect(row.element.tagName).toBe("BUTTON");
    await row.trigger("click");
    expect(store.state.selectedRowId).toBe("0x0F28B6EC");
  });

  it("目录外观察记录单独列出，不混入任何区域", async () => {
    const store = await collectionStore(() => ["0xF97006F6", "0xDEADBEEF", "0x12345678"]);
    store.actions.setRegion("unknown");
    const wrapper = mountWithStore(CollectionPage, store);
    const text = wrapper.get('[data-testid="unknown-records"]').text();
    expect(text).toContain("0xDEADBEEF");
    expect(text).toContain("0x12345678");
    expect(text).not.toContain("0xF97006F6");
  });

  it("资料文件不显示收集清单", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "profile.dat");
    store.actions.setView("regions");
    const wrapper = mountWithStore(CollectionPage, store);
    expect(wrapper.text()).toContain("资料文件没有收集品清单");
  });

  it("未分析时显示打开存档入口", async () => {
    const { store } = await createTestStore();
    store.actions.setView("regions");
    const wrapper = mountWithStore(CollectionPage, store);
    expect(wrapper.text()).toContain("尚未分析该文件");
  });
});
