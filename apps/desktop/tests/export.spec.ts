// 导出流程测试（SX-016）：JSON/CSV、分享脱敏、目标新目录、成功显示实际目标、
// 失败给原因与恢复、取消静默返回、旧快照必须携带过期标记。
import { describe, expect, it } from "vitest";
import ExportDialog from "../src/components/ExportDialog.vue";
import { createTestStore, analyzeCandidate, mountWithStore, MOCK_ROOT_A } from "./helpers";
import { mockPickedExportParent } from "../src/data/mockInputs";

async function analyzedStore() {
  const { store } = await createTestStore();
  await analyzeCandidate(store, "save1.dat");
  return { store };
}

describe("导出（store 与对话框）", () => {
  it("没有报告时不能打开导出（入口禁用并解释原因）", async () => {
    const { store } = await createTestStore();
    expect(store.canExport.value).toBe(false);
    store.actions.openExportDialog();
    expect(store.state.exportState.open).toBe(false);
    expect(store.state.notice?.text).toContain("没有可导出的报告");
  });

  it("分析进行中不能导出（避免与快照不一致）", async () => {
    const { store } = await analyzedStore();
    expect(store.canExport.value).toBe(true);
    store.state.analysis = { requestId: "req-x", path: "x", kind: "refresh", startedAt: 0 };
    store.actions.openExportDialog();
    expect(store.state.exportState.open).toBe(false);
    expect(store.state.notice?.text).toContain("分析/刷新进行中");
    store.state.analysis = null;
  });

  it("默认分享脱敏、JSON+CSV；选择位置后形成新目录目标", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    expect(store.state.exportState.format).toBe("both");
    expect(store.state.exportState.share).toBe(true);
    expect(store.state.exportState.folder).toMatch(/^sottr-explorer-\d{8}-\d{6}$/);
    expect(store.exportTargetPath.value).toBeNull();

    store.actions.setExportFolder("导出-01");
    await store.actions.pickExportParent();
    expect(store.state.exportState.parent).toBe(mockPickedExportParent);
    expect(store.exportTargetPath.value).toBe(`${mockPickedExportParent}\\导出-01`);
  });

  it("导出成功显示实际目标；仅 CSV 时只产生 CSV", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("导出-成功");
    store.actions.setExportFormat("csv");
    await store.actions.runExport();

    const state = store.state.exportState;
    expect(state.status).toBe("success");
    expect(state.outcome?.share).toBe(true);
    expect(state.outcome?.artifacts.map((artifact) => artifact.name)).toEqual(["report.csv"]);
    expect(state.outcome?.artifacts[0].path).toBe(`${mockPickedExportParent}\\导出-成功\\report.csv`);
    expect(store.state.notice?.text).toContain("已导出");
  });

  it("分享脱敏可关闭（显式操作），导出参数随之变化", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("导出-完整");
    store.actions.setExportShare(false);
    await store.actions.runExport();
    expect(store.state.exportState.outcome?.share).toBe(false);
  });

  it("目标目录已存在：报 OUTPUT_EXISTS、给建议目录名，一键换名后重试成功", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("导出-重名");
    await store.actions.runExport();
    expect(store.state.exportState.status).toBe("success");

    store.actions.openExportDialog();
    store.actions.setExportFolder("导出-重名");
    await store.actions.runExport();
    expect(store.state.exportState.status).toBe("error");
    expect(store.state.exportState.error?.code).toBe("OUTPUT_EXISTS");
    expect(store.state.exportState.suggestedFolder).toMatch(/^sottr-explorer-/);

    store.actions.applySuggestedExportFolder();
    expect(store.state.exportState.status).toBe("edit");
    expect(store.state.exportState.folder).not.toBe("导出-重名");
    await store.actions.runExport();
    expect(store.state.exportState.status).toBe("success");
  });

  it("目标位于存档目录内：报 INPUT_OUTPUT_OVERLAP（不覆盖存档）", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(MOCK_ROOT_A);
    store.actions.setExportFolder("导出-重叠");
    await store.actions.runExport();
    expect(store.state.exportState.error?.code).toBe("INPUT_OUTPUT_OVERLAP");
  });

  it("非法目录名与不存在的父级各有稳定错误码", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("非法:名称");
    await store.actions.runExport();
    expect(store.state.exportState.error?.code).toBe("OUTPUT_INVALID_TARGET");

    store.actions.setExportFolder("正常名称");
    store.actions.setExportParent("C:\\不存在的目录");
    await store.actions.runExport();
    expect(store.state.exportState.error?.code).toBe("PATH_MISSING");
  });

  it("旧快照导出：携带 stale 与刷新错误码（对话框常驻过期提示）", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    await store.actions.refresh(); // 模拟刷新失败 → stale
    expect(store.isStale.value).toBe(true);

    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("导出-过期");
    await store.actions.runExport();
    const outcome = store.state.exportState.outcome;
    expect(outcome?.freshness).toBe("stale");
    expect(outcome?.last_refresh_error_code).toBe("INPUT_CHANGED");
    expect(store.state.notice?.text).toContain("旧快照");

    const wrapper = mountWithStore(ExportDialog, store);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector('[data-testid="export-dialog"]')).not.toBeNull();
    expect(document.querySelector('[data-testid="export-stale-notice"]')?.textContent).toContain(
      "导出会携带过期标记",
    );
    expect(document.querySelector('[data-testid="export-success"]')?.textContent).toContain("旧快照");
    void wrapper;
  });

  it("取消导出静默返回：不产生文件、不改动已有产物", async () => {
    const { store } = await analyzedStore();
    store.actions.openExportDialog();
    store.actions.setExportParent(mockPickedExportParent);
    store.actions.setExportFolder("导出-取消");
    store.actions.closeExportDialog();
    expect(store.state.exportState.open).toBe(false);
    // 取消后没有产物：再次用同名目录导出仍然成功（说明取消没有创建任何目录）。
    store.actions.openExportDialog();
    store.actions.setExportFolder("导出-取消");
    await store.actions.runExport();
    expect(store.state.exportState.status).toBe("success");
  });

  it("对话框组件：表单控件驱动 store，成功面板显示实际目标", async () => {
    const { store } = await analyzedStore();
    mountWithStore(ExportDialog, store);
    store.actions.openExportDialog();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector('[data-testid="export-dialog"]')).not.toBeNull();

    // 通过真实 DOM 事件驱动表单（对话框 Teleport 到 body，与用户操作一致）。
    const parentInput = document.querySelector<HTMLInputElement>('[data-testid="export-parent"]');
    parentInput!.value = mockPickedExportParent;
    parentInput!.dispatchEvent(new Event("input", { bubbles: true }));
    const folderInput = document.querySelector<HTMLInputElement>('[data-testid="export-folder"]');
    folderInput!.value = "组件导出";
    folderInput!.dispatchEvent(new Event("input", { bubbles: true }));
    const jsonRadio = document.querySelector<HTMLInputElement>('[data-testid="export-format-json"]');
    jsonRadio!.click();
    expect(store.state.exportState.format).toBe("json");
    const shareBox = document.querySelector<HTMLInputElement>('[data-testid="export-share"]');
    shareBox!.click();
    expect(store.state.exportState.share).toBe(false);

    document.querySelector<HTMLElement>('[data-testid="export-submit"]')?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await new Promise((resolve) => setTimeout(resolve, 0));
    const success = document.querySelector('[data-testid="export-success"]');
    expect(success?.textContent).toContain("report.json");
    expect(success?.textContent).toContain(`${mockPickedExportParent}\\组件导出\\report.json`);
    expect(success?.textContent).toContain("分享脱敏：关闭");
  });

  it("对话框组件：失败面板显示错误码、原因与恢复建议", async () => {
    const { store } = await analyzedStore();
    mountWithStore(ExportDialog, store);
    store.actions.openExportDialog();
    // 目标位于“输入存档所在目录”内 → 重叠错误 + 恢复建议
    store.actions.setExportParent(MOCK_ROOT_A);
    store.actions.setExportFolder("组件失败");
    await new Promise((resolve) => setTimeout(resolve, 0));
    document.querySelector<HTMLElement>('[data-testid="export-submit"]')?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await new Promise((resolve) => setTimeout(resolve, 0));
    const box = document.querySelector('[data-testid="export-error"]');
    expect(box?.textContent).toContain("INPUT_OUTPUT_OVERLAP");
    expect(box?.textContent).toContain("把导出目标移出存档目录");
  });
});
