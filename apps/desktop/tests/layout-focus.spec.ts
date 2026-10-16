// 布局档位与焦点行为测试（SX-015 验收：800/1100/1280/1440px 与更窄降级、
// 模态焦点限制/关闭恢复、常驻详情无焦点陷阱、窄窗页面替换与滚动保留、快捷键）。
import { describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import App from "../src/App.vue";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";

async function mountApp(width: number) {
  const { store } = await createTestStore({ width });
  await analyzeCandidate(store, "save1.dat");
  store.actions.setView("regions");
  const wrapper = mountWithStore(App, store, { store, layoutWidth: width });
  await flushPromises();
  return { store, wrapper };
}

function press(key: string, options: KeyboardEventInit = {}): void {
  document.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...options }));
}

describe("布局档位（有效内容宽度）", () => {
  it.each([
    [1440, "wide"],
    [1280, "medium"],
    [1100, "medium"],
    [1000, "compact"],
    [800, "compact"],
    [799, "narrow"],
    [640, "narrow"],
  ])("%ipx → %s", async (width, expected) => {
    const { wrapper } = await mountApp(width);
    expect(wrapper.get('[data-testid="app-root"]').attributes("data-layout")).toBe(expected);
  });

  it("≥1440：详情常驻（非模态，无焦点陷阱）", async () => {
    const { store, wrapper } = await mountApp(1440);
    store.actions.openRow("loc:stella:san-juan-caches");
    await flushPromises();
    const pane = wrapper.get('[data-testid="detail-pane"]');
    expect(pane.text()).toContain("圣胡安贮藏点组");
    // 常驻详情不是对话框：没有 aria-modal，Tab 不被拦截。
    expect(pane.find('[role="dialog"]').exists()).toBe(false);
    const outside = wrapper.get("header button");
    (outside.element as HTMLElement).focus();
    press("Tab");
    expect(document.activeElement).toBe(outside.element);
  });

  it("1100–1439：右侧模态抽屉；Esc 关闭并恢复焦点到触发条目", async () => {
    const { store } = await mountApp(1280);
    const row = document.querySelector('[data-testid="row-loc:stella:san-juan-caches"]') as HTMLElement | null;
    store.actions.openRow("loc:stella:san-juan-caches");
    await flushPromises();
    const drawer = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(drawer).not.toBeNull();
    expect(drawer?.textContent).toContain("圣胡安贮藏点组");
    // 打开后焦点进入抽屉。
    expect(drawer?.contains(document.activeElement)).toBe(true);

    // 焦点限制：在最后一个可聚焦元素上按 Tab 回到第一个。
    const focusables = Array.from(
      drawer?.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled]), summary") ?? [],
    );
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    last.focus();
    press("Tab");
    expect(document.activeElement).toBe(first);
    first.focus();
    press("Tab", { shiftKey: true });
    expect(document.activeElement).toBe(last);
    expect(row).not.toBeNull();
  });

  it("模态抽屉关闭后焦点回到原触发条目", async () => {
    const { wrapper, store } = await mountApp(1280);
    const row = wrapper.get('[data-testid="row-loc:stella:san-juan-caches"]');
    (row.element as HTMLElement).focus();
    await row.trigger("click");
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
    press("Escape");
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.activeElement).toBe(row.element);
    expect(store.state.selectedRowId).toBeNull();
  });

  it("800–1099：近全宽抽屉（不挤成三列）", async () => {
    const { store } = await mountApp(1000);
    store.actions.openRow("loc:stella:san-juan-caches");
    await flushPromises();
    const drawer = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(drawer?.className ?? "").toContain("min(720px");
  });

  it("<800：单列，详情页面替换清单，进入聚焦标题、返回恢复滚动位置", async () => {
    const { store, wrapper } = await mountApp(799);
    expect(wrapper.find('[data-testid="app-sidebar"]').exists()).toBe(false);
    const list = wrapper.get('[data-testid="region-list"]');
    (list.element as HTMLElement).scrollTop = 120;

    const row = wrapper.get('[data-testid="row-loc:stella:san-juan-caches"]');
    await row.trigger("click");
    await flushPromises();
    // 无模态抽屉：是页面替换。
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    const heading = wrapper.get('[data-testid="detail-heading"]');
    expect(document.activeElement).toBe(heading.element);
    expect(store.state.listScrollTop).toBe(120);

    // 返回清单：滚动位置恢复，回到条目所在列表。
    await wrapper.get('[data-testid="detail-back"]').trigger("click");
    await flushPromises();
    const restored = wrapper.get('[data-testid="region-list"]');
    expect((restored.element as HTMLElement).scrollTop).toBe(120);
    expect(wrapper.find('[data-testid="row-loc:stella:san-juan-caches"]').exists()).toBe(true);
  });

  it("<800：Esc 从详情返回清单", async () => {
    const { store, wrapper } = await mountApp(799);
    store.actions.openRow("loc:stella:san-juan-caches");
    await flushPromises();
    expect(wrapper.find('[data-testid="detail-back"]').exists()).toBe(true);
    press("Escape");
    await flushPromises();
    expect(wrapper.find('[data-testid="detail-back"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="region-list"]').exists()).toBe(true);
  });

  it("导航在中等宽度以浮层展开（不挤窄清单）", async () => {
    const { store, wrapper } = await mountApp(1280);
    const rail = wrapper.get('[data-testid="app-sidebar"]');
    expect(rail.classes().join(" ")).toContain("w-14");
    store.actions.toggleNav();
    await flushPromises();
    const sidebars = wrapper.findAll('[data-testid="app-sidebar"]');
    expect(sidebars.length).toBe(2); // 轨道 + 浮层
    expect(sidebars[1].classes().join(" ")).toContain("w-[216px]");
    store.actions.setNavOverlay(false);
    await flushPromises();
    expect(wrapper.findAll('[data-testid="app-sidebar"]').length).toBe(1);
  });
});

describe("键盘快捷键（可发现、不与系统冲突）", () => {
  it("Ctrl+F 聚焦搜索、Ctrl+O 打开选择器、F5 刷新", async () => {
    const { store, wrapper } = await mountApp(1280);
    const focusSpy = vi.spyOn(store.actions, "requestSearchFocus");
    press("f", { ctrlKey: true });
    expect(focusSpy).toHaveBeenCalled();
    await flushPromises();
    expect(document.activeElement?.tagName).toBe("INPUT");

    const refreshSpy = vi.spyOn(store.actions, "refresh");
    press("F5");
    expect(refreshSpy).toHaveBeenCalled();

    const pickSpy = vi.spyOn(store.actions, "pickInput");
    press("o", { ctrlKey: true });
    expect(pickSpy).toHaveBeenCalledWith("save-file");
    await flushPromises();
    expect(store.state.view).toBe("import");
    expect(wrapper.text()).toContain("打开存档");
  });
});
