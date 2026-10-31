// 详情宽度拖动（2026-10-31 地图查询与详情增强·阶段一）：
// 手柄拖动/键盘/双击复位、宽度钳制、常驻/普通抽屉/紧凑抽屉三种宽度、偏好往返。
import { describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import App from "../src/App.vue";
import UiResizeHandle from "../src/components/ui/UiResizeHandle.vue";
import { createAppStore } from "../src/app/store";
import { clearPreferences } from "../src/app/prefs";
import { MockDataSource } from "../src/data/mock";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";

// 一个带官方中文名的故事条目（圣胡安文献）。
const ROW_ID = "0xCAA74459";

function mountHandle(overrides: Record<string, unknown> = {}) {
  return mount(UiResizeHandle, {
    props: {
      width: 400,
      min: 320,
      max: () => 720,
      defaultWidth: 400,
      label: "调整详情宽度",
      ...overrides,
    },
    attachTo: document.body,
  });
}

function handleElement(wrapper: ReturnType<typeof mountHandle>): HTMLElement {
  return wrapper.get('[role="separator"]').element as HTMLElement;
}

function pointer(element: HTMLElement, type: string, clientX: number): void {
  element.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX, button: 0 }));
}

async function mountApp(width: number) {
  const { store } = await createTestStore({ width });
  await analyzeCandidate(store, "save1.dat");
  // 区域页需要选定区域才显示逐项清单；详情用例需要条目可见。
  store.actions.setRegion("mission_of_san_juan");
  store.actions.setStatusFilter("all");
  store.actions.setView("regions");
  const wrapper = mountWithStore(App, store, { store, layoutWidth: width });
  await flushPromises();
  return { store, wrapper };
}

describe("UiResizeHandle", () => {
  it("暴露 separator 语义与当前宽度", () => {
    const wrapper = mountHandle({ width: 480 });
    const handle = wrapper.get('[role="separator"]');
    expect(handle.attributes("aria-orientation")).toBe("vertical");
    expect(handle.attributes("aria-valuemin")).toBe("320");
    expect(handle.attributes("aria-valuemax")).toBe("720");
    expect(handle.attributes("aria-valuenow")).toBe("480");
    expect(handle.attributes("tabindex")).toBe("0");
  });

  it("键盘 ←/→ 每次 16px，Shift 加速 64px，Home/End 到边界", async () => {
    const wrapper = mountHandle({ width: 400 });
    const handle = wrapper.get('[role="separator"]');

    await handle.trigger("keydown", { key: "ArrowLeft" });
    expect(wrapper.emitted("resize")?.at(-1)).toEqual([416]);
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([416]);

    await handle.trigger("keydown", { key: "ArrowRight", shiftKey: true });
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([336]);

    await handle.trigger("keydown", { key: "Home" });
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([320]);

    await handle.trigger("keydown", { key: "End" });
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([720]);
  });

  it("双击回到默认宽度", async () => {
    const wrapper = mountHandle({ width: 610, defaultWidth: 400 });
    await wrapper.get('[role="separator"]').trigger("dblclick");
    expect(wrapper.emitted("resize")?.at(-1)).toEqual([400]);
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([400]);
  });

  it("指针拖动实时发 resize，松手才发 commit；拖动中禁用文本选择", async () => {
    const wrapper = mountHandle({ width: 400 });
    const element = handleElement(wrapper);

    pointer(element, "pointerdown", 800);
    expect(document.body.style.userSelect).toBe("none");

    pointer(element, "pointermove", 750);
    expect(wrapper.emitted("resize")?.at(-1)).toEqual([450]);
    expect(wrapper.emitted("commit")).toBeUndefined();

    pointer(element, "pointermove", 700);
    expect(wrapper.emitted("resize")?.at(-1)).toEqual([500]);

    pointer(element, "pointerup", 700);
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([500]);
    expect(document.body.style.userSelect).toBe("");
  });

  it("拖动不会超过最大宽度", async () => {
    const wrapper = mountHandle({ width: 700 });
    const element = handleElement(wrapper);
    pointer(element, "pointerdown", 800);
    pointer(element, "pointermove", 700);
    expect(wrapper.emitted("resize")?.at(-1)).toEqual([720]);
    pointer(element, "pointerup", 700);
    expect(wrapper.emitted("commit")?.at(-1)).toEqual([720]);
  });
});

