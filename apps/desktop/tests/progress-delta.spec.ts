// 进度增量（P1-3）：store 会话历史与概览页展示。
//
// 用受控数据源精确控制两次分析的观察集合；历史只存内存，测试同时验证
// 不可比原因与“不含路径”的隐私口径。
import { describe, expect, it } from "vitest";
import OverviewPage from "../src/components/OverviewPage.vue";
import { buildDegradedReport } from "../src/data/mockReports";
import {
  candidatePath,
  ControlledSource,
  createTestStore,
  currentEnvelope,
  mountWithStore,
} from "./helpers";
import type { AppStore } from "../src/app/store";
import type { Report } from "../src/contract/report";

const SEED = { compressedSha256: "cd".repeat(32), compressedBytes: 597_590 };
const A = "0x0F28B6EC";
const B = "0x439B19A6";
const C = "0x851B4915";
const D = "0xCAA74459";

async function runAnalysis(
  store: AppStore,
  source: ControlledSource,
  options: { snapshotId: string; refresh?: boolean; inputRef?: string; report?: Report; candidate?: string },
): Promise<void> {
  if (!options.refresh) {
    store.actions.selectCandidate(candidatePath(options.candidate ?? "save1.dat"));
    store.actions.setPurpose("campaign");
  }
  const pending = options.refresh ? store.actions.refresh() : store.actions.startAnalysis();
  await Promise.resolve();
  const requestId = source.lastRequest().requestId;
  const envelope = currentEnvelope(requestId, options.inputRef ?? "save1.dat", options.snapshotId);
  source.resolvePending(options.report ? { ...envelope, report: options.report } : envelope);
  await pending;
}

