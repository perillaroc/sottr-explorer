// 设置与主题测试（SX-015：明暗主题、布局恢复、关于信息可复核）。
import { describe, expect, it } from "vitest";
import SettingsPage from "../src/components/SettingsPage.vue";
import { createTestStore, mountWithStore } from "./helpers";

describe("设置与关于", () => {
  it("主题选择写入 <html data-theme>，跟随系统解析为具体主题", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SettingsPage, store);
    await wrapper.get('[data-testid="theme-dark"]').setValue();
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme-preference")).toBe("dark");
    await wrapper.get('[data-testid="theme-light"]').setValue();
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    await wrapper.get('[data-testid="theme-system"]').setValue();
    expect(["light", "dark"]).toContain(document.documentElement.getAttribute("data-theme"));
    expect(store.state.theme).toBe("system");
  });

  it("布局恢复可用，且展示元数据版本与已支持范围", async () => {
    const { store } = await createTestStore();
    store.state.navCollapsed = true;
    store.state.detailOpen = false;
    const wrapper = mountWithStore(SettingsPage, store);
    await wrapper.get("button").trigger("click");
    expect(store.state.navCollapsed).toBe(false);
    expect(store.state.detailOpen).toBe(true);
    const text = wrapper.text();
    expect(text).toContain("元数据版本");
    expect(text).toContain("已验证坐标 0 项");
    expect(text).toContain("THIRD_PARTY_LICENSES.md");
    expect(text).toContain("Ctrl+O");
  });
});
