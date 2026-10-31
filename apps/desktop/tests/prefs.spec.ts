// 界面偏好持久化（SX-015；外部链接偏好见 2026-10-17 详情重设计阶段二）：
// 往返读写与容错解析——坏数据只回落受影响字段，不整份丢弃。
import { beforeEach, describe, expect, it } from "vitest";
import { clearPreferences, loadPreferences, savePreferences } from "../src/app/prefs";

// 显式固定存储键：它是对外契约（旧版本写下的偏好必须继续能被读到）。
const STORAGE_KEY = "sottr-explorer.ui.v1";

describe("界面偏好持久化", () => {
  beforeEach(() => {
    clearPreferences();
  });

  it("未写入过时使用默认值", () => {
    const preferences = loadPreferences();
    expect(preferences.searchEngine).toBe("bing");
    expect(preferences.linkOpenMode).toBe("open");
  });

  it("往返：写入后读回同一组外部链接偏好", () => {
    savePreferences({ ...loadPreferences(), searchEngine: "baidu", linkOpenMode: "copy" });
    const preferences = loadPreferences();
    expect(preferences.searchEngine).toBe("baidu");
    expect(preferences.linkOpenMode).toBe("copy");
    expect(preferences.theme).toBe("system");
  });

  it("容错：非法枚举回落默认，其余字段照常读回", () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        theme: "dark",
        navCollapsed: true,
        searchEngine: "duckduckgo",
        linkOpenMode: "window",
      }),
    );
    const preferences = loadPreferences();
    expect(preferences.searchEngine).toBe("bing");
    expect(preferences.linkOpenMode).toBe("open");
    expect(preferences.theme).toBe("dark");
    expect(preferences.navCollapsed).toBe(true);
  });

  it("容错：旧版本偏好（无这两个字段）仍可读，坏 JSON 回落全默认", () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ theme: "light", navCollapsed: true }));
    const legacy = loadPreferences();
    expect(legacy.theme).toBe("light");
    expect(legacy.searchEngine).toBe("bing");
    expect(legacy.linkOpenMode).toBe("open");

    window.localStorage.setItem(STORAGE_KEY, "{ 不是 JSON");
    const broken = loadPreferences();
    expect(broken.searchEngine).toBe("bing");
    expect(broken.linkOpenMode).toBe("open");
    expect(broken.theme).toBe("system");
  });

  it("面板宽度：默认值、钳制与往返（阶段一）", () => {
    const defaults = loadPreferences();
    expect(defaults.detailPaneWidth).toBe(400);
    expect(defaults.detailDrawerWidth).toBe(400);
    expect(defaults.detailCompactWidth).toBe(720);
    expect(defaults.mapRailWidth).toBe(420);

    savePreferences({
      ...defaults,
      detailPaneWidth: 480,
      detailDrawerWidth: 520,
      detailCompactWidth: 760,
      mapRailWidth: 500,
    });
    const roundTrip = loadPreferences();
    expect(roundTrip.detailPaneWidth).toBe(480);
    expect(roundTrip.detailDrawerWidth).toBe(520);
    expect(roundTrip.detailCompactWidth).toBe(760);
    expect(roundTrip.mapRailWidth).toBe(500);
  });

  it("面板宽度：越界值钳制、非数字回退默认，旧偏好缺字段时仍可读", () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        detailPaneWidth: 100,
        detailDrawerWidth: 5000,
        detailCompactWidth: 640,
        mapRailWidth: "wide",
      }),
    );
    const parsed = loadPreferences();
    expect(parsed.detailPaneWidth).toBe(320);
    expect(parsed.detailDrawerWidth).toBe(900);
    expect(parsed.detailCompactWidth).toBe(640);
    expect(parsed.mapRailWidth).toBe(420);

    // 旧版本偏好没有宽度字段：全部回落默认值，不影响其余字段。
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ theme: "dark" }));
    const legacy = loadPreferences();
    expect(legacy.theme).toBe("dark");
    expect(legacy.detailPaneWidth).toBe(400);
    expect(legacy.detailCompactWidth).toBe(720);
  });
});
