// 可交互地图（2026-10-31 地图查询与详情增强·阶段二）：
// 标记数量/位置、已收集/未收集、选中、点击 emit、缩放按钮/滚轮、聚焦、失败态。
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";

// 让「已配置底图素材」成立：其余导出保持真实实现。
vi.mock("../src/data/basemaps", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/data/basemaps")>();
  return { ...actual, basemapImageUrl: (entry: Basemap) => `/basemaps/${entry.image}` };
});

import MapCanvas from "../src/components/map/MapCanvas.vue";
import type { MapEntry } from "../src/app/map-view";
import type { PlanGroup, PlanItem, PlanRegion } from "../src/data/atlas";
import type { Basemap } from "../src/data/basemaps";

function basemap(overrides: Partial<Basemap> = {}): Basemap {
  return {
    basemap_id: "bm:test_region/1",
    version: "0.1.0",
    region_id: "test_region",
    unit_ids: [1],
    image: "maps/unit.webp",
    image_width: 100,
    image_height: 50,
    transform: {
      kind: "affine",
      matrix: [100, 0, 0, 0, 50, 0],
      coordinate_y_axis: "down",
      image_origin: "top_left",
    },
    transform_basis: "analytic",
    status: "calibrated",
    median_error_px: 1,
    p95_error_px: 2,
    max_error_px: 3,
    error_source: "measured",
    validation_samples: 5,
    source_refs: ["src:fixture"],
    review: { state: "reviewed" },
    ...overrides,
  };
}

const region: PlanRegion = {
  slug: "test_region",
  region_hash: "hash",
  kind: "main",
  collected: 1,
  total: 3,
  groups: [],
};

const group: PlanGroup = {
  category: "survival_caches",
  name: "生存贮藏点",
  required_for_100: true,
  collected: 1,
  total: 3,
  acquisition_rule_key: "",
  items: [],
};

function planItem(id: string, x: number, y: number, collected = false): PlanItem {
  return {
    id,
    display_name: `收集品 ${id}`,
    status: collected ? "collected" : "not_collected",
    is_dlc: false,
    required_for_100: true,
    unit: { unit: 1, zone: "zone_1", display_zh: "子关卡 A", display_en: "Sub-level A", name_source: "curated" },
    position: { x, y, space: "unit_normalized_unverified" },
    acquisition_text: "",
    acquisition_source: "category_rule",
    location_hint: "",
  };
}

function entries(): MapEntry[] {
  return [
    { region, group, item: planItem("a", 0.5, 0.5, true) },
    { region, group, item: planItem("b", 0.2, 0.4) },
    { region, group, item: planItem("c", 0.95, 0.1) },
  ];
}

let observerStub: { observed: number; disconnected: number };

beforeAll(() => {
  observerStub = { observed: 0, disconnected: 0 };
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(_callback: ResizeObserverCallback) {}
      observe(): void {
        observerStub.observed += 1;
      }
      disconnect(): void {
        observerStub.disconnected += 1;
      }
      unobserve(): void {}
    },
  );
});

afterAll(() => {
  vi.unstubAllGlobals();
});

