// 界面美化阶段五（2026-11-05）：地图、诊断、设置与关于。
// 地图的标记 / 工具条断言在 map-canvas.spec.ts，页头与清单行在 map-page.spec.ts。
import { describe, expect, it } from "vitest";
import SettingsPage from "../src/components/SettingsPage.vue";
import AboutPage from "../src/components/AboutPage.vue";
import DiagnosticsPage from "../src/components/DiagnosticsPage.vue";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

describe("设置页：三个分区与分段单选", () => {
  it("一个工作面分为显示 / 布局 / 外部链接；标签 128px、说明可见", async () => {
    const { store } = await createTestStore({ width: 1440 });
    const wrapper = mountWithStore(SettingsPage, store);
    const text = wrapper.text();
    for (const heading of ["显示", "布局", "外部链接"]) {
      expect(text).toContain(heading);
    }
    expect(wrapper.find('[data-testid="language-settings"] .w-32').exists()).toBe(true);
    expect(wrapper.get('[data-testid="language-hint"]').text()).toContain("中文");
    expect(wrapper.get('[data-testid="link-open-hint"]').text()).toContain("浏览器");
    expect(text).toContain("恢复导航展开状态、详情宽度与地图右栏宽度");
    // 原生 radio 仍保留，只是视觉改为分段选择。
    expect((wrapper.get('[data-testid="theme-system"]').element as HTMLInputElement).type).toBe("radio");
  });

  it("窄窗标签独占一行", async () => {
    const { store } = await createTestStore({ width: 640 });
    const wrapper = mountWithStore(SettingsPage, store);
    expect(wrapper.get('[data-testid="language-settings"]').classes()).toContain("flex-col");
  });
});

describe("关于页：品牌标记与长文", () => {
  it("页头显示 32px 罗盘、产品名与版本；许可折叠区使用共享 chevron", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(AboutPage, store);
    expect(wrapper.find("header svg").exists()).toBe(true);
    expect(wrapper.get("#about-title").text()).toBe("SOTTR Explorer");
    expect(wrapper.get("header").text()).toContain("只读存档分析");
    const details = wrapper.get('[data-testid="about-license-full"]');
    expect(details.classes()).toContain("ui-disclosure");
    expect(details.text()).toContain("依赖许可全文");
  });
});

describe("诊断页：统一工作面", () => {
  it("统一页头与 15px 区块标题；能力矩阵在带边界的工作面内", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(DiagnosticsPage, store);
    expect(wrapper.get('[data-testid="page-title"]').text()).toContain("诊断");
    for (const id of [
      "diagnostics-input",
      "diagnostics-capabilities",
      "diagnostics-counters",
      "diagnostics-locations",
      "diagnostics-entries",
      "diagnostics-observed",
    ]) {
      expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(true);
    }
    expect(wrapper.find('[data-testid="diagnostics-capabilities"] .ui-panel table').exists()).toBe(true);
    expect(wrapper.findAll(".ui-section-title").length).toBeGreaterThanOrEqual(6);
  });
});
