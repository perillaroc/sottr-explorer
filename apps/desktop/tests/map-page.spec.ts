// 地图查询页（2026-10-31 地图查询与详情增强·阶段二）：
// 区域/子关卡选择、标记与清单联动、标签页行为、筛选、空图状态、响应式与区域解耦。
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";

vi.mock("../src/data/basemaps", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/data/basemaps")>();
  return { ...actual, basemapImageUrl: (entry: Basemap) => `/basemaps/${entry.image}` };
});

// 遮罩采样结果由用例控制：默认 null（无法检测，保持原有渲染）。
const maskState = vi.hoisted(() => ({ mask: null as MapMask | null }));
vi.mock("../src/app/map-mask", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/app/map-mask")>();
  return { ...actual, readMapMask: () => maskState.mask };
});

import MapPage from "../src/components/MapPage.vue";
import { createAppStore } from "../src/app/store";
import { MockDataSource } from "../src/data/mock";
import type { MapMask } from "../src/app/map-mask";
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
  maskState.mask = null;
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
    expect(wrapper.get('[data-testid="map-calibration"]').text()).toMatch(/已校准|近似位置/);
  });

  it("页头以区域 · 子关卡为主标题，计数、校准与底图署名同在第一行", async () => {
    const { wrapper } = await mountMap(1440);
    expect(wrapper.get('[data-testid="page-title"]').text()).toContain("帕依提提");
    expect(wrapper.get('[data-testid="page-context"]').text()).toContain("地图查询");
    expect(wrapper.get('[data-testid="page-header"]').text()).toContain("底图 · vi7ual");
    expect(wrapper.get('[data-testid="map-counts"]').text()).toContain("显示");
    // 状态筛选与区域清单一致：未收集 / 已收集 / 全部。
    const statusButtons = wrapper
      .get('[aria-label="按收集状态筛选（地图）"]')
      .findAll("button")
      .map((button) => button.text());
    expect(statusButtons[0]).toContain("未收集");
    expect(statusButtons[1]).toContain("已收集");
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
    // 阶段五：清单行与区域清单共用选中规则（左侧 3px 标记 + 浅底）。
    expect(row.element.closest("li")?.className).toContain("ui-row-selected");
  });

  it("清单行使用状态图形与文字（不只靠颜色）", async () => {
    const { wrapper } = await mountMap(1440);
    const row = wrapper.get("[data-item-id]");
    expect(row.find(".ui-status-mark").exists()).toBe(true);
    expect(row.text()).toMatch(/已收集|未收集/);
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
    const missingButton = statusGroup.findAll("button").find((button) => button.text().includes("未收集"))!;
    await missingButton.trigger("click");
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

describe("地图页：底图留白（2026-11-01）", () => {
  it("落在留白区的点位用虚线外环标出，并在图下说明数量", async () => {
    // 100×100 遮罩：右侧 25% 与北侧一段留白，正好覆盖 Kuwaq unit 2287 的 4 条。
    const width = 100;
    const height = 100;
    const alpha = new Uint8ClampedArray(width * height).fill(255);
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        if (x >= 75 || (y < 25 && x >= 35 && x <= 55)) {
          alpha[y * width + x] = 0;
        }
      }
    }
    maskState.mask = { width, height, alpha };

    const { store, wrapper } = await mountMap(1440);
    store.actions.setMapRegion("kuwaq_yaku");
    store.actions.setMapBasemap("bm:kuwaq_yaku/2287");
    await flushPromises();
    await wrapper.get('[data-testid="map-image"]').trigger("load");
    await flushPromises();

    const offMap = wrapper.findAll('[data-map-marker="true"][data-off-map="true"]');
    expect(offMap).toHaveLength(4);
    expect(wrapper.findAll('[data-testid="map-marker-off-map"]')).toHaveLength(4);
    expect(wrapper.get('[data-testid="map-off-map-note"]').text()).toContain("4 个点位");
    expect(wrapper.get('[data-testid="map-off-map-note"]').text()).toContain("底图留白区");
  });

  it("无法采样底图时不显示留白说明", async () => {
    const { wrapper } = await mountMap(1440);
    expect(wrapper.find('[data-testid="map-off-map-note"]').exists()).toBe(false);
    expect(wrapper.findAll('[data-map-marker="true"][data-off-map="true"]')).toHaveLength(0);
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
