// store 状态机测试（SX-015 验收：切换槽位和失败刷新不串用报告；取消不产生新报告）。
import { describe, expect, it } from "vitest";
import { createTestStore, analyzeCandidate, candidatePath, ControlledSource, currentEnvelope } from "./helpers";

describe("store：请求隔离与状态流", () => {
  it("首次分析成功进入概览并记录当前快照", async () => {
    const { store } = await createTestStore();
    expect(store.state.discovery?.game_roots.length).toBeGreaterThan(0);
    store.actions.setView("import");
    await analyzeCandidate(store, "save1.dat");
    expect(store.state.view).toBe("overview");
    expect(store.currentSlot.value?.result?.freshness).toBe("current");
    expect(store.report.value?.observations.overall_completion.value).toBeCloseTo(66.20481872558594, 6);
    // 模拟模式使用固定观察集合（demoObservedIds）：计划可复现。
    expect(store.planAvailable.value).toBe(true);
    expect(store.planCounts.value.all).toBe(470);
    expect(store.planCounts.value.uncollected).toBe(157);
  });

  it("未确认用途时不能开始分析", async () => {
    const { store } = await createTestStore();
    store.actions.selectCandidate(candidatePath("save1.dat"));
    await store.actions.startAnalysis();
    expect(store.state.analysis).toBeNull();
    expect(store.state.notice?.text).toContain("请先确认这个文件用于");
  });

  it("切槽后晚到的旧结果不提交，也不出现在目标槽位", async () => {
    const source = new ControlledSource();
    const { store } = await createTestStore({ source });
    store.actions.selectCandidate(candidatePath("save1.dat"));
    store.actions.setPurpose("campaign");
    void store.actions.startAnalysis();
    expect(source.pendingCount).toBe(1);
    const oldRequest = source.lastRequest().requestId;

    // 切槽到另一个候选：旧请求被撤销提交资格并请求取消。
    store.actions.switchSlot(candidatePath("save204.dat"));
    expect(source.cancelledIds).toContain(oldRequest);
    expect(store.state.analysis).toBeNull();

    // 旧请求晚到成功：必须被丢弃（A 槽结果不能出现在 B 槽标题下）。
    source.resolvePending(currentEnvelope(oldRequest, "save1.dat", "snap-old"));
    await Promise.resolve();
    expect(store.state.slots[candidatePath("save1.dat")]?.result ?? null).toBeNull();
    expect(store.currentSlot.value).toBeNull();
    expect(store.report.value).toBeNull();
  });

  it("取消不产生新报告；旧请求晚到也不恢复成功状态", async () => {
    const source = new ControlledSource();
    const { store } = await createTestStore({ source });
    store.actions.selectCandidate(candidatePath("save1.dat"));
    store.actions.setPurpose("campaign");
    void store.actions.startAnalysis();
    const first = source.lastRequest().requestId;
    store.actions.cancelAnalysis();
    expect(store.state.analysis).toBeNull();
    expect(store.state.notice?.text).toContain("已取消");

    // 取消后的晚到成功结果被丢弃。
    source.resolvePending(currentEnvelope(first, "save1.dat", "snap-late"));
    await Promise.resolve();
    expect(store.currentSlot.value?.result ?? null).toBeNull();

    // 新请求正常提交。
    void store.actions.startAnalysis();
    const second = source.lastRequest().requestId;
    expect(second).not.toBe(first);
    source.resolvePending(currentEnvelope(second, "save1.dat", "snap-second"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(store.currentSlot.value?.result?.snapshot_id).toBe("snap-second");
  });

  it("刷新失败保留旧报告并标记 stale；输入摘要与新错误并存", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const before = store.currentSlot.value?.result;
    expect(before?.snapshot_id).toBeTruthy();
    const oldDigest = store.report.value?.input_digest.compressed_sha256;

    await store.actions.refresh(); // 模拟 save1 的第一次刷新失败
    const after = store.currentSlot.value?.result;
    expect(after?.freshness).toBe("stale");
    expect(store.isStale.value).toBe(true);
    expect(after?.snapshot_id).toBe(before?.snapshot_id);
    expect(after?.last_refresh_error?.code).toBe("INPUT_CHANGED");
    expect(store.report.value?.input_digest.compressed_sha256).toBe(oldDigest);

    // 再次刷新成功，过期状态解除。
    await store.actions.refresh();
    expect(store.currentSlot.value?.result?.freshness).toBe("current");
    expect(store.isStale.value).toBe(false);
  });

  it("无效的区域偏好回退到全部区域并说明原因", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    store.actions.setRegion("no_such_region");
    await analyzeCandidate(store, "save204.dat");
    expect(store.state.region).toBe("all");
    expect(store.state.notice?.text).toContain("已切换到“全部区域”");
  });

  it("资料文件不产生进度推定（不显示 0%）", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "profile.dat");
    const report = store.report.value;
    expect(report?.context.mode).toBe("profile");
    expect(report?.observations.overall_completion.value).toBeNull();
    expect(report?.observations.overall_completion.reason).toBeTruthy();
    expect(store.planAvailable.value).toBe(false);
    expect(store.planCounts.value.all).toBe(0);
  });
});
