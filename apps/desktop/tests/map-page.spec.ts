// 地图查询页（2026-10-31 地图查询与详情增强·阶段二）：
// 区域/子关卡选择、标记与清单联动、标签页行为、筛选、空图状态、响应式与区域解耦。
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";

vi.mock("../src/data/basemaps", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/data/basemaps")>();
  return { ...actual, basemapImageUrl: (entry: Basemap) => `/basemaps/${entry.image}` };
});

import MapPage from "../src/components/MapPage.vue";
import { createAppStore } from "../src/app/store";
import { MockDataSource } from "../src/data/mock";
import type { Basemap } from "../src/data/basemaps";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

beforeAll(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(_callback: ResizeObserverCallback) {}
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    },
  );
});

afterAll(() => {
  vi.unstubAllGlobals();
});

beforeEach(() => {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    width: 800,
    height: 600,
    top: 0,
    left: 0,
    right: 800,
    bottom: 600,
    toJSON: () => ({}),
  });
});

async function mountMap(width = 1440) {
  const { store } = await createTestStore({ width });
  await analyzeCandidate(store, "save1.dat");
  store.actions.setView("map");
  const wrapper = mountWithStore(MapPage, store);
  await flushPromises();
  return { store, wrapper };
}

describe("地图页：目录与默认选择", () => {
  it("区域选择只列有底图的区域；默认打开第一个有带坐标条目的区域", async () => {
    const { store, wrapper } = await mountMap(1440);
    const regionSelect = wrapper.get('[data-testid="map-region-select"]');
    expect(regionSelect.findAll("option").length).toBe(9);
    expect(store.state.mapRegion).toBe("paititi");
    expect(store.state.mapBasemapId).not.toBeNull();
    expect(wrapper.get('[data-testid="map-basemap-select"]').findAll("option").length).toBeGreaterThan(0);

    const markers = wrapper.findAll("[data-map-marker='true']");
    expect(markers.length).toBeGreaterThan(0);
    expect(wrapper.get('[data-testid="map-counts"]').text()).toContain("已收集");
    expect(wrapper.get('[data-testid="map-coverage"]').text()).toContain("42");
    expect(wrapper.get('[data-testid="map-coverage"]').text()).toContain("24/65");
    expect(wrapper.get('[data-testid="map-coverage"]').text()).toContain("291/470");
  });

  it("切换区域/子关卡只更新地图选择，不回写区域清单的区域", async () => {
    const { store } = await createTestStore({ width: 1440 });
    await analyzeCandidate(store, "save1.dat");
    store.actions.setRegion("mission_of_san_juan");
    store.actions.setView("map");
    const wrapper = mountWithStore(MapPage, store);
    await flushPromises();

    expect(store.state.mapRegion).toBe("mission_of_san_juan");
    store.actions.setMapRegion("paititi");
    await flushPromises();
    expect(store.state.mapRegion).toBe("paititi");
    expect(store.state.region).toBe("mission_of_san_juan");
    wrapper.unmount();
  });
});

describe("地图页：标记与清单联动", () => {
  it("点击标记：选中并自动切到「详情」，隐藏相对位置段落", async () => {
    const { store, wrapper } = await mountMap(1440);
    const marker = wrapper.findAll("[data-map-marker='true']")[0];
    const id = (marker.attributes("data-testid") ?? "").replace("map-marker-", "");
    await marker.trigger("click");
    await flushPromises();

    expect(store.state.selectedRowId).toBe(id);
    expect(store.state.mapRailTab).toBe("detail");
    expect(wrapper.get('[data-testid="map-detail-panel"]').text()).toContain(
      store.selectedEntry.value?.item.display_name ?? "",
    );
    expect(wrapper.find('[data-testid="position-section"]').exists()).toBe(false);
  });

  it("点击清单项：选中、高亮并保持在「清单」标签", async () => {
    const { store, wrapper } = await mountMap(1440);
    const row = wrapper.get("[data-item-id]");
    const id = row.attributes("data-item-id")!;
    await row.trigger("click");
    await flushPromises();

    expect(store.state.selectedRowId).toBe(id);
    expect(store.state.mapRailTab).toBe("list");
    expect(wrapper.get('[data-testid="map-tab-list"]').attributes("aria-selected")).toBe("true");
    expect(row.classes().join(" ")).toContain("bg-accent-bg");
  });

  it("标签页支持 ←/→ 切换；切换后各自保留滚动位置", async () => {
    const { wrapper } = await mountMap(1440);
    const listPanel = wrapper.get('[data-testid="map-list-panel"]').element as HTMLElement;
    listPanel.scrollTop = 128;

    await wrapper.get('[data-testid="map-tab-detail"]').trigger("click");
    expect(wrapper.get('[data-testid="map-tab-detail"]').attributes("aria-selected")).toBe("true");

    // ←/→ 在标签之间切换（当前焦点标签上按方向键）。
    await wrapper.get('[data-testid="map-tab-detail"]').trigger("keydown", { key: "ArrowLeft" });
    await flushPromises();
    expect(wrapper.get('[data-testid="map-tab-list"]').attributes("aria-selected")).toBe("true");

    await wrapper.get('[data-testid="map-tab-list"]').trigger("keydown", { key: "ArrowRight" });
    await flushPromises();
    expect(wrapper.get('[data-testid="map-tab-detail"]').attributes("aria-selected")).toBe("true");

    await wrapper.get('[data-testid="map-tab-list"]').trigger("click");
    await flushPromises();
    // v-show 保留 DOM，滚动位置不被重置。
    expect(listPanel.scrollTop).toBe(128);
  });

  it("改变状态筛选清空选中；详情空状态给出引导", async () => {
    const { store, wrapper } = await mountMap(1440);
    await wrapper.findAll("[data-map-marker='true']")[0].trigger("click");
    await flushPromises();
    expect(store.state.selectedRowId).not.toBeNull();

    const statusGroup = wrapper.get('[aria-label="按收集状态筛选（地图）"]');
    await statusGroup.findAll("button")[1].trigger("click");
    await flushPromises();
    expect(store.state.mapStatus).toBe("missing");
    expect(store.state.selectedRowId).toBeNull();
    expect(wrapper.get('[data-testid="map-detail-empty"]').text()).toContain("点击地图上的图标或清单中的条目");
  });
});

