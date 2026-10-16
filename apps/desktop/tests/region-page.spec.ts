// 区域与收集页测试（SX-015 验收：组行折叠、筛选与空状态区分、43 项稀疏不补零、
// 长名称、返回列表与范围提示）。
import { describe, expect, it } from "vitest";
import RegionPage from "../src/components/RegionPage.vue";
import { copy } from "../src/app/labels";
import { createTestStore, analyzeCandidate, mountWithStore, candidatePath } from "./helpers";

async function regionStore() {
  const { store } = await createTestStore();
  await analyzeCandidate(store, "save1.dat");
  store.actions.setView("regions");
  return store;
}

describe("区域与收集页", () => {
  it("默认参考遗漏视图按攻略组折叠为组行，不复制为多个精确条目", async () => {
    const store = await regionStore();
    const wrapper = mountWithStore(RegionPage, store);
    const text = wrapper.text();
    expect(text).toContain("圣胡安贮藏点组 · 本组 8 项参考遗漏推定");
    expect(text).toContain("圣胡安石碑组 · 本组 1 项参考遗漏推定");
    expect(text).toContain("圣胡安墓穴宝藏组 · 本组 1 项参考遗漏推定");
    expect(text).toContain("攻略组线索，不复制为精确点位");
    // 组行只有一条“攻略组”定位徽标；组外遗漏（帕伊提提 2 项）为逐项行。
    expect(text).toContain("帕伊提提");
    // 展开组内条目显示成员，但仍标注攻略组。
    const expand = wrapper
      .findAll("button")
      .find((button) => button.text().includes("展开组内条目"));
    expect(expand).toBeTruthy();
    await expand?.trigger("click");
    expect(wrapper.text()).toContain("组内共 8 个成员 ID");
  });

  it("区域范围与剩余提示：其余 N 项请在全部区域查看", async () => {
    const store = await regionStore();
    store.actions.setRegion("mission_of_san_juan");
    const wrapper = mountWithStore(RegionPage, store);
    expect(wrapper.text()).toContain("区域与收集 · 圣胡安");
    expect(wrapper.get('[data-testid="remaining-hint"]').text()).toContain("其余 2 项请在全部区域查看");
    // 本区域内不再出现帕伊提提逐项行。
    expect(wrapper.text()).not.toContain("帕伊提提");
  });

  it("搜索匹配候选内部名；无结果文案与“未发现遗漏”区分", async () => {
    const store = await regionStore();
    const wrapper = mountWithStore(RegionPage, store);
    store.actions.setSearch("SpanishMission_03");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("本组 1 项参考遗漏推定");

    store.actions.setSearch("完全不存在的名字");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(copy.emptyFiltered);
    expect(wrapper.text()).not.toContain(copy.emptyMissing);
    expect(wrapper.text()).toContain("清空搜索");
  });

  it("状态筛选：已观察与状态未知都能找到条目；未知不被默认视图永久隐藏", async () => {
    const store = await regionStore();
    const wrapper = mountWithStore(RegionPage, store);
    store.actions.setStatusFilter("observed");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("已观察");
    expect(wrapper.text()).toContain("原始观察"); // 依据徽标与状态分开

    store.actions.setStatusFilter("unknown");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("状态未知");
    expect(wrapper.text()).toContain("0x5EED0001");
  });

  it("支持范围内确实没有遗漏时显示专门文案（空差集不等于全游戏 100%）", async () => {
    const store = await regionStore();
    const path = candidatePath("save1.dat");
    const slot = store.state.slots[path];
    if (!slot?.result?.report) {
      throw new Error("测试前置：应有报告");
    }
    // 模拟空差集：移除全部参考遗漏条目（其余状态保留）。
    slot.result.report.assessments = slot.result.report.assessments.filter(
      (item) => item.assessment !== "likely_missing",
    );
    store.actions.setRegion("all");
    const wrapper = mountWithStore(RegionPage, store);
    expect(wrapper.text()).toContain(copy.emptyMissing);
    expect(wrapper.text()).toContain(copy.coverage); // 同屏保留类别覆盖
    expect(wrapper.text()).not.toContain("100%");
  });

  it("区域未知范围有单独说明；区域计数不补零", async () => {
    const store = await regionStore();
    const wrapper = mountWithStore(RegionPage, store);
    store.actions.setRegion("unknown");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(copy.emptyUnknownRegion);

    store.actions.setRegion("mission_of_san_juan");
    await wrapper.vm.$nextTick();
    expect(wrapper.get('[data-cell="mission_of_san_juan:survival_caches:value"]').text()).toContain("0/8");
  });

  it("点击行选择条目（保留稳定 ID），并用按钮承载键盘语义", async () => {
    const store = await regionStore();
    const wrapper = mountWithStore(RegionPage, store);
    const row = wrapper.get('[data-testid="row-loc:stella:san-juan-caches"]');
    expect(row.element.tagName).toBe("BUTTON");
    await row.trigger("click");
    expect(store.state.selectedRowId).toBe("loc:stella:san-juan-caches");
  });

  it("中文长名称允许完整显示（两行换行，不裁掉名称）", async () => {
    const store = await regionStore();
    const path = candidatePath("save1.dat");
    const slot = store.state.slots[path];
    const report = slot?.result?.report;
    if (!report) {
      throw new Error("测试前置：应有报告");
    }
    // 追加一个长中文名条目（模拟数据），验证不截断。
    const longName = "这是一条用于检查两行换行与完整显示的超长中文收集品名称";
    store.metadata.items["0x5EED0002"] = {
      ...store.metadata.items["0x5EED0001" as never],
      internal_id: "0x5EED0002",
      display_name: longName,
      name_kind: "curated_short",
      aliases: [],
      category_id: "survival_caches",
      region_id: "mission_of_san_juan",
    };
    report.assessments = [
      ...report.assessments,
      {
        internal_id: "0x5EED0002",
        observation: "reference_present_current_absent",
        assessment: "likely_missing",
        rule_id: "refdiff.v1.likely_missing",
        evidence_refs: [],
        limitations: ["模拟长名称"],
      },
    ];
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("all");
    const wrapper = mountWithStore(RegionPage, store);
    expect(wrapper.text()).toContain(longName);
    expect(wrapper.find(".line-clamp-2").exists()).toBe(true);
  });
});
