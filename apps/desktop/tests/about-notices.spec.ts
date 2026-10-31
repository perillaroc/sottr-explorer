// SX-018 关于区：许可通知离线全文与交付声明。
import { describe, expect, it } from "vitest";

import SettingsPage from "../src/components/SettingsPage.vue";
import { createTestStore, mountWithStore } from "./helpers";

describe("SettingsPage 关于与许可", () => {
  it("离线通知全文包含关键声明与依赖清单", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SettingsPage, store);
    const about = wrapper.find('[data-testid="about-notices"]');
    expect(about.exists()).toBe(true);
    const text = about.text();
    expect(text).toContain("WebView2");
    expect(text).toContain("SmartScreen");
    expect(text).toContain("TrRebootModTools");
    expect(text).toContain("MIT");
    expect(text).toContain("Rust 依赖");
    expect(text).toContain("JavaScript 依赖");
    expect(text).toContain("事实性数据");
    expect(text).toContain("43/43");
    expect(text).toContain("vi7ual");
    wrapper.unmount();
  });
});
