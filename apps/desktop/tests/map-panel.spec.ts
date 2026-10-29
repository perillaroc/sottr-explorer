// 底图面板（P3-01）：底图模式的点位投影，以及无素材/未验证/缺记录时的回退。
import { describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";

// 让「已配置外置素材」成立：其余导出保持真实实现。
vi.mock("../src/data/basemaps", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/data/basemaps")>();
  return { ...actual, basemapImageUrl: () => "/basemaps/maps/unit.webp" };
});

import MapPanel from "../src/components/MapPanel.vue";
import { resolveBasemapMode, type Basemap } from "../src/data/basemaps";
import type { PlanItem } from "../src/data/atlas";

function item(id: string, x: number, y: number, collected = false): PlanItem {
  return {
    id,
    display_name: id,
    status: collected ? "collected" : "not_collected",
    is_dlc: false,
    required_for_100: true,
    position: { x, y, space: "unit_normalized_unverified" },
    acquisition_text: "",
    acquisition_source: "category_rule",
    location_hint: "",
  };
}

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

describe("底图显示判定（P3-02）", () => {
  const url = "/basemaps/maps/unit.webp";

  it("有记录、状态非 unverified、图片可用时画底图", () => {
    expect(resolveBasemapMode(basemap(), url, false)).toEqual({ mode: "map", reason: "map" });
    expect(resolveBasemapMode(basemap({ status: "approximate" }), url, false)).toEqual({
      mode: "map",
      reason: "map",
    });
  });

  it("没有记录时回退，原因 no-record", () => {
    expect(resolveBasemapMode(null, url, false)).toEqual({
      mode: "scatter",
      reason: "no-record",
    });
  });

  it("状态为 unverified 时回退，原因 unverified", () => {
    expect(resolveBasemapMode(basemap({ status: "unverified" }), url, false)).toEqual({
      mode: "scatter",
      reason: "unverified",
    });
  });

  it("没有图片素材时回退，原因 no-image", () => {
    expect(resolveBasemapMode(basemap(), null, false)).toEqual({
      mode: "scatter",
      reason: "no-image",
    });
  });

  it("图片加载失败时回退，原因 image-error", () => {
    expect(resolveBasemapMode(basemap(), url, true)).toEqual({
      mode: "scatter",
      reason: "image-error",
    });
  });
});

describe("点位语义（P3-03）", () => {
  it("已收集实心、未收集空心，当前条目外圈并最后绘制", () => {
    const wrapper = mount(MapPanel, {
      props: {
        items: [
          item("a", 0.2, 0.2, true),
          item("b", 0.5, 0.5),
          item("c", 0.8, 0.8),
        ],
        selectedId: "c",
        basemap: basemap(),
      },
    });
    const dots = wrapper.findAll('[data-testid="basemap-dot"]');
    expect(dots).toHaveLength(3);

    const collected = dots.find((dot) => dot.attributes("data-collected") === "true");
    expect(collected?.classes()).toContain("bg-[var(--accent-strong)]");

    const hollow = dots.filter((dot) => dot.attributes("data-collected") === "false");
    expect(hollow).toHaveLength(2);
    expect(hollow[0].classes()).toContain("bg-transparent");
    expect(hollow[0].classes()).toContain("border-[var(--muted)]");

    // 当前条目排在最后，避免被相邻点位盖住外圈。
    const last = dots[dots.length - 1];
    expect(last.attributes("data-selected")).toBe("true");
    expect(last.classes()).toContain("z-10");
    expect(wrapper.findAll('[data-testid="basemap-dot-ring"]')).toHaveLength(1);
  });

  it("没有当前条目时不画外圈", () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: null, basemap: basemap() },
    });
    expect(wrapper.find('[data-testid="basemap-dot-ring"]').exists()).toBe(false);
  });
});

