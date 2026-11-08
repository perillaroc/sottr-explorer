// UiProgressBar（2026-11-05 界面美化阶段一）：0 / 50 / 100%、总数为 0、
// 数值钳制与可访问标签。
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import UiProgressBar from "../src/components/ui/UiProgressBar.vue";

function fillWidth(wrapper: ReturnType<typeof mount>): string {
  return (wrapper.find(".ui-progress__fill").element as HTMLElement).style.width;
}

describe("UiProgressBar", () => {
  it("按已收集 / 总数计算百分比并暴露可访问文本", () => {
    const wrapper = mount(UiProgressBar, {
      props: { value: 5, total: 10, label: "区域收集进度" },
    });

    expect(wrapper.attributes("role")).toBe("progressbar");
    expect(wrapper.attributes("aria-label")).toBe("区域收集进度");
    expect(wrapper.attributes("aria-valuemin")).toBe("0");
    expect(wrapper.attributes("aria-valuemax")).toBe("100");
    expect(wrapper.attributes("aria-valuenow")).toBe("50");
    expect(wrapper.attributes("aria-valuetext")).toBe("5 / 10");
    expect(fillWidth(wrapper)).toBe("50%");
  });

  it("0% 与 100% 边界", () => {
    const empty = mount(UiProgressBar, { props: { value: 0, total: 10, label: "进度" } });
    expect(empty.attributes("aria-valuenow")).toBe("0");
    expect(empty.attributes("aria-valuetext")).toBe("0 / 10");
    expect(fillWidth(empty)).toBe("0%");

    const full = mount(UiProgressBar, { props: { value: 10, total: 10, label: "进度" } });
    expect(full.attributes("aria-valuenow")).toBe("100");
    expect(full.attributes("aria-valuetext")).toBe("10 / 10");
    expect(fillWidth(full)).toBe("100%");
  });

  it("总数为 0 时按 0% 处理，不产生 NaN", () => {
    const wrapper = mount(UiProgressBar, { props: { value: 0, total: 0, label: "进度" } });

    expect(wrapper.attributes("aria-valuenow")).toBe("0");
    expect(wrapper.attributes("aria-valuetext")).toBe("0 / 0");
    expect(fillWidth(wrapper)).toBe("0%");
  });

  it("数值钳制在 [0, total] 内", () => {
    const over = mount(UiProgressBar, { props: { value: 15, total: 10, label: "进度" } });
    expect(over.attributes("aria-valuenow")).toBe("100");
    expect(over.attributes("aria-valuetext")).toBe("10 / 10");
    expect(fillWidth(over)).toBe("100%");

    const negative = mount(UiProgressBar, { props: { value: -3, total: 10, label: "进度" } });
    expect(negative.attributes("aria-valuenow")).toBe("0");
    expect(negative.attributes("aria-valuetext")).toBe("0 / 10");
    expect(fillWidth(negative)).toBe("0%");
  });

  it("非有限数值不产生 NaN", () => {
    const wrapper = mount(UiProgressBar, {
      props: { value: Number.NaN, total: 10, label: "进度" },
    });

    expect(wrapper.attributes("aria-valuenow")).toBe("0");
    expect(wrapper.attributes("aria-valuetext")).toBe("0 / 10");
    expect(fillWidth(wrapper)).toBe("0%");
  });

  it("必收色调、主进度高度与测试定位", () => {
    const wrapper = mount(UiProgressBar, {
      props: { value: 352, total: 352, label: "必收进度", tone: "gold", size: "md", testId: "required-bar" },
    });

    expect(wrapper.attributes("data-testid")).toBe("required-bar");
    expect(wrapper.classes()).toContain("ui-progress--md");
    expect(wrapper.find(".ui-progress__fill").classes()).toContain("ui-progress__fill--gold");
    expect(wrapper.attributes("aria-valuenow")).toBe("100");
  });
});