beforeEach(() => {
  // 容器 800×600：fit = min(8, 12) = 8 → 内层画布 800×400，垂直居中 100。
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

async function mountCanvas(overrides: Record<string, unknown> = {}) {
  const wrapper = mount(MapCanvas, {
    props: {
      basemap: basemap(),
      entries: entries(),
      selectedId: null,
      label: "测试区域 · 子关卡 A",
      ...overrides,
    },
    attachTo: document.body,
  });
  await nextTick();
  return wrapper;
}

describe("MapCanvas", () => {
  it("按底图比例绘制标记（像素位置来自变换矩阵）", async () => {
    const wrapper = await mountCanvas();
    const markers = wrapper.findAll("[data-map-marker='true']");
    expect(markers.length).toBe(3);
    expect(wrapper.get('[data-testid="map-canvas"]').attributes("data-zoom")).toBe("1");
    // (0.5, 0.5) → u=50, v=25 → ×8 → (400, 200)。
    const a = wrapper.get('[data-testid="map-marker-a"]').element as HTMLElement;
    expect(a.style.left).toBe("400px");
    expect(a.style.top).toBe("200px");
    expect(a.getAttribute("aria-label")).toContain("收集品 a");
    expect(a.getAttribute("aria-label")).toContain("已收集");
  });

  it("已收集/未收集与选中状态体现在标记上", async () => {
    const wrapper = await mountCanvas({ selectedId: "b" });
    expect(wrapper.get('[data-testid="map-marker-a"]').attributes("data-collected")).toBe("true");
    expect(wrapper.get('[data-testid="map-marker-b"]').attributes("data-collected")).toBe("false");
    expect(wrapper.get('[data-testid="map-marker-b"]').attributes("data-selected")).toBe("true");
    expect(wrapper.find('[data-testid="map-marker-ring"]').exists()).toBe(true);
    expect(wrapper.get('[data-testid="map-marker-a"]').attributes("data-category")).toBe("survival_caches");
  });

  it("点击标记发出条目 ID；点击空白发出 null", async () => {
    const wrapper = await mountCanvas();
    await wrapper.get('[data-testid="map-marker-b"]').trigger("click");
    expect(wrapper.emitted("select")?.at(-1)).toEqual(["b"]);
    await wrapper.get('[data-testid="map-canvas"]').trigger("click");
    expect(wrapper.emitted("select")?.at(-1)).toEqual([null]);
  });

  it("缩放按钮：放大/缩小/重置，标记尺寸不随缩放变化", async () => {
    const wrapper = await mountCanvas();
    await wrapper.get('[data-testid="map-zoom-in"]').trigger("click");
    expect(Number(wrapper.get('[data-testid="map-canvas"]').attributes("data-zoom"))).toBeCloseTo(1.35);
    const marker = wrapper.get('[data-testid="map-marker-a"]').element as HTMLElement;
    expect(marker.style.transform).toContain("scale(0.74074");
    await wrapper.get('[data-testid="map-zoom-out"]').trigger("click");
    await wrapper.get('[data-testid="map-zoom-reset"]').trigger("click");
    expect(wrapper.get('[data-testid="map-canvas"]').attributes("data-zoom")).toBe("1");
    expect(marker.style.transform).toBe("translate(-50%, -50%) scale(1)");
  });

  it("滚轮以指针为锚点缩放并阻止页面滚动；双击放大", async () => {
    const wrapper = await mountCanvas();
    const container = wrapper.get('[data-testid="map-canvas"]').element as HTMLElement;
    const wheel = new WheelEvent("wheel", { deltaY: -100, clientX: 400, clientY: 300, bubbles: true, cancelable: true });
    container.dispatchEvent(wheel);
    expect(wheel.defaultPrevented).toBe(true);
    await nextTick();
    expect(Number(container.getAttribute("data-zoom"))).toBeGreaterThan(1);

    const before = Number(container.getAttribute("data-zoom"));
    container.dispatchEvent(new MouseEvent("dblclick", { clientX: 400, clientY: 300, bubbles: true }));
    await nextTick();
    expect(Number(container.getAttribute("data-zoom"))).toBeGreaterThan(before);
  });

  it("focusItem 把标记带入可视区（不改变缩放）", async () => {
    const wrapper = await mountCanvas();
    await wrapper.get('[data-testid="map-zoom-in"]').trigger("click");
    const layer = wrapper.get('[data-testid="map-layer"]').element as HTMLElement;
    const before = layer.style.transform;
    (wrapper.vm as unknown as { focusItem: (id: string) => void }).focusItem("c");
    await nextTick();
    expect(layer.style.transform).not.toBe(before);
    expect(wrapper.get('[data-testid="map-canvas"]').attributes("data-zoom")).toBe("1.35");
  });

  it("底图未验证或图片加载失败时显示错误态（不显示破图）", async () => {
    const unverified = await mountCanvas({ basemap: basemap({ status: "unverified" }) });
    expect(unverified.text()).toContain("未经验证");
    expect(unverified.findAll("[data-map-marker='true']").length).toBe(0);

    const failed = await mountCanvas();
    await failed.get('[data-testid="map-image"]').trigger("error");
    expect(failed.text()).toContain("加载失败");
    expect(failed.findAll("[data-map-marker='true']").length).toBe(0);
  });

  it("aria-label 概述区域 · 子关卡与标记数", async () => {
    const wrapper = await mountCanvas();
    expect(wrapper.get('[data-testid="map-canvas"]').attributes("aria-label")).toContain("测试区域 · 子关卡 A");
    expect(wrapper.get('[data-testid="map-canvas"]').attributes("aria-label")).toContain("3 个收集品标记");
  });
});