describe("地图页：空状态", () => {
  it("无坐标条目的子关卡：地图与清单都说明无条目", async () => {
    const { store, wrapper } = await mountMap(1440);
    store.actions.setMapRegion("cozumel");
    store.actions.setMapBasemap("bm:cozumel/54");
    await flushPromises();

    expect(wrapper.find('[data-testid="map-empty-items"]').exists()).toBe(true);
    expect(wrapper.findAll("[data-map-marker='true']").length).toBe(0);

    await wrapper.get('[data-testid="map-tab-list"]').trigger("click");
    await flushPromises();
    expect(wrapper.get('[data-testid="map-list-empty"]').text()).toContain("没有带坐标的收集品");
    expect(wrapper.get('[data-testid="map-list-unplaced"]').text()).toContain("0 项无坐标");
  });
});

describe("地图页：响应式", () => {
  it("800–1099：右栏为右侧浮层，点击标记自动打开并切到详情", async () => {
    const { wrapper } = await mountMap(1000);
    expect(wrapper.find('[data-testid="map-rail-inline"]').exists()).toBe(false);
    expect(document.querySelector('[role="dialog"]')).toBeNull();

    await wrapper.findAll("[data-map-marker='true']")[0].trigger("click");
    await flushPromises();
    const dialog = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    expect(dialog?.querySelector('[data-testid="map-rail"]')).not.toBeNull();
    expect(dialog?.querySelector('[data-testid="map-tab-detail"]')?.getAttribute("aria-selected")).toBe("true");
  });

  it("<800：地图 + 折叠清单；标记打开全宽详情浮层且地图不被卸载", async () => {
    const { wrapper } = await mountMap(799);
    expect(wrapper.find('[data-testid="map-rail-inline"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="map-narrow-list"]').exists()).toBe(true);

    // 清单行只选中，不打开浮层。
    const row = wrapper.get("[data-item-id]");
    await row.trigger("click");
    await flushPromises();
    expect(document.querySelector('[role="dialog"]')).toBeNull();

    // 行尾「详情」按钮打开全宽浮层。
    await wrapper.get('[data-testid^="map-row-detail-"]').trigger("click");
    await flushPromises();
    const dialog = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    expect(dialog?.querySelector('[data-testid="item-detail"]')).not.toBeNull();

    // 关闭浮层后地图仍在（缩放/选中状态不被页面替换重置）。
    const close = dialog?.querySelector('button[aria-label="关闭详情（Esc）"]') as HTMLElement | null;
    close?.click();
    await flushPromises();
    expect(wrapper.find('[data-testid="map-canvas"]').exists()).toBe(true);
  });
});

describe("地图选择校验（store）", () => {
  it("setMapBasemap 拒绝不属于当前区域的底图", async () => {
    const source = new MockDataSource({ delayMs: 0 });
    const store = createAppStore(source, { persist: false, layoutWidth: 1440 });
    await store.actions.boot();
    await analyzeCandidate(store, "save1.dat");
    store.actions.setView("map");
    expect(store.state.mapRegion).toBe("paititi");

    const before = store.state.mapBasemapId;
    store.actions.setMapBasemap("bm:cozumel/54");
    expect(store.state.mapBasemapId).toBe(before);
  });
});
