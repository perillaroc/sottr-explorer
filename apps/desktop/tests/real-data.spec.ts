// 真实数据绑定测试（SX-016 / SX-022）：把 Rust 端到端产出的 `DesktopResult`（来自真实存档）
// 喂给真实界面组件，验证界面消费的是 Rust 核心的真实输出，而不是模拟夹具。
//
// 未设置 `SOTTR_DESKTOP_E2E_RESULT`（桌面端到端结果 JSON 路径）时跳过，
// 并在日志中说明跳过原因（不记为通过）。
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import OverviewPage from "../src/components/OverviewPage.vue";
import CollectionPage from "../src/components/CollectionPage.vue";
import { createAppStore } from "../src/app/store";
import { productionMetadata } from "../src/data/metadata";
import { MockDataSource } from "../src/data/mock";
import { mountWithStore } from "./helpers";
import type { DesktopResult } from "../src/contract/desktop-result";
import type { AnalyzeRequest, CandidatesOutcomeDto, DiscoverOutcomeDto } from "../src/data/source";

const resultPath = process.env.SOTTR_DESKTOP_E2E_RESULT ?? "";
if (!resultPath) {
  // 跳过原因：真实存档属于私人夹具，只能在授权环境用环境变量显式提供。
  console.info("[real-data] 未设置 SOTTR_DESKTOP_E2E_RESULT，跳过真实输出绑定测试");
}

/** 只服务真实结果的桩数据源：分析返回 Rust 产出的信封（其他命令不参与本测试）。 */
class RealResultSource extends MockDataSource {
  constructor(private readonly result: DesktopResult) {
    super({ delayMs: 0 });
  }

  override async analyze(request: AnalyzeRequest): Promise<DesktopResult> {
    return { ...this.result, request_id: request.requestId };
  }

  override async discover(): Promise<DiscoverOutcomeDto> {
    return {
      candidates: [],
      game_roots: [`C:\真实夹具\${this.result.input_ref}`],
    };
  }

  override async listCandidates(root: string): Promise<CandidatesOutcomeDto> {
    return { root, files: [], skipped: [], depth_limited_dirs: [] };
  }
}

describe.skipIf(!resultPath)("真实 Rust 输出 → 界面绑定（SX-016 / SX-022）", () => {
  it("真实报告在概览与区域页按契约呈现（游戏内完成度与工具计数分离）", async () => {
    const result = JSON.parse(readFileSync(resultPath, "utf8")) as DesktopResult;
    expect(result.report, "真实结果必须包含报告").toBeTruthy();
    const report = result.report!;

    const source = new RealResultSource(result);
    const store = createAppStore(source, {
      persist: false,
      layoutWidth: 1280,
      metadata: productionMetadata,
      // 真实路径：收集状态由当前报告的物品表决定，不使用演示观察集合。
      observedIds: () => null,
    });
    const path = `C:\真实夹具\${result.input_ref}`;
    store.actions.selectCandidate(path);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();

    // 概览：游戏内完成度字段与工具的收集品计数分开标示。
    const overview = mountWithStore(OverviewPage, store);
    await flushPromises();
    const overall = report.observations.overall_completion.value;
    if (overall !== null) {
      expect(overview.get('[data-testid="overall-value"]').text()).toContain(`${overall.toFixed(2)}%`);
    } else {
      expect(overview.get('[data-testid="overall-value"]').text()).toContain("不可用");
    }
    expect(overview.get('[data-testid="coverage-banner"]').text()).toContain("收集状态口径");
    expect(overview.get('[data-testid="uncollected-value"]').text()).toContain(
      `${store.planCounts.value.uncollected} 项`,
    );

    // 区域页：区域清单来自图鉴，计数来自真实报告的物品表。
    store.actions.setView("regions");
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setStatusFilter("all");
    const region = mountWithStore(CollectionPage, store);
    await flushPromises();
    expect(region.find('[data-testid="coverage-banner"]').exists()).toBe(true);
    const sanJuan = store.planRegion.value;
    expect(sanJuan?.slug).toBe("mission_of_san_juan");
    expect(region.text()).toContain("圣胡安教会");

    // 区域计数与存档自带的区域计数器一致（逐类别核对）。
    const realCaches = report.observations.region_counters.find(
      (counter) => counter.counter_key === "counter:mission_of_san_juan:survival_caches",
    );
    if (realCaches?.value !== null && realCaches?.value !== undefined) {
      const group = sanJuan?.groups.find((entry) => entry.category === "survival_caches");
      expect(group?.collected).toBe(realCaches.value);
      expect(group?.total).toBe(realCaches.source_target);
    }

    // 名称：故事条目使用游戏官方中文名（来自 locals 对齐），不是“名称待确认”。
    const story = sanJuan?.groups
      .filter((entry) => entry.category === "artifacts")
      .flatMap((entry) => entry.items)
      .find((item) => item.name_zh);
    expect(story?.name_zh).toBeTruthy();
    expect(region.text()).toContain(story!.name_zh!);
  });
});