describe("底图自适应（P3-04）", () => {
  it("方图宽度上限等于最大高度 320", () => {
    const square = basemap({ image_width: 2048, image_height: 2048 });
    square.transform.matrix = [2048, 0, 0, 0, 2048, 0];
    const wrapper = mount(MapPanel, {
      props: { items: [], selectedId: null, basemap: square },
    });
    const frame = wrapper.get('[data-testid="basemap-frame"]');
    expect(frame.classes()).toContain("w-full");
    expect(frame.attributes("style")).toContain("aspect-ratio: 2048 / 2048");
    expect(frame.attributes("style")).toContain("max-width: min(100%, 320px)");
  });

  it("宽图按宽高比放宽上限，仍不超过最大高度", () => {
    const wide = basemap({ image_width: 2048, image_height: 1024 });
    wide.transform.matrix = [2048, 0, 0, 0, 1024, 0];
    const wrapper = mount(MapPanel, {
      props: { items: [], selectedId: null, basemap: wide },
    });
    const style = wrapper.get('[data-testid="basemap-frame"]').attributes("style");
    expect(style).toContain("aspect-ratio: 2048 / 1024");
    expect(style).toContain("max-width: min(100%, 640px)");
  });
});

describe("懒加载（P3-05）", () => {
  it("未展开时不挂载图片与点位，展开后才挂载", async () => {
    const props = { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: basemap() };
    const wrapper = mount(MapPanel, { props: { ...props, active: false } });
    expect(wrapper.find('[data-testid="basemap-image"]').exists()).toBe(false);
    expect(wrapper.findAll('[data-testid="basemap-dot"]')).toHaveLength(0);
    // 判定仍是「有底图可用」，只是没展开。
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("map");
    expect(wrapper.find('[data-testid="basemap-frame"]').exists()).toBe(true);

    await wrapper.setProps({ active: true });
    expect(wrapper.get('[data-testid="basemap-image"]').attributes("src")).toBe(
      "/basemaps/maps/unit.webp",
    );
    expect(wrapper.findAll('[data-testid="basemap-dot"]')).toHaveLength(1);
  });

  it("未传 active 时按展开处理（默认立即加载）", () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: basemap() },
    });
    expect(wrapper.find('[data-testid="basemap-image"]').exists()).toBe(true);
  });
});

describe("底图面板", () => {
  it("按矩阵把归一化坐标投影到位图百分比", () => {
    const wrapper = mount(MapPanel, {
      props: {
        items: [item("a", 0.25, 0.5, true), item("b", 0.75, 0.2)],
        selectedId: "a",
        basemap: basemap(),
      },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("map");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-reason")).toBe("map");
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="basemap-image"]').attributes("src")).toBe(
      "/basemaps/maps/unit.webp",
    );

    const dots = wrapper.findAll('[data-testid="basemap-dot"]');
    expect(dots).toHaveLength(2);
    const selected = dots.find((dot) => dot.attributes("data-selected") === "true");
    expect(selected?.attributes("style")).toContain("left: 25%");
    expect(selected?.attributes("style")).toContain("top: 50%");
    expect(selected?.attributes("data-collected")).toBe("true");
    const other = dots.find((dot) => dot.attributes("data-selected") === "false");
    expect(other?.attributes("style")).toContain("left: 75%");
    expect(other?.attributes("style")).toContain("top: 20%");
  });

  it("没有基底层记录时回退散点", () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: null },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("scatter");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-reason")).toBe("no-record");
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("未校准");
  });

  it("状态为 unverified 时即使有记录也回退散点", () => {
    const wrapper = mount(MapPanel, {
      props: {
        items: [item("a", 0.5, 0.5)],
        selectedId: "a",
        basemap: basemap({ status: "unverified", median_error_px: null, p95_error_px: null, max_error_px: null }),
      },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("scatter");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-reason")).toBe("unverified");
  });

  it("图片加载失败后回退散点", async () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: basemap() },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("map");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-reason")).toBe("map");
    await wrapper.get('[data-testid="basemap-image"]').trigger("error");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("scatter");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-reason")).toBe("image-error");
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(true);
  });

  it("近似状态改用语「近似位置」", () => {
    const wrapper = mount(MapPanel, {
      props: {
        items: [item("a", 0.5, 0.5)],
        selectedId: "a",
        basemap: basemap({ status: "approximate" }),
      },
    });
    expect(wrapper.text()).toContain("近似位置");
  });
});
