// UiStatusMark（2026-11-05 界面美化阶段一）：状态图形必须有文字 / aria，
// 不只靠颜色；未知状态不自行猜测。
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import UiStatusMark from "../src/components/ui/UiStatusMark.vue";

describe("UiStatusMark", () => {
  it("已收集渲染勾选圆形并提供可访问文字", () => {
    const wrapper = mount(UiStatusMark, { props: { status: "collected" } });

    expect(wrapper.classes()).toContain("ui-status-mark");
    expect(wrapper.classes()).toContain("ui-status-mark--collected");
    expect(wrapper.attributes("data-status")).toBe("collected");
    expect(wrapper.attributes("role")).toBe("img");
    expect(wrapper.attributes("aria-label")).toBe("已收集");
    expect(wrapper.attributes("title")).toBe("已收集");
    expect(wrapper.find("svg").exists()).toBe(true);
  });

  it("未收集使用空心圆形并提供可访问文字，不渲染勾选", () => {
    const wrapper = mount(UiStatusMark, { props: { status: "uncollected" } });

    expect(wrapper.classes()).toContain("ui-status-mark--uncollected");
    expect(wrapper.attributes("role")).toBe("img");
    expect(wrapper.attributes("aria-label")).toBe("未收集");
    expect(wrapper.find("svg").exists()).toBe(false);
  });

  it("未知状态不猜测，回退为未知文字", () => {
    // 运行时数据可能超出类型声明：组件不得把未知值当成已收集 / 未收集。
    const wrapper = mount(UiStatusMark, {
      props: { status: "mystery" as unknown as "unknown" },
    });

    expect(wrapper.classes()).toContain("ui-status-mark--unknown");
    expect(wrapper.attributes("data-status")).toBe("unknown");
    expect(wrapper.attributes("aria-label")).toBe("未知");
  });

  it("decorative 时由相邻文字表意，不重复播报", () => {
    const wrapper = mount(UiStatusMark, {
      props: { status: "collected", decorative: true },
    });

    expect(wrapper.attributes("aria-hidden")).toBe("true");
    expect(wrapper.attributes("role")).toBeUndefined();
    expect(wrapper.attributes("aria-label")).toBeUndefined();
    expect(wrapper.attributes("title")).toBeUndefined();
  });

  it("label 覆盖默认文字", () => {
    const wrapper = mount(UiStatusMark, {
      props: { status: "uncollected", label: "待补收集" },
    });

    expect(wrapper.attributes("aria-label")).toBe("待补收集");
    expect(wrapper.attributes("title")).toBe("待补收集");
  });
});
