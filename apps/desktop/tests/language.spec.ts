// 界面语言（2026-10-20）：中文 / English / 中英文对照，默认中文；偏好持久化。
import { beforeEach, describe, expect, it } from "vitest";
import SettingsPage from "../src/components/SettingsPage.vue";
import CollectionPage from "../src/components/CollectionPage.vue";
import { createAppStore } from "../src/app/store";
import { clearPreferences, loadPreferences, savePreferences } from "../src/app/prefs";
import { setLanguage } from "../src/app/i18n";
import { findPlanItem } from "../src/data/atlas";
import { MockDataSource } from "../src/data/mock";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

describe("界面语言", () => {
  beforeEach(() => {
    clearPreferences();
    setLanguage("zh");
  });

  it("默认中文：设置页只显示中文，中文选项选中", async () => {
    const { store } = await createTestStore();
    expect(store.state.language).toBe("zh");
    expect(document.documentElement.lang).toBe("zh-CN");

    const wrapper = mountWithStore(SettingsPage, store);
    expect((wrapper.get('[data-testid="language-zh"]').element as HTMLInputElement).checked).toBe(true);
    expect(wrapper.text()).toContain("设置与关于");
    expect(wrapper.text()).toContain("语言");
    expect(wrapper.text()).not.toContain("Settings & About");
  });

  it("切换 English：界面文案、<html lang> 与图鉴类别名切换", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    store.actions.setView("regions");
    store.actions.setRegion("mission_of_san_juan");

    const wrapper = mountWithStore(CollectionPage, store);
    await store.actions.setLanguage("en");
    await wrapper.vm.$nextTick();
    expect(store.state.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(wrapper.text()).toContain("Regions & Collection");
    expect(wrapper.text()).toContain("Main regions");
    expect(wrapper.text()).toContain("Collected");
    expect(wrapper.text()).not.toContain("区域与收集");
  });

  it("中英文对照：静态文案与收集品名称并列显示", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const wrapper = mountWithStore(SettingsPage, store);

    await store.actions.setLanguage("both");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("设置与关于 / Settings & About");
    expect(wrapper.text()).toContain("语言 / Language");

    const entry = findPlanItem(store.plan.value, "0xF97006F6");
    expect(entry).not.toBeNull();
    expect(entry?.item.display_name).toBe("隐秘之城的谜团 / HIDDEN CITY RIDDLE");
    expect(entry?.item.location_hint).toContain("特殊 / 跨区域");
  });

  it("语言偏好写入 localStorage：重建 store 后仍是 English", () => {
    savePreferences({ ...loadPreferences(), language: "en" });
    const source = new MockDataSource({ delayMs: 0 });
    const reloaded = createAppStore(source, { persist: true, layoutWidth: 1280 });
    expect(reloaded.state.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
  });

  it("旧版本偏好没有 language 字段时回落中文", () => {
    window.localStorage.setItem(
      "sottr-explorer.ui.v1",
      JSON.stringify({ theme: "dark", navCollapsed: true }),
    );
    const preferences = loadPreferences();
    expect(preferences.language).toBe("zh");
  });
});
