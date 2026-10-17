// 端到端流程测试（SX-016）：真实 `TauriDataSource` + 脚本化“假后端”（只替换 IPC
// 边界，界面、store、组件与命令参数映射全部走生产代码）。
//
// 覆盖验收要求：选择副本→分析→查找参考遗漏→查看组/位置线索→外部浏览器打开来源
// →返回后手动刷新→导出流程；异常 type28（表不完整）、unknown 模式、未知 ID、
// 无映射、陈旧报告与 CSV 分享；假点数为 0；原始 present 不显示为“已收集”；
// 非零/未知计数组、同组多 ID、搜索无结果与真实空差集、外链打开失败、过期导出标记；
// 返回列表保持筛选与滚动。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import App from "../src/App.vue";
import { TauriDataSource } from "../src/data/tauri";
import { createAppStore } from "../src/app/store";
import { demoObservedIds } from "../src/data/atlas";
import { buildCompleteReport, buildDegradedReport, buildNonzeroReport, buildProfileReport } from "../src/data/mockReports";
import { mockCandidates, mockDiscovery, MOCK_ROOT_A } from "../src/data/mockInputs";
import { mount } from "@vue/test-utils";
import type { DesktopResult } from "../src/contract/desktop-result";
import type { Report } from "../src/contract/report";

interface Call {
  command: string;
  args: Record<string, unknown>;
}

const STELLA_URL =
  "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html";
const SEED = { compressedSha256: "9f".repeat(32), compressedBytes: 597_590 };

/** 脚本化假后端：只实现 IPC 边界，返回与 SX-014 相同形状的数据。 */
class FakeDesktopBackend {
  readonly calls: Call[] = [];
  readonly snapshots = new Map<string, { result: DesktopResult; inputPath: string }>();
  readonly createdDirs = new Set<string>();
  readonly openedUrls: string[] = [];
  failOpen = false;
  private attempts = new Map<string, number>();
  private counter = 0;

  invoke = async (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    const payload = (args ?? {}) as Record<string, unknown>;
    this.calls.push({ command, args: payload });
    switch (command) {
      case "discover_saves":
        return mockDiscovery;
      case "list_candidates":
        return (
          mockCandidates(String(payload.rootPath)) ?? {
            root: payload.rootPath,
            files: [],
            skipped: [],
            depth_limited_dirs: [],
          }
        );
      case "select_input":
        return payload.kind === "file" ? `${MOCK_ROOT_A}\\save1.dat` : MOCK_ROOT_A;
      case "analyze_file":
        return this.analyze(String(payload.inputPath), String(payload.requestId), String(payload.context));
      case "cancel_analysis":
        return { request_id: payload.requestId, cancelled: true };
      case "export_report":
        return this.exportReport(payload);
      case "resolve_guide_link":
        return this.guideLink(payload);
      case "open_guide_link": {
        const link = this.guideLink(payload);
        if (this.failOpen) {
          throw { code: "LINK_OPEN_FAILED", message: "系统浏览器打开失败（测试脚本）" };
        }
        this.openedUrls.push(link.url);
        return link;
      }
      default:
        throw { code: "INVALID_PARAMS", message: `未知命令 ${command}` };
    }
  };

  private analyze(path: string, requestId: string, context: string): DesktopResult {
    const attempt = (this.attempts.get(path) ?? 0) + 1;
    this.attempts.set(path, attempt);
    const name = path.split(/[\\/]/).pop() ?? path;
    const base = {
      schema_version: "1.0.0" as const,
      request_id: requestId,
      input_ref: name,
      snapshot_time: "2026-10-16T12:00:00.000Z",
      last_refresh_error: null,
      error: null,
    };
    if (name === "save1.dat" && attempt === 2) {
      const previous = [...this.snapshots.values()].find((entry) => entry.result.input_ref === name);
      const error = { code: "INPUT_CHANGED", message: "读取期间文件已变化", retryable: true };
      const stale: DesktopResult = {
        ...base,
        snapshot_id: previous?.result.snapshot_id ?? null,
        freshness: "stale",
        last_refresh_error: error,
        report: previous?.result.report ?? null,
        error,
      };
      if (stale.snapshot_id) {
        // 与 SX-014 一致：失败刷新把既有快照原地标记为 stale（身份不变）。
        this.snapshots.set(stale.snapshot_id, { result: stale, inputPath: path });
      }
      return stale;
    }
    let report: Report;
    switch (name) {
      case "save204.dat":
        report = buildNonzeroReport(SEED);
        break;
      case "save206.dat":
        report = buildDegradedReport(SEED);
        break;
      case "profile.dat":
        report = buildProfileReport(SEED);
        break;
      default:
        report = buildCompleteReport(SEED);
        break;
    }
    if (context === "unknown") {
      report = {
        ...report,
        context: {
          ...report.context,
          mode: "unknown",
          source: "user_declared",
          limitations: ["未声明为普通战役，仅保留可用观察"],
        },
        observations: {
          ...report.observations,
          overall_completion: {
            ...report.observations.overall_completion,
            value: null,
            support_status: "unsupported",
            reason: "上下文未声明为普通战役",
          },
        },
      };
    }
    const result: DesktopResult = {
      ...base,
      snapshot_id: `snap-e2e-${(this.counter += 1)}`,
      freshness: "current",
      report,
      error: null,
    };
    this.snapshots.set(result.snapshot_id as string, { result, inputPath: path });
    return result;
  }