describe("详情宽度（App 集成）", () => {
  it("宽屏常驻详情：拖动实时预览、松手写入偏好", async () => {
    const { store, wrapper } = await mountApp(1440);
    store.actions.openRow(ROW_ID);
    await flushPromises();

    const pane = wrapper.get('[data-testid="detail-pane"]');
    expect((pane.element as HTMLElement).style.width).toBe("400px");

    const element = wrapper.get('[data-testid="detail-pane-resize"]').element as HTMLElement;
    pointer(element, "pointerdown", 900);
    pointer(element, "pointermove", 830);
    await flushPromises();
    // 拖动中只做实时预览：偏好尚未写入。
    expect((pane.element as HTMLElement).style.width).toBe("470px");
    expect(store.state.detailPaneWidth).toBe(400);

    pointer(element, "pointerup", 830);
    await flushPromises();
    expect(store.state.detailPaneWidth).toBe(470);
    expect((pane.element as HTMLElement).style.width).toBe("470px");
  });

  it("宽屏常驻详情：键盘调整与双击复位", async () => {
    const { store, wrapper } = await mountApp(1440);
    store.actions.openRow(ROW_ID);
    await flushPromises();
    const pane = wrapper.get('[data-testid="detail-pane"]');
    const handle = wrapper.get('[data-testid="detail-pane-resize"]');

    await handle.trigger("keydown", { key: "ArrowLeft" });
    await flushPromises();
    expect(store.state.detailPaneWidth).toBe(416);
    expect((pane.element as HTMLElement).style.width).toBe("416px");

    await handle.trigger("keydown", { key: "End" });
    await flushPromises();
    expect(store.state.detailPaneWidth).toBe(720);

    await handle.trigger("dblclick");
    await flushPromises();
    expect(store.state.detailPaneWidth).toBe(400);
  });

  it("1100–1439：抽屉默认 400px，拖动写入 detailDrawerWidth", async () => {
    const { store } = await mountApp(1280);
    store.actions.openRow(ROW_ID);
    await flushPromises();
    const drawer = document.querySelector('[role="dialog"]') as HTMLElement;
    expect(drawer.style.width).toBe("400px");

    const handle = document.querySelector('[data-testid="detail-drawer-resize"]') as HTMLElement;
    pointer(handle, "pointerdown", 1000);
    pointer(handle, "pointermove", 900);
    await flushPromises();
    expect(drawer.style.width).toBe("500px");

    pointer(handle, "pointerup", 900);
    await flushPromises();
    expect(store.state.detailDrawerWidth).toBe(500);
    expect(store.state.detailCompactWidth).toBe(720);
  });

  it("800–1099：紧凑抽屉保留 720px 默认值，调整写入 detailCompactWidth", async () => {
    const { store } = await mountApp(1000);
    store.actions.openRow(ROW_ID);
    await flushPromises();
    const drawer = document.querySelector('[role="dialog"]') as HTMLElement;
    expect(drawer.style.width).toBe("720px");

    const handle = document.querySelector('[data-testid="detail-drawer-resize"]') as HTMLElement;
    handle.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
    await flushPromises();
    expect(store.state.detailCompactWidth).toBe(704);
    expect(store.state.detailDrawerWidth).toBe(400);
  });

  it("窄窗（<800）不提供宽度手柄：详情为整页替换", async () => {
    const { store, wrapper } = await mountApp(799);
    store.actions.openRow(ROW_ID);
    await flushPromises();
    expect(wrapper.find('[data-testid="detail-pane"]').exists()).toBe(false);
    expect(document.querySelector('[data-testid="detail-drawer-resize"]')).toBeNull();
    expect(wrapper.find('[data-testid="detail-back"]').exists()).toBe(true);
  });
});

describe("宽度偏好（store）", () => {
  it("宽度写入 localStorage 后重建 store 仍保留；resetLayout 一并复位", () => {
    clearPreferences();
    const source = new MockDataSource({ delayMs: 0 });
    const store = createAppStore(source, { persist: true, layoutWidth: 1440 });
    store.actions.setDetailPaneWidth(480);
    store.actions.setDetailDrawerWidth(520);
    store.actions.setDetailCompactWidth(760);
    store.actions.setMapRailWidth(500);

    const reloaded = createAppStore(source, { persist: true, layoutWidth: 1440 });
    expect(reloaded.state.detailPaneWidth).toBe(480);
    expect(reloaded.state.detailDrawerWidth).toBe(520);
    expect(reloaded.state.detailCompactWidth).toBe(760);
    expect(reloaded.state.mapRailWidth).toBe(500);

    reloaded.actions.resetLayout();
    expect(reloaded.state.detailPaneWidth).toBe(400);
    expect(reloaded.state.detailDrawerWidth).toBe(400);
    expect(reloaded.state.detailCompactWidth).toBe(720);
    expect(reloaded.state.mapRailWidth).toBe(420);
    clearPreferences();
  });

  it("宽度 setter 钳制到安全范围", () => {
    const store = createAppStore(new MockDataSource({ delayMs: 0 }), { persist: false, layoutWidth: 1280 });
    store.actions.setDetailPaneWidth(100);
    expect(store.state.detailPaneWidth).toBe(320);
    store.actions.setDetailPaneWidth(2000);
    expect(store.state.detailPaneWidth).toBe(720);
    store.actions.setMapRailWidth(1000);
    expect(store.state.mapRailWidth).toBe(600);
  });
});
