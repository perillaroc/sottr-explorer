// 应用外壳测试（SX-015：模拟数据横幅、当前文件标识、多槽结果不串用、提示可关闭）。
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import App from "../src/App.vue";
import { createTestStore, analyzeCandidate, candidatePath, mountWithStore } from "./helpers";

describe("应用外壳", () => {
  it("模拟数据横幅在流程中可见，并标注演示视口宽度", async () => {
    const { store } = await createTestStore({ width: 1280 });
    const wrapper = mountWithStore(App, store, { store, layoutWidth: 1280 });
    await flushPromises();
    expect(wrapper.get('[data-testid="mock-banner"]').text()).toContain("模拟数据");
    store.actions.setMockWidth(800);
    await flushPromises();
    expect(wrapper.get('[data-testid="mock-banner"]').text()).toContain("演示视口宽度 800px");
  });

  it("多槽结果不串用：顶栏与概览始终显示当前文件与其快照", async () => {
    const { store } = await createTestStore({ width: 1280 });
    await analyzeCandidate(store, "save1.dat");
    const firstSnapshot = store.currentSlot.value?.result?.snapshot_id;
    const firstDigest = store.report.value?.input_digest.compressed_sha256;

    store.actions.switchSlot(candidatePath("save204.dat"));
    await analyzeCandidate(store, "save204.dat");
    expect(store.currentSlot.value?.name).toBe("save204.dat");
    expect(store.report.value?.input_digest.compressed_sha256).not.toBe(firstDigest);

    const wrapper = mountWithStore(App, store, { store, layoutWidth: 1280 });
    await flushPromises();
    expect(wrapper.get('[data-testid="current-file"]').text()).toBe("save204.dat");
    expect(wrapper.get('[data-testid="current-file"]').text()).not.toContain("save1.dat");

    store.actions.switchSlot(candidatePath("save1.dat"));
    await flushPromises();
    expect(wrapper.get('[data-testid="current-file"]').text()).toBe("save1.dat");
    expect(store.currentSlot.value?.result?.snapshot_id).toBe(firstSnapshot);
  });

  it("提示条可关闭；分析状态靠近文件选择器", async () => {
    const { store } = await createTestStore({ width: 1280 });
    await analyzeCandidate(store, "save1.dat");
    store.state.notice = { kind: "info", text: "测试提示" };
    const wrapper = mountWithStore(App, store, { store, layoutWidth: 1280 });
    await flushPromises();
    expect(wrapper.get('[data-testid="notice"]').text()).toContain("测试提示");
    await wrapper.get('[data-testid="notice"] button').trigger("click");
    expect(wrapper.find('[data-testid="notice"]').exists()).toBe(false);
    expect(wrapper.get('[data-testid="analysis-status"]').text()).toContain("分析完成");
  });
});
