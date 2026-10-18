// 组件与 store 测试基础设施（SX-015）。
import { mount } from "@vue/test-utils";
import type { Component } from "vue";
import { storeKey } from "../src/app/inject";
import { createAppStore, type AppStore } from "../src/app/store";
import { MockDataSource, MOCK_ROOT_A, MOCK_ROOT_B } from "../src/data/mock";
import type { AnalyzeRequest, DataSource } from "../src/data/source";
import type { DesktopResult } from "../src/contract/desktop-result";
import { buildCompleteReport } from "../src/data/mockReports";

export { MOCK_ROOT_A, MOCK_ROOT_B };
export function candidatePath(name: string, root: string = MOCK_ROOT_A): string {
  return `${root}\\${name}`;
}

export interface TestStoreOptions {
  width?: number;
  source?: DataSource;
  boot?: boolean;
  /**
   * 覆盖观察集合；返回 `null` 表示改用当前报告的物品表。
   * 默认：模拟模式使用 demoObservedIds，真实模式读取报告。
   */
  observedIds?: () => string[] | null;
}

/** 创建测试 store（默认持久化关闭、MockDataSource 无延迟）。 */
export async function createTestStore(
  options: TestStoreOptions = {},
): Promise<{ store: AppStore; source: DataSource }> {
  const source = options.source ?? new MockDataSource({ delayMs: 0 });
  const store = createAppStore(source, {
    persist: false,
    layoutWidth: options.width ?? 1280,
    observedIds: options.observedIds,
  });
  if (options.boot !== false) {
    await store.actions.boot();
  }
  return { store, source };
}

/** 走完整流程分析一个候选文件（选择→用途→开始）。 */
export async function analyzeCandidate(
  store: AppStore,
  name: string,
  purpose: "campaign" | "unknown" = "campaign",
): Promise<void> {
  store.actions.selectCandidate(candidatePath(name));
  store.actions.setPurpose(purpose);
  await store.actions.startAnalysis();
}

/** 挂载组件并注入 store（attachTo body 以便焦点断言）。 */
export function mountWithStore<C extends Component>(
  component: C,
  store: AppStore,
  props: Record<string, unknown> = {},
) {
  return mount(component, {
    props: props as never,
    global: { provide: { [storeKey]: store } },
    attachTo: document.body,
  });
}

/** 受控数据源：analyze 挂起，由测试显式解决（用于晚到结果与取消语义）。 */
export class ControlledSource implements DataSource {
  readonly kind = "mock" as const;
  private pending: { request: AnalyzeRequest; resolve: (result: DesktopResult) => void }[] = [];
  readonly cancelledIds: string[] = [];

  async discover() {
    return new MockDataSource().discover();
  }

  async listCandidates(root: string) {
    return new MockDataSource().listCandidates(root);
  }

  async pick(): Promise<string | null> {
    return candidatePath("save1.dat");
  }

  analyze(request: AnalyzeRequest): Promise<DesktopResult> {
    return new Promise((resolve) => {
      this.pending.push({ request, resolve });
    });
  }

  cancel(requestId: string): void {
    this.cancelledIds.push(requestId);
  }

  /** 受控源不提供导出/链接（SX-016 的端到端用例使用 MockDataSource 或 Tauri 桩）。 */
  async exportReport(): Promise<never> {
    throw new Error("ControlledSource 不支持导出");
  }

  async resolveGuideLink(): Promise<never> {
    throw new Error("ControlledSource 不支持链接解析");
  }

  async openGuideLink(): Promise<never> {
    throw new Error("ControlledSource 不支持链接打开");
  }

  async openExternalUrl(): Promise<never> {
    throw new Error("ControlledSource 不支持合成外链");
  }

  get pendingCount(): number {
    return this.pending.length;
  }

  lastRequest(): AnalyzeRequest {
    const request = this.pending[this.pending.length - 1]?.request;
    if (!request) {
      throw new Error("没有挂起的分析请求");
    }
    return request;
  }

  /** 按顺序解决挂起请求（可传入旧请求的晚到结果）。 */
  resolvePending(result: DesktopResult): void {
    const item = this.pending.shift();
    if (!item) {
      throw new Error("没有挂起的分析请求");
    }
    item.resolve({ ...result, request_id: item.request.requestId });
  }
}

const SEED = { compressedSha256: "ab".repeat(32), compressedBytes: 597_590 };

/** 当前快照信封（测试用）。 */
export function currentEnvelope(address: string, inputRef: string, snapshotId = "snap-new"): DesktopResult {
  return {
    schema_version: "1.0.0",
    request_id: address,
    input_ref: inputRef,
    snapshot_id: snapshotId,
    snapshot_time: "2026-10-16T00:00:00.000Z",
    freshness: "current",
    last_refresh_error: null,
    report: buildCompleteReport(SEED),
    error: null,
  };
}

/** 取消的无结果信封（测试用）。 */
export function cancelledEnvelope(requestId: string, inputRef: string): DesktopResult {
  return {
    schema_version: "1.0.0",
    request_id: requestId,
    input_ref: inputRef,
    snapshot_id: null,
    snapshot_time: null,
    freshness: "none",
    last_refresh_error: null,
    report: null,
    error: null,
  };
}
