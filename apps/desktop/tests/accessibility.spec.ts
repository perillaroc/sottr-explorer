// P4-4 辅助技术回归：跨页结构不变量（landmark、可访问名称、表单标签、
// 跳到正文、对话框命名、强制性颜色样式）。
// Narrator 与 Windows 系统高对比度仍需人工，步骤与记录见
// docs/development/accessibility.md 与开发工作区的验收记录。
import { readFileSync } from "node:fs";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import App from "../src/App.vue";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";

const styles = readFileSync("src/styles.css", "utf8");

const VIEWS = [
  "import",
  "overview",
  "full",
  "plan",
  "regions",
  "map",
  "diagnostics",
  "settings",
  "about",
] as const;

beforeAll(() => {
  // MapPage 依赖 ResizeObserver；jsdom 没有实现。
  globalThis.ResizeObserver = class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  } as unknown as typeof ResizeObserver;
});

afterAll(() => {
  // @ts-expect-error 测试环境清理
  delete globalThis.ResizeObserver;
});

function accessibleName(element: Element): string {
  const tag = element.tagName.toLowerCase();
  if (tag === "input" || tag === "select" || tag === "textarea") {
    const label = element.closest("label");
    if (label && (label.textContent ?? "").trim().length > 0) {
      return (label.textContent ?? "").trim();
    }
  }
  const aria = element.getAttribute("aria-label");
  if (aria && aria.trim().length > 0) {
    return aria.trim();
  }
  const labelledBy = element.getAttribute("aria-labelledby");
  if (labelledBy) {
    const text = labelledBy
      .split(/\s+/)
      .map((id) => document.getElementById(id)?.textContent ?? "")
      .join(" ")
      .trim();
    if (text.length > 0) {
      return text;
    }
  }
  const text = (element.textContent ?? "").trim();
  if (text.length > 0) {
    return text;
  }
  const title = element.getAttribute("title");
  if (title && title.trim().length > 0) {
    return title.trim();
  }
  return "";
}

function hasLabel(element: Element): boolean {
  if (element.getAttribute("aria-label") || element.getAttribute("aria-labelledby")) {
    return true;
  }
  if (element.closest("label")) {
    return true;
  }
  const id = element.getAttribute("id");
  return Boolean(id && document.querySelector(`label[for="${id}"]`));
}

function controls(root: ParentNode): Element[] {
  return [...root.querySelectorAll("button, a[href], input, select, textarea, summary")].filter(
    (element) => !element.closest('[aria-hidden="true"]') && !element.hasAttribute("hidden"),
  );
}

async function mountApp(width = 1280) {
  const { store } = await createTestStore({ width });
  await analyzeCandidate(store, "save1.dat");
  store.actions.setRegion("mission_of_san_juan");
  store.actions.setStatusFilter("all");
  const wrapper = mountWithStore(App, store, { store, layoutWidth: width });
  await flushPromises();
  return { store, wrapper };
}

describe("辅助技术回归", () => {
  it("样式：焦点环不依赖阴影，并为强制性颜色提供选中态轮廓", () => {
    expect(styles).toMatch(/:focus-visible\s*\{[^}]*outline:\s*2px solid var\(--focus\)/s);
    expect(styles).toMatch(/@media \(forced-colors: active\)/);
    expect(styles).toMatch(/outline:\s*2px solid Highlight/);
    expect(styles).not.toMatch(/:focus-visible\s*\{[^}]*outline:\s*none/s);
  });

  it("跳到正文：首个可聚焦元素，指向带 tabindex 的 main", async () => {
    const { wrapper } = await mountApp();
    const root = wrapper.get('[data-testid="app-root"]').element;
    const skip = wrapper.get('[data-testid="skip-link"]');
    const focusables = [...root.querySelectorAll("a[href], button, input, select, textarea, [tabindex]")].filter(
      (element) => !element.hasAttribute("hidden"),
    );
    expect(focusables[0]).toBe(skip.element);
    expect(skip.attributes("href")).toBe("#main-content");

    const main = wrapper.get("main#main-content");
    expect(main.attributes("tabindex")).toBe("-1");
    expect(main.attributes("id")).toBe("main-content");
  });

  it("landmark：单一 main、有名称的导航、页面视图各有一个 h1", async () => {
    const { store, wrapper } = await mountApp();
    expect(wrapper.findAll("main")).toHaveLength(1);
    const nav = wrapper.get("nav");
    expect(nav.attributes("aria-label")).toBeTruthy();

    for (const view of VIEWS) {
      store.actions.setView(view);
      await flushPromises();
      const headings = wrapper.findAll("main h1");
      expect(headings.length, `${view} 应有且只有一个 h1`).toBe(1);
      expect(headings[0].text().trim().length).toBeGreaterThan(0);
    }
  });

  it("所有视图的可交互控件都有可访问名称", async () => {
    const { store, wrapper } = await mountApp();
    for (const view of VIEWS) {
      store.actions.setView(view);
      await flushPromises();
      const unnamed = controls(wrapper.get("main").element)
        .filter((element) => accessibleName(element).length === 0)
        .map((element) => element.outerHTML.slice(0, 120));
      expect(unnamed, `${view} 存在无名称控件`).toEqual([]);
    }
  });

  it("表单控件都有标签：设置页的显示、布局与外部链接选项", async () => {
    const { store, wrapper } = await mountApp();
    store.actions.setView("settings");
    await flushPromises();
    const inputs = [...wrapper.get("main").element.querySelectorAll("input, select, textarea")].filter(
      (element) => !element.hasAttribute("hidden") && (element as HTMLInputElement).type !== "hidden",
    );
    expect(inputs.length).toBeGreaterThan(0);
    const unlabeled = inputs.filter((element) => !hasLabel(element)).map((element) => element.outerHTML.slice(0, 120));
    expect(unlabeled).toEqual([]);
  });

  it("详情抽屉：role=dialog、有名称，打开后焦点进入", async () => {
    const { store } = await mountApp(1280);
    store.actions.openRow("0xCAA74459");
    await flushPromises();
    const dialog = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute("aria-label")?.trim().length).toBeGreaterThan(0);
    expect(dialog?.contains(document.activeElement)).toBe(true);
  });

  it("地图页：区域与标记以可访问名称暴露（含收集状态）", async () => {
    const { store, wrapper } = await mountApp();
    store.actions.setView("map");
    await flushPromises();
    const canvas = wrapper.find('[data-testid="map-canvas"]');
    if (canvas.exists()) {
      expect(canvas.attributes("aria-label")).toBeTruthy();
      for (const marker of wrapper.findAll("[data-map-marker='true']")) {
        expect(marker.attributes("aria-label")?.trim().length).toBeGreaterThan(0);
      }
    }
    // 无论画布是否可用，清单标签页必须存在且可被读屏定位。
    expect(wrapper.text()).toContain("详情");
  });

  it("状态提示是礼貌级 live region，不打断当前朗读", async () => {
    const { store, wrapper } = await mountApp();
    store.actions.setView("import");
    await flushPromises();
    const notice = wrapper.find('[data-testid="notice"]');
    if (notice.exists()) {
      expect(notice.attributes("role")).toBe("status");
      expect(notice.attributes("aria-live")).toBe("polite");
    }
  });
});