describe("进度增量（store 与概览页）", () => {
  it("两次分析新增条目：历史、基线、总/区域增量与新增列表", async () => {
    let ids = [A, B];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-1" });
    expect(store.progressHistoryForCurrent.value).toHaveLength(1);
    expect(store.progressComparison.value).toBeNull();

    ids = [A, B, C];
    await runAnalysis(store, source, { snapshotId: "snap-2", refresh: true });
    expect(store.progressHistoryForCurrent.value).toHaveLength(2);
    expect(store.progressBaselineId.value).toBe("snap-1");
    const delta = store.progressComparison.value;
    expect(delta?.status).toBe("comparable");
    expect(delta?.totalDelta).toBe(1);
    expect(delta?.requiredDelta).toBe(1);
    expect(delta?.added.map((item) => item.id)).toEqual([C]);
    expect(delta?.after.snapshotTime).toBe("2026-10-16T00:00:00.000Z");

    const wrapper = mountWithStore(OverviewPage, store);
    const section = wrapper.find('[data-testid="progress-delta"]');
    expect(section.exists()).toBe(true);
    expect(section.text()).toContain("主区域收集品");
    expect(section.text()).toContain("+1");
    const added = wrapper.find('[data-testid="progress-added-item"]');
    expect(added.text()).toContain(C);
    expect(wrapper.find('[data-testid="progress-status"]').exists()).toBe(false);
    // 隐私：快照对象本身不含路径（历史映射的槽位键是会话内状态，不随快照导出）。
    const snapshots = Object.values(store.state.progressHistory).flat();
    for (const snapshot of snapshots) {
      for (const value of Object.values(snapshot)) {
        if (typeof value === "string") {
          expect(value.includes("\\")).toBe(false);
        }
      }
    }
  });

  it("可选择更早的基线；没有变化时增量为 0", async () => {
    let ids = [A, B];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-1" });
    ids = [A, B, C];
    await runAnalysis(store, source, { snapshotId: "snap-2", refresh: true });
    ids = [A, B, C, D];
    await runAnalysis(store, source, { snapshotId: "snap-3", refresh: true });
    expect(store.progressComparison.value?.added.map((item) => item.id)).toEqual([D]);

    store.actions.setProgressBaseline("snap-1");
    expect(store.progressBaselineId.value).toBe("snap-1");
    expect(store.progressComparison.value?.added.map((item) => item.id)).toEqual([C, D]);

    const wrapper = mountWithStore(OverviewPage, store);
    const select = wrapper.find('[data-testid="progress-baseline"]');
    expect(select.findAll("option")).toHaveLength(2);
    await select.setValue("snap-2");
    expect(store.progressBaselineId.value).toBe("snap-2");
    expect(store.progressComparison.value?.added.map((item) => item.id)).toEqual([D]);
    expect(wrapper.find('[data-testid="progress-added"]').text()).toContain("新增条目（1）");
  });

  it("不同存档：明确不可比，不显示新增/丢失", async () => {
    let ids = [A];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-1" });
    ids = [A, B];
    await runAnalysis(store, source, { snapshotId: "snap-2", refresh: true, inputRef: "save202.dat" });

    const delta = store.progressComparison.value;
    expect(delta?.status).toBe("different_save");
    expect(delta?.comparable).toBe(false);
    expect(delta?.added).toEqual([]);
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.find('[data-testid="progress-status"]').text()).toContain("不同存档");
    expect(wrapper.find('[data-testid="progress-added"]').exists()).toBe(false);
    // 不可比时仍显示两侧摘要（“槽位对比可见”），但不显示增量与条目列表。
    const totals = wrapper.find('[data-testid="progress-totals"]');
    expect(totals.exists()).toBe(true);
    expect(totals.text()).toContain("1 → 2");
    expect(totals.text()).not.toContain("+1");
  });

  it("物品表不完整：明确不可比，不推断新增或丢失", async () => {
    let ids = [A];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-1" });
    ids = [A, B];
    await runAnalysis(store, source, {
      snapshotId: "snap-2",
      refresh: true,
      report: buildDegradedReport(SEED),
    });

    const delta = store.progressComparison.value;
    expect(delta?.status).toBe("inventory_incomplete");
    expect(delta?.added).toEqual([]);
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.find('[data-testid="progress-status"]').text()).toContain("物品表不完整");
    expect(wrapper.find('[data-testid="progress-totals"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="progress-added"]').exists()).toBe(false);
  });

  it("槽位对比：可选择其他槽位的快照作为基线，并明确标为不同存档", async () => {
    let ids = [A];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-save1-1" });
    ids = [A, B];
    await runAnalysis(store, source, { snapshotId: "snap-save1-2", refresh: true });

    store.actions.switchSlot(candidatePath("save202.dat"));
    ids = [A];
    await runAnalysis(store, source, { snapshotId: "snap-202-1", candidate: "save202.dat", inputRef: "save202.dat" });
    ids = [A, C];
    await runAnalysis(store, source, {
      snapshotId: "snap-202-2",
      refresh: true,
      inputRef: "save202.dat",
    });

    // 基线选项包含另一个槽位的快照（排除当前槽位最新一次）。
    const options = store.progressBaselineOptions.value;
    expect(options.map((snapshot) => snapshot.inputRef)).toContain("save1.dat");
    expect(options.some((snapshot) => snapshot.snapshotId === "snap-save1-2")).toBe(true);
    expect(options.some((snapshot) => snapshot.snapshotId === "snap-202-2")).toBe(false);

    store.actions.setProgressBaseline("snap-save1-2");
    expect(store.progressBaselineId.value).toBe("snap-save1-2");
    const delta = store.progressComparison.value;
    expect(delta?.status).toBe("different_save");
    expect(delta?.added).toEqual([]);
    expect(delta?.before.inputRef).toBe("save1.dat");
    expect(delta?.after.inputRef).toBe("save202.dat");

    const wrapper = mountWithStore(OverviewPage, store);
    const status = wrapper.find('[data-testid="progress-status"]').text();
    expect(status).toContain("不同存档");
    expect(status).toContain("save1.dat");
    expect(status).toContain("save202.dat");
  });

  it("同一槽位只分析一次时，也可选择其他槽位的快照作对照", async () => {
    let ids = [A];
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => ids });
    await runAnalysis(store, source, { snapshotId: "snap-save1-1" });

    store.actions.switchSlot(candidatePath("save202.dat"));
    ids = [A, B];
    await runAnalysis(store, source, {
      snapshotId: "snap-202-1",
      candidate: "save202.dat",
      inputRef: "save202.dat",
    });

    expect(store.progressHistoryForCurrent.value).toHaveLength(1);
    expect(store.progressComparison.value).toBeNull();
    expect(store.progressBaselineOptions.value.map((snapshot) => snapshot.snapshotId)).toEqual(["snap-save1-1"]);

    store.actions.setProgressBaseline("snap-save1-1");
    const delta = store.progressComparison.value;
    expect(delta?.status).toBe("different_save");

    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.find('[data-testid="progress-empty"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="progress-baseline"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="progress-status"]').text()).toContain("不同存档");
    expect(wrapper.find('[data-testid="progress-totals"]').text()).toContain("1 → 2");
  });

  it("首次分析：提示本会话还没有上一次分析", async () => {
    const source = new ControlledSource();
    const { store } = await createTestStore({ source, observedIds: () => [A] });
    await runAnalysis(store, source, { snapshotId: "snap-1" });
    const wrapper = mountWithStore(OverviewPage, store);
    expect(wrapper.find('[data-testid="progress-empty"]').text()).toContain("还没有上一次分析");
    expect(wrapper.find('[data-testid="progress-baseline"]').exists()).toBe(false);
  });
});
