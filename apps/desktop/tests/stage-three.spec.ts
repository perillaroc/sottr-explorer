// 阶段三页面骨架（2026-11-05 界面美化）：概览摘要工作面 / 区域索引列数、
// 全收集必收工作带与矩阵、导入文件表格与选中标记。
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import OverviewPage from "../src/components/OverviewPage.vue";
import FullCollectionPage from "../src/components/FullCollectionPage.vue";
import SlotPicker from "../src/components/SlotPicker.vue";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

describe("概览摘要与区域索引", () => {
  it("摘要是一个工作面：宽档三列、紧凑档单列；区域索引宽档两列、紧凑档单列", async () => {
    const { store } = await createTestStore({ width: 1440 });
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);

    const summary = wrapper.get('[data-testid="overview-summary"]');
    expect(summary.classes()).toContain("grid-cols-3");
    expect(summary.classes()).toContain("divide-x");
    expect(wrapper.get('[data-testid="region-progress"]').classes()).toContain("grid-cols-2");

    store.actions.setLayoutWidth(1000);
    await wrapper.vm.$nextTick();
    expect(wrapper.get('[data-testid="overview-summary"]').classes()).toContain("grid-cols-1");
    expect(wrapper.get('[data-testid="overview-summary"]').classes()).toContain("divide-y");
    expect(wrapper.get('[data-testid="region-progress"]').classes()).toContain("grid-cols-1");
  });

  it("完成区域显示文字与勾选，不只把进度条变绿", async () => {
    const { store } = await createTestStore({ width: 1440 });
    await analyzeCandidate(store, "save1.dat");
    const complete = store.plan.value.regions.find(
      (region) => region.kind === "main" && region.total > 0 && region.collected === region.total,
    );
    expect(complete).toBeTruthy();

    const wrapper = mountWithStore(OverviewPage, store);
    const row = wrapper.get(`[data-testid="overview-region-${complete?.slug}"]`);
    expect(row.text()).toContain("已全部收集");
    expect(row.find("svg").exists()).toBe(true);
  });

  it("主区域与特殊区域共用区域入口结构，但保持不同的定位前缀", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(OverviewPage, store);

    expect(wrapper.findAll('[data-testid^="overview-region-"]').length).toBe(9);
    expect(wrapper.findAll('[data-testid^="overview-special-"]').length).toBe(3);
    expect(wrapper.text()).toContain("特殊区域（3）");
  });
});

describe("全收集必收工作带", () => {
  it("必收工作带使用旧金进度，保留 352/352 与地图 / 合集 / 子关卡口径", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(FullCollectionPage, store);

    const band = wrapper.get('[data-testid="required-band"]');
    expect(band.text()).toContain("必收");
    expect(band.text()).toContain("缺失");
    const coverage = wrapper.get('[data-testid="guidance-coverage"]');
    expect(coverage.text()).toContain("必收指引 352/352");
    expect(coverage.text()).toContain("地图 243");
    expect(coverage.text()).toContain("合集 32");
    expect(coverage.text()).toContain("子关卡 77");
    expect(coverage.find(".ui-progress__fill--gold").exists()).toBe(true);
  });

  it("矩阵用可换行的类别按钮，不用中点串挤压", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(FullCollectionPage, store);

    const row = wrapper.get('[data-testid="required-region-paititi"]');
    expect(row.findAll("button").length).toBeGreaterThan(0);
    expect(row.text()).not.toContain(" · ");
  });
});

describe("导入文件表格与用途工作面", () => {
  it("文件表格有边界，选中行使用浅青底与左侧标记", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SlotPicker, store);
    expect(wrapper.find('[data-testid="file-table"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="purpose-panel"]').exists()).toBe(true);

    await wrapper.findAll('input[name="candidate"]')[0].setValue();
    await flushPromises();
    const selectedRow = wrapper
      .get('[data-testid="file-table"]')
      .findAll("tr")
      .find((row) => row.classes().includes("bg-accent-bg"));
    expect(selectedRow).toBeTruthy();
    expect(selectedRow?.find("td").classes()).toContain("border-accent");
  });
});
