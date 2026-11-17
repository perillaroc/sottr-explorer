// 诊断页（2026-10-17 重设计）：承接从详情面板移出的「当前选中条目的原始证据」。
import { describe, expect, it } from "vitest";
import DiagnosticsPage from "../src/components/DiagnosticsPage.vue";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

describe("诊断：当前选中条目的原始证据", () => {
  it("图鉴口径显示特殊区域与挑战古墓共用 ID", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(DiagnosticsPage, store);
    const text = wrapper.get('[data-testid="diagnostics-catalog-scope"]').text();
    expect(text).toContain("57 条");
    expect(text).toContain("23 条有子关卡");
    expect(text).toContain("34 条无 unit");
    expect(text).toContain("8 个 ID / 12 个名称");
    expect(text).toContain("0xC9D62DEE");
    expect(text).toContain("不逐项拆分");
  });

  it("未选中条目时显示空状态提示", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(DiagnosticsPage, store);
    expect(wrapper.get('[data-testid="selected-evidence"]').text()).toContain("未选中条目");
  });

  it("选中后显示图鉴版本、区域绑定、子关卡、坐标空间、获取规则与状态判定", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const target = store.plan.value.regions
      .flatMap((region) => region.groups.flatMap((group) => group.items))
      .find((item) => item.position && item.unit && item.status === "not_collected");
    expect(target).toBeDefined();
    store.actions.openRow(target!.id);
    const wrapper = mountWithStore(DiagnosticsPage, store);
    const section = wrapper.get('[data-testid="selected-evidence"]');
    const text = section.text();
    expect(text).toContain(target!.display_name);
    expect(text).toContain("图鉴版本");
    expect(text).toContain("绑定签名距离");
    expect(text).toContain("子关卡");
    expect(text).toContain("unit_normalized_unverified");
    expect(text).toContain("获取方式规则");
    expect(section.get('[data-testid="selected-status-reason"]').text()).toContain(
      "未出现在当前存档的物品表中",
    );
  });

  it("已收集条目的状态判定说明与未收集相反", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const collected = store.plan.value.regions
      .flatMap((region) => region.groups.flatMap((group) => group.items))
      .find((item) => item.status === "collected");
    expect(collected).toBeDefined();
    store.actions.openRow(collected!.id);
    const wrapper = mountWithStore(DiagnosticsPage, store);
    expect(wrapper.get('[data-testid="selected-status-reason"]').text()).toContain(
      "出现在当前存档的物品表中",
    );
  });
});
