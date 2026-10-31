// SX-018 关于区：许可通知离线全文与交付声明。
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";

import AboutPage from "../src/components/AboutPage.vue";
import { createTestStore, mountWithStore } from "./helpers";

describe("AboutPage 关于与许可", () => {
  it("离线通知全文包含关键声明与依赖清单", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(AboutPage, store);
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

  it("依赖许可全文可离线展开，含产品许可与各类依赖正文", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(AboutPage, store);
    const details = wrapper.get('[data-testid="about-license-full"]');
    (details.element as HTMLDetailsElement).open = true;
    await details.trigger("toggle");
    for (let i = 0; i < 100 && !wrapper.find('[data-testid="license-full-text"]').exists(); i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 50));
      await flushPromises();
    }
    const text = wrapper.get('[data-testid="license-full-text"]').text();
    expect(text).toContain("MIT License");
    expect(text).toContain("Apache License");
    expect(text).toContain("Mozilla Public License Version 2.0");
    expect(text).toContain("selectors 0.38.0");
    expect(text).toContain("包内未附完整许可正文的依赖");
    expect(text).toContain("只有许可标识/SPDX 指针");
    wrapper.unmount();
  });
});
