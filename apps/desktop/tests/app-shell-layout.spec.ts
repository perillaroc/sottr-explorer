// 应用外壳与页面骨架（2026-11-05 界面美化阶段二）：
// 顶栏 56px 三段与四档降级、侧栏宽度 / 选中标记、浮层导航定位、
// 全局通知与 mock 横幅、AppBrandMark / AppPageHeader。
import { describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import App from "../src/App.vue";
import AppBrandMark from "../src/components/AppBrandMark.vue";
import AppPageHeader from "../src/components/AppPageHeader.vue";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

async function mountApp(width: number) {
  const { store } = await createTestStore({ width });
  await analyzeCandidate(store, "save1.dat");
  const wrapper = mountWithStore(App, store, { store, layoutWidth: width });
  await flushPromises();
  return { store, wrapper };
}

describe("AppBrandMark", () => {
  it("默认 24px 罗盘，可放大到关于页的 32px", () => {
    const wrapper = mount(AppBrandMark);
    expect(wrapper.find("svg").attributes("width")).toBe("24");
    expect(wrapper.find("svg").attributes("height")).toBe("24");
    expect(wrapper.classes()).toContain("text-accent");
    expect(wrapper.attributes("aria-hidden")).toBe("true");

    const large = mount(AppBrandMark, { props: { size: 32 } });
    expect(large.find("svg").attributes("width")).toBe("32");
  });
});

describe("AppPageHeader", () => {
  it("渲染标题、上下文、说明、页面动作与工具条插槽", () => {
    const wrapper = mount(AppPageHeader, {
      props: {
        title: "区域与收集",
        context: "帕依提提 · 快照 2026-11-08 12:43",
        description: "按区域查看收集品；计数来自当前快照。",
        titleId: "collection-title",
      },
      slots: {
        actions: '<button data-testid="page-action">查看未收集</button>',
        default: '<div data-testid="page-toolbar">工具条</div>',
      },
    });

    expect(wrapper.get('[data-testid="page-title"]').text()).toBe("区域与收集");
    expect(wrapper.get('[data-testid="page-title"]').attributes("id")).toBe("collection-title");
    expect(wrapper.get('[data-testid="page-title"]').classes()).toContain("ui-page-title");
    expect(wrapper.get('[data-testid="page-title"]').classes()).toContain("line-clamp-2");
    expect(wrapper.get('[data-testid="page-context"]').text()).toContain("帕依提提");
    expect(wrapper.get('[data-testid="page-context"]').attributes("title")).toContain("帕依提提");
    expect(wrapper.get('[data-testid="page-description"]').text()).toContain("计数来自当前快照");
    expect(wrapper.get('[data-testid="page-action"]').text()).toBe("查看未收集");
    expect(wrapper.get('[data-testid="page-toolbar"]').text()).toBe("工具条");
  });

  it("没有可选内容时不渲染空段落与工具栏", () => {
    const wrapper = mount(AppPageHeader, { props: { title: "设置" } });
    expect(wrapper.find('[data-testid="page-context"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="page-description"]').exists()).toBe(false);
    expect(wrapper.text()).toBe("设置");
  });
});

describe("顶栏三段与四档降级", () => {
  it("≥1440：56px 高、品牌副标题、文件名、刷新文字完整显示", async () => {
    const { wrapper } = await mountApp(1440);
    const banner = wrapper.get('[data-testid="app-topbar"]');
    expect(banner.classes()).toContain("h-14");
    expect(wrapper.find('[data-testid="brand-subtitle"]').exists()).toBe(true);
    expect(wrapper.get('[data-testid="current-file"]').text()).toBe("save1.dat");
    expect(wrapper.get('[data-testid="topbar-refresh"]').text()).toContain("刷新");
  });

  it("1100–1439：隐藏副标题，状态压缩为“分析完成”，刷新仍带文字", async () => {
    const { wrapper } = await mountApp(1280);
    expect(wrapper.find('[data-testid="brand-subtitle"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="analysis-status"]').text()).toContain("分析完成");
    expect(wrapper.get('[data-testid="topbar-refresh"]').text()).toContain("刷新");
  });

  it("800–1099：文件名可截断，状态进一步压缩为“完成”", async () => {
    const { wrapper } = await mountApp(1000);
    expect(wrapper.find('[data-testid="current-file"]').exists()).toBe(true);
    expect(wrapper.get('[data-testid="analysis-status"]').text()).toContain("完成");
    expect(wrapper.get('[data-testid="analysis-status"]').text()).not.toContain("分析完成");
    expect(wrapper.get('[data-testid="topbar-refresh"]').text()).toContain("刷新");
  });

  it("<800：刷新变为图标按钮，文件名与切换入口仍保留", async () => {
    const { wrapper } = await mountApp(799);
    const refresh = wrapper.get('[data-testid="topbar-refresh"]');
    expect(refresh.text()).toBe("");
    expect(refresh.attributes("aria-label")).toContain("刷新");
    expect(wrapper.get('[data-testid="current-file"]').text()).toBe("save1.dat");
    expect(wrapper.get('[data-testid="app-topbar"]').find('[aria-haspopup="menu"]').exists()).toBe(true);
  });

  it("状态完整时间保留在 title，简短文字不丢失状态语义", async () => {
    const { store, wrapper } = await mountApp(1280);
    const status = wrapper.get('[data-testid="analysis-status"]');
    expect(status.attributes("title")).toContain("快照");
    store.actions.setView("import");
    await flushPromises();
    expect(wrapper.get('[data-testid="analysis-status"]').text()).toContain("分析完成");
  });

  it("未分析时刷新按钮禁用并说明原因，概览等导航保持禁用", async () => {
    const { store } = await createTestStore({ width: 1280 });
    const wrapper = mountWithStore(App, store, { store, layoutWidth: 1280 });
    await flushPromises();

    const refresh = wrapper.get('[data-testid="topbar-refresh"]');
    expect((refresh.element as HTMLButtonElement).disabled).toBe(true);
    expect(refresh.attributes("title")).toBe("尚未分析，无法刷新");

    const overview = wrapper
      .get('[data-testid="app-sidebar"]')
      .findAll("button")
      .find((button) => button.text().includes("概览"));
    expect((overview?.element as HTMLButtonElement | undefined)?.disabled).toBe(true);
  });
});

describe("侧栏宽度与选中态", () => {
  it("宽屏展开 224px；一级导航 40px、18px 图标；选中保留 aria-current 与左侧标记", async () => {
    const { wrapper } = await mountApp(1440);
    const sidebar = wrapper.get('[data-testid="app-sidebar"]');
    expect(sidebar.classes()).toContain("w-[224px]");

    const active = sidebar.get('button[aria-current="page"]');
    expect(active.classes()).toContain("ui-row-selected");
    expect(active.classes()).toContain("h-10");
    expect(active.find("svg").attributes("width")).toBe("18");
  });

  it("中等宽度折叠为 64px 轨道，图标居中且标签仍可读", async () => {
    const { store, wrapper } = await mountApp(1280);
    const sidebar = wrapper.get('[data-testid="app-sidebar"]');
    expect(sidebar.classes()).toContain("w-16");
    const active = sidebar.get('button[aria-current="page"]');
    expect(active.classes()).toContain("justify-center");
    expect(active.text()).toContain("概览"); // sr-only 标签保留（分析完成后默认在概览）

    store.actions.setView("settings");
    await flushPromises();
    expect(wrapper.get('[data-testid="app-sidebar"] button[aria-current="page"]').text()).toContain("设置");
  });

  it("区域入口 36px，选中使用同一左侧标记", async () => {
    const { store, wrapper } = await mountApp(1440);
    store.actions.setView("regions");
    store.actions.setRegion("paititi");
    await flushPromises();
    const region = wrapper
      .get('[data-testid="app-sidebar"]')
      .findAll("button")
      .find((button) => button.text().includes("帕依提提"));
    expect(region).toBeTruthy();
    expect(region?.classes()).toContain("h-9");
    expect(region?.classes()).toContain("ui-row-selected");
  });
});

describe("浮层导航与全局提示", () => {
  it("浮层导航位于内容行内，不覆盖顶栏 / 模拟横幅", async () => {
    const { store, wrapper } = await mountApp(1280);
    store.actions.setNavOverlay(true);
    await flushPromises();
    const rail = wrapper.get('[data-testid="app-sidebar"]');
    const overlay = wrapper.get('[data-testid="nav-overlay"]');
    expect(overlay.element.parentElement).toBe(rail.element.parentElement);
    expect(overlay.classes().join(" ")).not.toContain("top-12");
    expect(overlay.classes().join(" ")).not.toContain("top-14");
    expect(wrapper.findAll('[data-testid="app-sidebar"]').length).toBe(2);
  });

  it("全局通知条有状态图标与状态底色，仍可关闭", async () => {
    const { store, wrapper } = await mountApp(1280);
    store.state.notice = { kind: "warning", text: "测试警告" };
    await flushPromises();
    const notice = wrapper.get('[data-testid="notice"]');
    expect(notice.find("svg").exists()).toBe(true);
    expect(notice.classes().join(" ")).toContain("bg-caution-bg");
    await notice.get("button").trigger("click");
    expect(wrapper.find('[data-testid="notice"]').exists()).toBe(false);
  });

  it("mock 横幅保持可见并使用更紧凑的字号", async () => {
    const { wrapper } = await mountApp(1280);
    const banner = wrapper.get('[data-testid="mock-banner"]');
    expect(banner.text()).toContain("模拟数据");
    expect(banner.classes()).toContain("text-[12px]");
  });

  it("窄窗 mock 横幅使用短句，避免双语长文占满首屏", async () => {
    const { wrapper } = await mountApp(640);
    const banner = wrapper.get('[data-testid="mock-banner"]');
    expect(banner.text()).toContain("模拟数据");
    expect(banner.text()).not.toContain("SX-016");
  });
});
