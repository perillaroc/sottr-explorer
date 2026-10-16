// 真实数据绑定测试（SX-016）：把 Rust 端到端产出的 `DesktopResult`（来自真实存档，
// 见 `cargo test -p sottr-desktop --test ipc_commands real_fixture_end_to_end_*`）
// 喂给真实界面组件，验证界面消费的是 Rust 核心的真实输出，而不是模拟夹具。
//
// 未设置 `SOTTR_DESKTOP_E2E_RESULT`（桌面端到端结果 JSON 路径）时跳过，
// 并在日志中说明跳过原因（不记为通过）。
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import OverviewPage from "../src/components/OverviewPage.vue";
import RegionPage from "../src/components/RegionPage.vue";
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
      game_roots: [`C:\\真实夹具\\${this.result.input_ref}`],
    };
  }

  override async listCandidates(root: string): Promise<CandidatesOutcomeDto> {
    return { root, files: [], skipped: [], depth_limited_dirs: [] };
  }
}

describe.skipIf(!resultPath)("真实 Rust 输出 → 界面绑定（SX-016）", () => {
  it("真实报告在概览/区域/详情中按契约呈现（66.20% 与限定计数分离）", async () => {
    const result = JSON.parse(readFileSync(resultPath, "utf8")) as DesktopResult;
    expect(result.report, "真实结果必须包含报告").toBeTruthy();
    const report = result.report!;

    const source = new RealResultSource(result);
    const store = createAppStore(source, {
      persist: false,
      layoutWidth: 1280,
      metadata: productionMetadata,
    });
    const path = `C:\\真实夹具\\${result.input_ref}`;
    store.actions.selectCandidate(path);
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();

    // 概览：整体候选与支持范围内遗漏分别标示；计数缺失不补零。
    const overview = mountWithStore(OverviewPage, store);
    await flushPromises();
    const overall = report.observations.overall_completion.value;
    if (overall !== null) {
      expect(overview.get('[data-testid="overall-value"]').text()).toContain(`${overall.toFixed(2)}%`);
    } else {
      expect(overview.get('[data-testid="overall-value"]').text()).toContain("不可用");
    }
    expect(overview.get('[data-testid="missing-value"]').text()).toContain(`${store.missingTotal.value} 项`);
    expect(overview.get('[data-testid="overall-summary"]').text()).toContain("推定");
    expect(overview.get('[data-testid="coverage-banner"]').text()).toContain("参考遗漏仅覆盖");

    // 区域页：真实元数据分析出的类别计数来自真实报告（先选到圣胡安区域）。
    store.actions.setView("regions");
    store.actions.setRegion("mission_of_san_juan");
    const region = mountWithStore(RegionPage, store);
    await flushPromises();
    expect(region.find('[data-testid="coverage-banner"]').exists()).toBe(true);
    const sanJuan = region.find('[data-cell^="mission_of_san_juan"]');
    expect(sanJuan.exists()).toBe(true);
    const realCaches = report.observations.region_counters.find(
      (counter) => counter.counter_key === "counter:mission_of_san_juan:survival_caches",
    );
    if (realCaches?.value !== null && realCaches?.value !== undefined) {
      expect(
        region.get('[data-cell="mission_of_san_juan:survival_caches:value"]').text(),
      ).toContain(`${realCaches.value}/${realCaches.source_target}`);
    }

    // 名字回退：真实元数据没有 display_name → 可读标签 + 名称待确认。
    store.actions.setStatusFilter("all");
    await flushPromises();
    expect(region.text()).toContain("名称待确认");
    // 原始 present 不显示为“已收集”。
    expect(region.text()).not.toContain("已验证已收集");
  });
});
