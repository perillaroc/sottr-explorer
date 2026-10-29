// 底图面板（P3-01）：底图模式的点位投影，以及无素材/未验证/缺记录时的回退。
import { describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";

// 让「已配置外置素材」成立：其余导出保持真实实现。
vi.mock("../src/data/basemaps", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/data/basemaps")>();
  return { ...actual, basemapImageUrl: () => "/basemaps/maps/unit.webp" };
});

import MapPanel from "../src/components/MapPanel.vue";
import type { Basemap } from "../src/data/basemaps";
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
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="basemap-image"]').attributes("src")).toBe(
      "/basemaps/maps/unit.webp",
    );

    const dots = wrapper.findAll('[data-testid="basemap-dot"]');
    expect(dots).toHaveLength(2);
    expect(dots[0].attributes("style")).toContain("left: 25%");
    expect(dots[0].attributes("style")).toContain("top: 50%");
    expect(dots[0].attributes("data-collected")).toBe("true");
    expect(dots[0].attributes("data-selected")).toBe("true");
    expect(dots[1].attributes("style")).toContain("left: 75%");
    expect(dots[1].attributes("style")).toContain("top: 20%");
  });

  it("没有基底层记录时回退散点", () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: null },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("scatter");
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
  });

  it("图片加载失败后回退散点", async () => {
    const wrapper = mount(MapPanel, {
      props: { items: [item("a", 0.5, 0.5)], selectedId: "a", basemap: basemap() },
    });
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("map");
    await wrapper.get('[data-testid="basemap-image"]').trigger("error");
    expect(wrapper.get('[data-testid="map-panel"]').attributes("data-mode")).toBe("scatter");
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
