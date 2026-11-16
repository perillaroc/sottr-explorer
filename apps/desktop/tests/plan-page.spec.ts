// 收集计划页（P1-2）：汇总、筛选、导出与窄窗可用性。
//
// 页面只读取 store 的 `planChecklist`（由 `buildPlan` 派生），本测试验证
// “这次去哪里收”的交互与导出内容，不重复核心口径（见 plan-checklist.spec.ts）。
import { describe, expect, it } from "vitest";
import PlanPage from "../src/components/PlanPage.vue";
import PlanExportDialog from "../src/components/PlanExportDialog.vue";
import { MockDataSource } from "../src/data/mock";
import { mockPickedExportParent } from "../src/data/mockInputs";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

async function analyzedPlan(width = 1280) {
  const source = new MockDataSource({ delayMs: 0 });
  const { store } = await createTestStore({ source, width });
  await analyzeCandidate(store, "save1.dat");
  store.actions.setView("plan");
  return { store, source };
}

describe("收集计划页", () => {
  it("显示汇总与未收集条目（只列未收集）", async () => {
    const { store } = await analyzedPlan();
    const wrapper = mountWithStore(PlanPage, store);
    const checklist = store.planChecklist.value;
    expect(checklist.items).toBeGreaterThan(0);
    expect(wrapper.find('[data-testid="plan-page"]').exists()).toBe(true);
    const summary = wrapper.find('[data-testid="plan-summary"]').text();
    expect(summary).toContain(String(checklist.items));
    expect(summary).toContain(String(checklist.required));
    expect(wrapper.findAll('[data-testid^="plan-item-"]')).toHaveLength(checklist.items);
    // 已收集条目不在清单里。
    const firstRegion = checklist.regions[0];
    const firstUnit = firstRegion.units[0];
    const firstItem = firstUnit.groups[0].items[0];
    expect(store.plan.value.regions
      .flatMap((region) => region.groups.flatMap((group) => group.items))
      .find((item) => item.id === firstItem.id)?.status).toBe("not_collected");
  });

  it("口径筛选：仅必收 → 全部未收集；区域与类别筛选互相独立", async () => {
    const { store } = await analyzedPlan();
    const wrapper = mountWithStore(PlanPage, store);
    const requiredCount = store.planChecklist.value.items;

    await wrapper.find('[data-testid="plan-category-chips"] button').trigger("click");
    store.actions.setPlanScope("all");
    await wrapper.vm.$nextTick();
    const allCount = store.planChecklist.value.items;
    expect(allCount).toBeGreaterThan(requiredCount);
    expect(wrapper.findAll('[data-testid^="plan-item-"]')).toHaveLength(allCount);

    store.actions.setPlanRegion("mission_of_san_juan");
    store.actions.setPlanCategory(null);
    await wrapper.vm.$nextTick();
    const sections = wrapper.findAll('section[data-testid^="plan-region-"]');
    expect(sections).toHaveLength(1);
    expect(store.state.planRegion).toBe("mission_of_san_juan");

    const categories = store.planCategoryOptions.value;
    const target = categories.find((entry) => entry.id === "monoliths") ?? categories[0];
    store.actions.setPlanCategory(target.id);
    await wrapper.vm.$nextTick();
    expect(store.planChecklist.value.regions.every((region) =>
      region.units.every((unit) => unit.groups.every((group) => group.category === target.id)),
    )).toBe(true);
  });

  it("导出：按当前筛选生成 HTML/CSV，分享模式不含输入文件名；可打开导出目录", async () => {
    const { store, source } = await analyzedPlan();
    store.actions.setPlanRegion("mission_of_san_juan");
    const checklist = store.planChecklist.value;
    mountWithStore(PlanExportDialog, store);
    store.actions.openPlanExportDialog();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector('[data-testid="plan-export-scope"]')?.textContent).toContain("圣胡安");

    store.actions.setPlanExportParent(mockPickedExportParent);
    store.actions.setPlanExportFolder("计划-导出");
    document.querySelector<HTMLElement>('[data-testid="plan-export-submit"]')?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await new Promise((resolve) => setTimeout(resolve, 0));

    const state = store.state.planExportState;
    expect(state.status).toBe("success");
    expect(state.outcome?.artifacts.map((artifact) => artifact.name)).toEqual(["plan.html", "plan.csv"]);
    expect(state.outcome?.output_dir).toBe(`${mockPickedExportParent}\\计划-导出`);

    expect(source.planExports).toHaveLength(1);
    const request = source.planExports[0];
    expect(request.share).toBe(true);
    expect(request.html).not.toBeNull();
    expect(request.csv).not.toBeNull();
    expect(request.html).not.toContain("save1.dat");
    expect(request.html).toContain('<meta charset="utf-8">');
    expect(request.csv?.startsWith("﻿")).toBe(true);
    expect(request.csv?.split("\r\n").length).toBe(checklist.items + 2);
    const firstItem = checklist.regions[0].units[0].groups[0].items[0];
    expect(request.html).toContain(firstItem.id);
    expect(request.csv).toContain(firstItem.id);

    const success = document.querySelector('[data-testid="plan-export-success"]');
    expect(success?.textContent).toContain("plan.html");
    expect(success?.textContent).toContain(`${mockPickedExportParent}\\计划-导出\\plan.html`);

    await store.actions.openExportFolder(state.outcome!.output_dir);
    expect(source.openedExportDirs).toEqual([state.outcome!.output_dir]);
  });

  it("取消勾选分享脱敏：产物写入输入文件名与快照时间", async () => {
    const { store, source } = await analyzedPlan();
    store.actions.openPlanExportDialog();
    store.actions.setPlanExportShare(false);
    store.actions.setPlanExportParent(mockPickedExportParent);
    store.actions.setPlanExportFolder("计划-完整");
    await store.actions.runPlanExport();
    expect(store.state.planExportState.status).toBe("success");
    expect(source.planExports[0].html).toContain("save1.dat");
  });

  it("没有报告时导出入口禁用并解释原因", async () => {
    const { store } = await createTestStore();
    expect(store.planChecklist.value.items).toBe(0);
    store.actions.openPlanExportDialog();
    expect(store.state.planExportState.open).toBe(false);
    expect(store.state.notice?.text).toContain("没有可导出的报告");
  });

  it("窄窗：条目单列、操作按钮仍可聚焦且不禁用", async () => {
    const { store } = await analyzedPlan(700);
    const wrapper = mountWithStore(PlanPage, store);
    expect(wrapper.find('[data-testid="plan-page"]').exists()).toBe(true);
    const item = store.planChecklist.value.regions[0].units[0].groups[0].items[0];
    const row = wrapper.find(`[data-testid="plan-item-${item.id}"]`);
    expect(row.exists()).toBe(true);
    const buttons = row.findAll("button");
    expect(buttons.length).toBeGreaterThan(0);
    for (const button of buttons) {
      const element = button.element as HTMLButtonElement;
      expect(element.disabled).toBe(false);
      expect(element.getAttribute("tabindex")).not.toBe("-1");
    }
    expect(wrapper.text()).toContain("窄窗");
  });
});
