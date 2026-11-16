// 分享报告（P1-5）：概览入口、对话框、导出内容与打开目录。
import { describe, expect, it } from "vitest";
import OverviewPage from "../src/components/OverviewPage.vue";
import ShareReportDialog from "../src/components/ShareReportDialog.vue";
import { MockDataSource } from "../src/data/mock";
import { mockPickedExportParent } from "../src/data/mockInputs";
import { analyzeCandidate, candidatePath, createTestStore, mountWithStore } from "./helpers";

async function analyzedReport() {
  const source = new MockDataSource({ delayMs: 0 });
  const { store } = await createTestStore({ source });
  await analyzeCandidate(store, "save1.dat");
  return { store, source };
}

describe("分享报告导出", () => {
  it("概览入口打开对话框；报告含摘要与固定分享口径说明", async () => {
    const { store } = await analyzedReport();
    const wrapper = mountWithStore(OverviewPage, store);
    await wrapper.find('[data-testid="open-share-report"]').trigger("click");
    expect(store.state.shareReportState.open).toBe(true);

    mountWithStore(ShareReportDialog, store);
    await new Promise((resolve) => setTimeout(resolve, 0));
    const summary = document.querySelector('[data-testid="share-report-summary"]')?.textContent ?? "";
    expect(summary).toContain("主区域");
    expect(summary).toContain("缺失必收");
    const redaction = document.querySelector('[data-testid="share-report-redaction"]')?.textContent ?? "";
    expect(redaction).toContain("不含存档路径");
    expect(redaction).toContain("地图缩略图");
  });

  it("导出 share.html：内容不含输入文件名/路径，成功后显示产物并可打开目录", async () => {
    const { store, source } = await analyzedReport();
    store.actions.openShareReportDialog();
    store.actions.setShareReportParent(mockPickedExportParent);
    store.actions.setShareReportFolder("分享-测试");
    await store.actions.runShareReport();

    const state = store.state.shareReportState;
    expect(state.status).toBe("success");
    expect(state.outcome?.share).toBe(true);
    expect(state.outcome?.artifacts.map((artifact) => artifact.name)).toEqual(["share.html"]);
    expect(state.outcome?.output_dir).toBe(`${mockPickedExportParent}\\分享-测试`);

    expect(source.shareReports).toHaveLength(1);
    const html = source.shareReports[0].html;
    expect(html).toContain("进度分享");
    expect(html).toContain("必收缺失清单");
    expect(html).not.toContain("save1.dat");
    expect(html).not.toContain(mockPickedExportParent);
    expect(html).not.toContain("http://");
    expect(html).not.toContain("https://");

    await store.actions.openExportFolder(state.outcome!.output_dir);
    expect(source.openedExportDirs).toEqual([state.outcome!.output_dir]);
  });

  it("对话框组件：默认目录名、目标路径与导出成功面板", async () => {
    const { store } = await analyzedReport();
    mountWithStore(ShareReportDialog, store);
    store.actions.openShareReportDialog();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(store.state.shareReportState.folder).toMatch(/^sottr-share-\d{8}-\d{6}$/);
    expect(store.shareReportTargetPath.value).toBeNull();

    store.actions.setShareReportParent(mockPickedExportParent);
    const folder = document.querySelector<HTMLInputElement>('[data-testid="share-report-folder"]');
    folder!.value = "分享-组件";
    folder!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLElement>('[data-testid="share-report-submit"]')?.click();
    await new Promise((resolve) => setTimeout(resolve, 0));
    await new Promise((resolve) => setTimeout(resolve, 0));
    const success = document.querySelector('[data-testid="share-report-success"]');
    expect(success?.textContent).toContain("share.html");
    expect(success?.textContent).toContain(`${mockPickedExportParent}\\分享-组件\\share.html`);
  });

  it("资料文件不能生成分享报告（入口禁用并解释）", async () => {
    const source = new MockDataSource({ delayMs: 0 });
    const { store } = await createTestStore({ source });
    // 选择并分析资料文件：计划不可用，分享报告被拒绝。
    store.actions.selectCandidate(candidatePath("profile.dat"));
    store.actions.setPurpose("unknown");
    await store.actions.startAnalysis();
    store.actions.openShareReportDialog();
    expect(store.state.shareReportState.open).toBe(false);
    expect(store.state.notice?.text).toContain("资料文件");
  });
});
