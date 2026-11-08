// RegionProgressRow（2026-11-05 界面美化阶段三）：概览与全部区域共用的区域入口。
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import RegionProgressRow from "../src/components/RegionProgressRow.vue";

describe("RegionProgressRow", () => {
  it("显示名称、计数、未收集数量与可访问进度条，点击发出 open", async () => {
    const wrapper = mount(RegionProgressRow, {
      props: { slug: "paititi", name: "帕依提提", collected: 105, total: 159, testId: "region-row" },
    });

    const row = wrapper.get('[data-testid="region-row"]');
    expect(row.text()).toContain("帕依提提");
    expect(row.text()).toContain("105/159");
    expect(row.text()).toContain("未收集 54 项");
    const bar = wrapper.get('[role="progressbar"]');
    expect(bar.attributes("aria-label")).toContain("帕依提提");
    expect(bar.attributes("aria-valuenow")).toBe("66");

    await row.trigger("click");
    expect(wrapper.emitted("open")).toEqual([["paititi"]]);
  });

  it("全部收集时同时显示文字与勾选", () => {
    const wrapper = mount(RegionProgressRow, {
      props: { slug: "head", name: "巨蛇之首", collected: 1, total: 1 },
    });

    expect(wrapper.text()).toContain("已全部收集");
    expect(wrapper.find("svg").exists()).toBe(true);
  });

  it("总数为 0 时不宣告完成", () => {
    const wrapper = mount(RegionProgressRow, {
      props: { slug: "empty", name: "空区域", collected: 0, total: 0 },
    });

    expect(wrapper.text()).toContain("无条目");
    expect(wrapper.text()).not.toContain("已全部收集");
    expect(wrapper.find("svg").exists()).toBe(false);
  });

  it("提供地图按钮时发出 openMap，并有可读名称；不影响 open", async () => {
    const wrapper = mount(RegionProgressRow, {
      props: {
        slug: "paititi",
        name: "帕依提提",
        collected: 1,
        total: 2,
        mapAvailable: true,
        mapTestId: "region-map",
      },
    });

    const map = wrapper.get('[data-testid="region-map"]');
    expect(map.attributes("aria-label")).toContain("帕依提提");
    await map.trigger("click");
    expect(wrapper.emitted("openMap")).toEqual([["paititi"]]);
    expect(wrapper.emitted("open")).toBeUndefined();
  });

  it("未提供地图能力时不渲染地图按钮", () => {
    const wrapper = mount(RegionProgressRow, {
      props: { slug: "croft", name: "克劳馥庄园", collected: 0, total: 5 },
    });

    expect(wrapper.find('[data-testid="region-map"]').exists()).toBe(false);
  });
});