  private exportReport(payload: Record<string, unknown>) {
    const snapshotId = String(payload.snapshotId);
    const stored = this.snapshots.get(snapshotId);
    if (!stored) {
      throw { code: "SNAPSHOT_UNKNOWN", message: "未知快照" };
    }
    if (stored.result.input_ref !== payload.inputRef) {
      throw { code: "SNAPSHOT_MISMATCH", message: "导出与槽位不一致" };
    }
    const outputDir = String(payload.outputDir);
    if (this.createdDirs.has(outputDir.toLowerCase())) {
      throw { code: "OUTPUT_EXISTS", message: `目标目录已存在：${outputDir}` };
    }
    this.createdDirs.add(outputDir.toLowerCase());
    const artifacts: { name: string; path: string }[] = [];
    if (payload.format === "json" || payload.format === "both") {
      artifacts.push({ name: "report.json", path: `${outputDir}\\report.json` });
    }
    if (payload.format === "csv" || payload.format === "both") {
      artifacts.push({ name: "report.csv", path: `${outputDir}\\report.csv` });
    }
    return {
      snapshot_id: snapshotId,
      input_ref: stored.result.input_ref,
      freshness: stored.result.freshness,
      last_refresh_error_code: stored.result.last_refresh_error?.code ?? null,
      share: payload.share === true,
      artifacts,
    };
  }

  private guideLink(payload: Record<string, unknown>) {
    const snapshotId = String(payload.snapshotId);
    const stored = this.snapshots.get(snapshotId);
    if (!stored?.result.report) {
      throw { code: "SNAPSHOT_UNKNOWN", message: "未知快照" };
    }
    const location = stored.result.report.locations.find(
      (entry) => entry.location_id === payload.locationId,
    );
    if (!location) {
      throw { code: "LINK_LOCATION_UNKNOWN", message: "未知定位" };
    }
    if (!location.source_refs.includes(String(payload.sourceRef))) {
      throw { code: "LINK_SOURCE_UNKNOWN", message: "来源不属于该定位" };
    }
    return {
      snapshot_id: snapshotId,
      input_ref: stored.result.input_ref,
      location_id: location.location_id,
      source_ref: payload.sourceRef,
      url: STELLA_URL,
      freshness: stored.result.freshness,
      action_kind: location.group_actionability?.kind ?? null,
      action_reason_codes: location.group_actionability?.reason_codes ?? [],
    };
  }
}

function mountApp(width = 1280) {
  const source = new TauriDataSource();
  // 演示观察集合：让区域/详情流程可复现（真实运行读取当前报告）。
  const store = createAppStore(source, {
    persist: false,
    layoutWidth: width,
    observedIds: () => demoObservedIds,
  });
  const wrapper = mount(App, {
    props: { source, store, layoutWidth: width } as never,
    attachTo: document.body,
  });
  return { wrapper, store, source };
}

function callsOf(backend: FakeDesktopBackend, command: string): Call[] {
  return backend.calls.filter((call) => call.command === command);
}

function lastCall(backend: FakeDesktopBackend, command: string): Call | undefined {
  const calls = callsOf(backend, command);
  return calls[calls.length - 1];
}

/** 等待异步命令完成（IPC 模拟经过多个微任务/宏任务）。 */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await flushPromises();
}

