// 设计令牌对比度（2026-11-05 界面美化阶段一）：直接读取 styles.css，
// 核对明暗主题在最不利背景上的 WCAG 对比度，并确认新增令牌已接入 Tailwind 映射。
// 数值记录见 doc/2026-11-05-界面美化/对比度记录.md。
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync("src/styles.css", "utf8");

type Tokens = Record<string, string>;

function tokenBlock(selector: string): Tokens {
  const start = styles.indexOf(selector);
  if (start < 0) {
    throw new Error(`未找到样式块：${selector}`);
  }
  const open = styles.indexOf("{", start);
  const close = styles.indexOf("}", open);
  if (open < 0 || close < 0) {
    throw new Error(`样式块不完整：${selector}`);
  }
  const body = styles.slice(open + 1, close);
  const tokens: Tokens = {};
  for (const match of body.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6});/g)) {
    tokens[match[1]] = match[2];
  }
  return tokens;
}

function rgb(hex: string): [number, number, number] {
  const value = hex.replace("#", "");
  return [0, 2, 4].map((index) => Number.parseInt(value.slice(index, index + 2), 16)) as [
    number,
    number,
    number,
  ];
}

function luminance(hex: string): number {
  const [r, g, b] = rgb(hex).map((channel) => {
    const c = channel / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

function expectContrast(tokens: Tokens, fg: string, bg: string, minimum = 4.5): void {
  const fgColor = tokens[fg];
  const bgColor = tokens[bg];
  expect(fgColor, `缺少令牌 --${fg}`).toMatch(/^#[0-9a-fA-F]{6}$/);
  expect(bgColor, `缺少令牌 --${bg}`).toMatch(/^#[0-9a-fA-F]{6}$/);
  const ratio = contrast(fgColor, bgColor);
  expect(ratio, `--${fg} on --${bg} = ${ratio.toFixed(2)}:1`).toBeGreaterThanOrEqual(minimum);
}

const themes = [
  { name: "浅色", tokens: tokenBlock(':root,') },
  { name: "深色", tokens: tokenBlock('[data-theme="dark"]') },
];

describe("设计令牌对比度", () => {
  for (const theme of themes) {
    describe(theme.name, () => {
      it("正文、辅助、强调与旧金文字在 canvas / surface / raised 上 ≥4.5:1", () => {
        for (const background of ["canvas", "surface", "raised"]) {
          for (const foreground of ["text", "muted", "accent", "accent-strong", "gold"]) {
            expectContrast(theme.tokens, foreground, background);
          }
        }
      });

      it("主按钮文字在 accent 与 accent-hover 上 ≥4.5:1", () => {
        expectContrast(theme.tokens, "accent-contrast", "accent");
        expectContrast(theme.tokens, "accent-contrast", "accent-hover");
      });

      it("强调文字在 accent-bg 上、旧金文字在 gold-bg 上 ≥4.5:1", () => {
        expectContrast(theme.tokens, "accent-strong", "accent-bg");
        expectContrast(theme.tokens, "gold", "gold-bg");
      });

      it("危险按钮文字在默认与 hover 填充上 ≥4.5:1", () => {
        expectContrast(theme.tokens, "danger-contrast", "danger");
        expectContrast(theme.tokens, "danger-contrast", "danger-hover");
      });

      it("状态色在各自背景与 raised 上 ≥4.5:1", () => {
        for (const status of ["info", "caution", "danger", "success"]) {
          expectContrast(theme.tokens, status, `${status}-bg`);
          expectContrast(theme.tokens, status, "raised");
        }
      });

      it("12 个类别色在 canvas / surface / raised / accent-bg 上 ≥4.5:1", () => {
        const categories = Object.keys(theme.tokens).filter((name) => name.startsWith("cat-"));
        expect(categories).toHaveLength(12);
        for (const category of categories) {
          for (const background of ["canvas", "surface", "raised", "accent-bg"]) {
            expectContrast(theme.tokens, category, background);
          }
        }
      });
    });
  }

  it("新增令牌已接入 Tailwind 映射", () => {
    for (const mapping of [
      "--color-accent-hover: var(--accent-hover);",
      "--color-gold: var(--gold);",
      "--color-gold-fill: var(--gold-fill);",
      "--color-gold-bg: var(--gold-bg);",
      "--color-map-stage: var(--map-stage);",
      "--color-danger-contrast: var(--danger-contrast);",
      "--radius-control: var(--ui-radius-control);",
      "--radius-section: var(--ui-radius-section);",
      "--radius-overlay: var(--ui-radius-overlay);",
    ]) {
      expect(styles).toContain(mapping);
    }
  });
});
