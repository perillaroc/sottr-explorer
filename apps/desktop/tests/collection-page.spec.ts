// 区域与收集页测试（SX-022）：区域卡片、逐项清单、状态筛选、搜索、目录外记录。
import { describe, expect, it } from "vitest";
import CollectionPage from "../src/components/CollectionPage.vue";
import { categoryNameOf } from "../src/app/collection";
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
    // 阶段四：区域名为主标题，“区域与收集”作为页面上文。
    expect(wrapper.get('[data-testid="page-title"]').text()).toContain("圣胡安教会");
    expect(wrapper.get('[data-testid="page-context"]').text()).toContain("区域与收集");
    expect(text).toContain("文献");
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

  it("分组头使用 sticky，保留类别色与子类别", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    const wrapper = mountWithStore(CollectionPage, store);
    const header = wrapper.get('[data-testid="group-header"]');
    expect(header.classes()).toContain("sticky");
    expect(header.classes()).toContain("top-0");
    expect(header.find("h2")?.classes().some((name) => name.startsWith("cat-text-"))).toBe(true);
    const storyHeader = wrapper
      .findAll('[data-testid="group-header"]')
      .find((entry) => entry.text().includes("故事收藏"));
    expect(storyHeader).toBeTruthy();
    expect(storyHeader?.find('[data-testid="group-subcategory"]').text()).toContain("文献");
  });

  it("清单行有状态图形与文字；选中行使用左侧标记与名称加粗", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("all");
    const wrapper = mountWithStore(CollectionPage, store);
    const row = wrapper.get('[data-testid="row-0x0F28B6EC"]');
    expect(row.classes()).toContain("min-h-14");
    expect(row.find(".ui-status-mark").exists()).toBe(true);
    expect(row.text()).toContain("已收集");

    await row.trigger("click");
    await wrapper.vm.$nextTick();
    const item = wrapper.get('[data-testid="row-0x0F28B6EC"]').element.closest("li");
    expect(item?.className).toContain("ui-row-selected");
    expect(wrapper.get('[data-testid="row-0x0F28B6EC"] .line-clamp-2').classes()).toContain("font-medium");
  });

  it("分类单独成行并按类别着色：分组头是着色类别名，行内是同一色相的圆点", async () => {
    const store = await collectionStore();
    store.actions.setRegion("mission_of_san_juan");
    const wrapper = mountWithStore(CollectionPage, store);
    const list = wrapper.get('[data-testid="region-list"]');

    // 第一行：着色类别名 + 计数；第二行：子类别。
    const headers = list.findAll("h2");
    expect(headers.length).toBeGreaterThan(0);
    for (const header of headers) {
      expect(header.classes().some((name) => name.startsWith("cat-text-"))).toBe(true);
    }
    const artifacts = headers.find((header) => header.text().includes("故事收藏"));
    expect(artifacts?.classes()).toContain("cat-text-artifacts");
    expect(artifacts?.text()).not.toContain("遗物");
    expect(list.get('[data-testid="group-subcategory"]').text()).toContain("文献");
    expect(list.get('[data-testid="group-subcategory"]').text()).not.toContain("子类别：");

    // 行内圆点：每行一个，颜色与所在分组一致。
    const rows = list.findAll('[data-testid^="row-"]');
    const dots = list.findAll(".cat-dot");
    expect(dots.length).toBe(rows.length);
    const firstGroup = store.planGroups.value[0];
    expect(dots[0].classes().some((name) => name.startsWith("cat-dot-"))).toBe(true);
    expect(dots[0].attributes("title")).toBe(categoryNameOf(store.atlas, firstGroup.category));
  });

  it("区域与条目都能跳转地图：选中条目并打开其子关卡底图", async () => {
    const store = await collectionStore();
    store.actions.setRegion("paititi");
    store.actions.setStatusFilter("all");
    const wrapper = mountWithStore(CollectionPage, store);

    // 区域级按钮：打开帕依提提默认底图，不选中条目。
    await wrapper.get('[data-testid="region-map-open"]').trigger("click");
    expect(store.state.view).toBe("map");
    expect(store.state.mapRegion).toBe("paititi");
    expect(store.state.selectedRowId).toBeNull();

    // 回到区域清单，点击某个条目的地图按钮：带着条目进入地图并聚焦。
    store.actions.setView("regions");
    store.actions.setRegion("paititi");
    await wrapper.vm.$nextTick();
    const itemButton = wrapper.find('[data-testid^="item-map-"]');
    expect(itemButton.exists()).toBe(true);
    await itemButton.trigger("click");
    const id = (itemButton.attributes("data-testid") ?? "").replace("item-map-", "");
    expect(store.state.view).toBe("map");
    expect(store.state.selectedRowId).toBe(id);
    expect(store.state.mapRailTab).toBe("detail");
    expect(store.state.mapBasemapId).not.toBeNull();
  });

  it("无底图条目不显示地图按钮", async () => {
    const store = await collectionStore();
    // 克劳馥庄园（特殊区域）没有底图。
    store.actions.setRegion("croft_manor");
    store.actions.setStatusFilter("all");
    const wrapper = mountWithStore(CollectionPage, store);
    expect(wrapper.find('[data-testid^="item-map-"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="region-map-open"]').exists()).toBe(false);
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