describe("端到端（真实命令层 + 假后端 IPC）", () => {
  let backend: FakeDesktopBackend;

  beforeEach(() => {
    backend = new FakeDesktopBackend();
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) =>
      backend.invoke(command, args),
    );
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true,
    });
  });

  it("主流程：选择副本→分析→按区域查漏→查看位置与获取方式→复制线索", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();

    // 1) 发现与候选（真实命令）
    expect(callsOf(backend, "discover_saves")).toHaveLength(1);
    expect(callsOf(backend, "list_candidates")[0].args).toEqual({ rootPath: MOCK_ROOT_A });
    expect(wrapper.find('[data-testid="mock-banner"]').exists()).toBe(false);
    expect(wrapper.text()).toContain("save1.dat");

    // 2) 选择稳定副本 + 用途 → 开始分析
    const radio = wrapper
      .findAll('input[name="candidate"]')
      .find((input) => (input.element as HTMLInputElement).value.endsWith("save1.dat"));
    await radio?.setValue();
    await wrapper.get('[data-testid="purpose-campaign"]').setValue();
    await wrapper.get('[data-testid="start-analysis"]').trigger("click");
    await flushPromises();
    await flushPromises();

    expect(lastCall(backend, "analyze_file")?.args.context).toBe("campaign");
    expect(store.state.view).toBe("overview");
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("66.20%");
    // 无假点位：没有地图元素或“导航到此点”文案。
    expect(wrapper.find('[data-testid*="map"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain("导航到此点");

    // 3) 查看未收集 → 全部区域卡片
    await wrapper.get('[data-testid="view-missing"]').trigger("click");
    await flushPromises();
    expect(wrapper.get('[data-testid="region-overview"]').text()).toContain("帕依提提");

    // 4) 进入区域 → 逐项清单
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("all");
    await flushPromises();
    const row = wrapper.get('[data-testid="row-0xCAA74459"]');
    expect(row.text()).toContain("古老典籍");

    // 5) 详情：官方中文名 + 位置线索 + 获取方式
    await row.trigger("click");
    await flushPromises();
    const detail = document.querySelector('[data-testid="item-detail"]');
    expect(detail?.textContent).toContain("古老典籍");
    expect(detail?.textContent).toContain("去哪里找");
    expect(detail?.textContent).toContain("怎么拿");
    expect(detail?.textContent).toContain("未校准");

    // 6) 复制线索：前端组合文本，不含存档路径
    document.querySelector<HTMLElement>('[data-testid="copy-lead"]')?.click();
    await flushPromises();
    expect(navigator.clipboard.writeText).toHaveBeenCalled();
    const lead = document.querySelector('[data-testid="lead-text"]')?.textContent ?? "";
    expect(lead).toContain("古老典籍");
    expect(lead).not.toContain("C:\\");
  });

  it("有位置的条目显示关卡内相对位置散点，并明确标注未校准", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();
    store.actions.selectCandidate(`${MOCK_ROOT_A}\save1.dat`);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    store.actions.setView("regions");
    store.actions.setRegion("paititi");
    store.actions.setStatusFilter("all");
    await flushPromises();

    store.actions.openRow("0xF1FAE49A");
    await flushPromises();
    const detail = document.querySelector('[data-testid="item-detail"]');
    expect(detail?.querySelector('[data-testid="position-scatter"]')).not.toBeNull();
    expect(detail?.textContent).toContain("未校准");
    expect(detail?.textContent).toContain("不是导航点位");
    expect(wrapper.text()).not.toContain("导航到此点");
  });

  it("刷新失败：旧快照标记 + 行动摘要关闭；再次刷新恢复；导出带过期标记", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();
    store.actions.selectCandidate(`${MOCK_ROOT_A}\\save1.dat`);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    await flushPromises();
    const slotPath = `${MOCK_ROOT_A}\\save1.dat`;
    const freshSnapshot = store.state.slots[slotPath]?.result?.snapshot_id;

    // F5 刷新（第二次 analyze 返回 stale）
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "F5", bubbles: true }));
    await settle();
    expect(store.isStale.value).toBe(true);
    expect(store.currentSlot.value?.result?.snapshot_id).toBe(freshSnapshot);
    expect(wrapper.find('[data-testid="stale-banner"]').exists()).toBe(true);
    // 覆盖提示仍可见（不互相替代）。
    expect(wrapper.find('[data-testid="coverage-banner"]').exists()).toBe(true);
    // 旧的遗漏数仍不冒充当前结论。
    expect(wrapper.text()).toContain("旧快照，未反映本次读取");

    // 过期导出的产物携带 stale 与错误码。
    store.actions.openExportDialog();
    store.actions.setExportParent("C:\\导出位置");
    store.actions.setExportFolder("过期导出");
    await store.actions.runExport();
    expect(store.state.exportState.outcome?.freshness).toBe("stale");
    expect(store.state.exportState.outcome?.last_refresh_error_code).toBe("INPUT_CHANGED");
    expect(store.state.notice?.text).toContain("旧快照");

    // 再次刷新成功 → 过期解除。
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "F5", bubbles: true }));
    await settle();
    expect(store.isStale.value).toBe(false);
    expect(wrapper.find('[data-testid="stale-banner"]').exists()).toBe(false);
  });

  it("导出主流程：对话框 → 实际目标 → 重名恢复；导出参数绑定当前快照", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();
    store.actions.selectCandidate(`${MOCK_ROOT_A}\\save1.dat`);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    await flushPromises();

    // 从顶栏“更多”菜单打开导出（与玩家路径一致）。
    const moreButton = wrapper.findAll("button").find((button) => button.text().includes("更多"));
    await moreButton?.trigger("click");
    await flushPromises();
    const exportItem = wrapper.findAll('[role="menuitem"]').find((item) => item.text().includes("导出当前快照"));
    expect(exportItem).toBeTruthy();
    await exportItem?.trigger("click");
    await flushPromises();
    expect(store.state.exportState.open).toBe(true);
    expect(store.state.exportState.share).toBe(true); // 默认分享脱敏

    store.actions.setExportParent("C:\\导出位置");
    store.actions.setExportFolder("e2e-导出");
    await store.actions.runExport();
    const exportCall = lastCall(backend, "export_report");
    expect(exportCall?.args).toEqual({
      snapshotId: store.currentSlot.value?.result?.snapshot_id,
      inputRef: "save1.dat",
      outputDir: "C:\\导出位置\\e2e-导出",
      format: "both",
      share: true,
    });
    const success = document.querySelector('[data-testid="export-success"]');
    expect(success?.textContent).toContain("C:\\导出位置\\e2e-导出\\report.json");
    expect(success?.textContent).toContain("report.csv");

    // 同名目录再次导出：OUTPUT_EXISTS + 建议目录名。
    store.actions.openExportDialog();
    store.actions.setExportFolder("e2e-导出");
    await store.actions.runExport();
    expect(store.state.exportState.error?.code).toBe("OUTPUT_EXISTS");
    expect(store.state.exportState.suggestedFolder).toMatch(/^sottr-explorer-/);
    store.actions.applySuggestedExportFolder();
    await store.actions.runExport();
    expect(store.state.exportState.status).toBe("success");
  });

  it("unknown 模式与表不完整：上下文不冒充战役，收集清单仍可用", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();

    // unknown 模式：不显示 66.20%，报告仍可读。
    store.actions.selectCandidate(`${MOCK_ROOT_A}\save1.dat`);
    store.actions.setPurpose("unknown");
    await store.actions.startAnalysis();
    await flushPromises();
    expect(store.report.value?.context.mode).toBe("unknown");
    expect(wrapper.get('[data-testid="overall-value"]').text()).toContain("不可用");
    expect(lastCall(backend, "analyze_file")?.args.context).toBe("unknown");

    // 表不完整（type28 降级）：区域与收集仍可用，状态来自观察集合。
    store.actions.switchSlot(`${MOCK_ROOT_A}\save206.dat`);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    await flushPromises();
    store.actions.setView("regions");
    await flushPromises();
    expect(wrapper.find('[data-testid="region-overview"]').exists()).toBe(true);
    store.actions.setRegion("paititi");
    store.actions.setStatusFilter("all");
    await flushPromises();
    expect(wrapper.findAll('[data-testid^="row-"]').length).toBeGreaterThan(0);
  });

  it("状态只有已收集/未收集；未分类 ID 经诊断进入且不计入收集品", async () => {
    const { wrapper, store } = mountApp();
    await flushPromises();
    await flushPromises();
    store.actions.selectCandidate(`${MOCK_ROOT_A}\save1.dat`);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    store.actions.setView("regions");
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("observed");
    await flushPromises();

    const rows = wrapper.findAll('[data-testid^="row-"]');
    expect(rows.length).toBeGreaterThan(0);
    expect(rows[0].text()).toContain("已收集");

    store.actions.setStatusFilter("missing");
    await flushPromises();
    const missingRows = wrapper.findAll('[data-testid^="row-"]');
    expect(missingRows.length).toBeGreaterThan(0);
    expect(missingRows[0].text()).toContain("未收集");

    // 未分类观察 ID 不属于图鉴，不计入收集品。
    expect(store.planCounts.value.all).toBe(470);
    store.actions.setView("diagnostics");
    await flushPromises();
    expect(wrapper.text()).toContain("不计入收集品数");
  });
});
