// 收集品详情测试（SX-022）：名称与状态、去哪里找（未校准相对位置）、怎么拿、原始证据。
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import CollectionDetail from "../src/components/CollectionDetail.vue";
import { storeKey } from "../src/app/inject";
import { createTestStore, analyzeCandidate } from "./helpers";

async function detailFor(store: Awaited<ReturnType<typeof createTestStore>>["store"], id: string) {
  store.actions.openRow(id);
  const entry = store.selectedEntry.value;
  if (!entry) {
    throw new Error(`找不到计划条目：${id}`);
  }
  return mount(CollectionDetail, {
    props: { region: entry.region, group: entry.group, item: entry.item, mode: "pane" },
    global: { provide: { [storeKey]: store } },
    attachTo: document.body,
  });
}

describe("收集品详情", () => {
  it("故事条目显示官方中文名与英文名、状态与所在区域/子关卡", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0xF97006F6");
    const text = wrapper.text();
    expect(text).toContain("隐秘之城的谜团");
    expect(text).toContain("HIDDEN CITY RIDDLE");
    expect(text).toContain("0xF97006F6");
    expect(wrapper.get('[data-testid="detail-heading"]').text()).toContain("隐秘之城的谜团");
    expect(text).toContain("特殊 / 跨区域");
  });

  it("有位置的条目：显示未校准的相对位置散点与说明", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    // 帕依提提主城的遗物（有坐标）
    const wrapper = await detailFor(store, "0xF1FAE49A");
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("未校准");
    expect(wrapper.text()).toContain("不是导航点位");
    expect(wrapper.get('[data-testid="detail-location"]').text()).toContain("帕依提提");
  });

  it("获取方式来自类别规则，并标注为人工整理", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0xF1FAE49A");
    expect(wrapper.get('[data-testid="detail-acquisition"]').text()).toContain("在关卡内拾取");
    expect(wrapper.text()).toContain("类别通用说明（人工整理");
  });

  it("非故事收集品使用类别通用获取方式", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0x0F28B6EC");
    expect(wrapper.get('[data-testid="detail-acquisition"]').text()).toContain("箱子或背包");
    expect(wrapper.text()).toContain("生存贮藏点");
  });

  it("状态说明区分已收集 / 未收集，并提示刷新", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const all = store.plan.value.regions.flatMap((region) => region.groups.flatMap((group) => group.items));
    const uncollectedId = all.find((item) => item.status === "not_collected")!.id;
    const collectedId = all.find((item) => item.status === "collected")!.id;

    const uncollected = await detailFor(store, uncollectedId);
    expect(uncollected.get('[data-testid="detail-status-reason"]').text()).toContain("未出现在当前存档的物品表中");
    expect(uncollected.text()).toContain("刷新");

    const collected = await detailFor(store, collectedId);
    expect(collected.get('[data-testid="detail-status-reason"]').text()).toContain("出现在当前存档的物品表中");
  });

  it("共用 ID 的挑战古墓列出全部名称且不拆分判定", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0xC9D62DEE");
    expect(wrapper.text()).toContain("按 ID 计数，不拆分判定");
  });

  it("原始证据默认折叠，展开后显示图鉴版本、子关卡与坐标空间", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0xF1FAE49A");
    const details = wrapper.get("details");
    expect(details.attributes("open")).toBeUndefined();
    await details.get("summary").trigger("click");
    details.element.setAttribute("open", "");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("图鉴版本");
    expect(wrapper.text()).toContain("unit_normalized_unverified");
    expect(wrapper.text()).toContain("zone");
  });

  it("复制线索只含名称/状态/获取方式/位置，不含私人路径", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = await detailFor(store, "0xF1FAE49A");
    await wrapper.get('[data-testid="copy-lead"]').trigger("click");
    await wrapper.vm.$nextTick();
    const lead = wrapper.get('[data-testid="lead-text"]').text();
    expect(lead).toContain("庆典火把");
    expect(lead).toContain("怎么拿");
    expect(lead).toContain("位置");
    expect(lead).not.toContain("C:");
  });
});
