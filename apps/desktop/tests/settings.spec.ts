// 设置与主题测试（SX-015：明暗主题、布局恢复、关于信息可复核）。
import { describe, expect, it } from "vitest";
import SettingsPage from "../src/components/SettingsPage.vue";
import { createTestStore, mountWithStore } from "./helpers";
import { createAppStore } from "../src/app/store";
import { MockDataSource } from "../src/data/mock";
import { clearPreferences } from "../src/app/prefs";

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

  it("外部链接：引擎与打开方式可切换，默认 Bing + 打开浏览器", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SettingsPage, store);
    expect((wrapper.get('[data-testid="search-engine-bing"]').element as HTMLInputElement).checked).toBe(true);
    expect((wrapper.get('[data-testid="link-open-open"]').element as HTMLInputElement).checked).toBe(true);

    await wrapper.get('[data-testid="search-engine-baidu"]').setValue();
    expect(store.state.searchEngine).toBe("baidu");
    await wrapper.get('[data-testid="link-open-copy"]').setValue();
    expect(store.state.linkOpenMode).toBe("copy");

    // 单选组状态跟随 store（切换后新选项为选中态）。
    expect((wrapper.get('[data-testid="search-engine-baidu"]').element as HTMLInputElement).checked).toBe(true);
    expect((wrapper.get('[data-testid="link-open-copy"]').element as HTMLInputElement).checked).toBe(true);
  });

  it("外部链接设置经 localStorage 往返：重建 store（刷新）后仍在", async () => {
    clearPreferences();
    const source = new MockDataSource({ delayMs: 0 });
    const store = createAppStore(source, { persist: true, layoutWidth: 1280 });
    const wrapper = mountWithStore(SettingsPage, store);
    await wrapper.get('[data-testid="search-engine-google"]').setValue();
    await wrapper.get('[data-testid="link-open-copy"]').setValue();

    const reloaded = createAppStore(source, { persist: true, layoutWidth: 1280 });
    expect(reloaded.state.searchEngine).toBe("google");
    expect(reloaded.state.linkOpenMode).toBe("copy");
    clearPreferences();
  });
});
