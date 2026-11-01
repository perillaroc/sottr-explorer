// 底图留白检测（2026-11-01）：alpha 阈值、邻域容差、越界与不可用回退。
import { describe, expect, it } from "vitest";
import { pointOnMap, readMapMask, type MapMask } from "../src/app/map-mask";

function mask(
  width: number,
  height: number,
  alphaAt: (x: number, y: number) => number,
): MapMask {
  const alpha = new Uint8ClampedArray(width * height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      alpha[y * width + x] = alphaAt(x, y);
    }
  }
  return { width, height, alpha };
}

describe("pointOnMap", () => {
  it("没有遮罩时按「已绘制」处理，保持原有渲染", () => {
    expect(pointOnMap(null, 0.99, 0.01)).toBe(true);
  });

  it("alpha > 8 才算已绘制（与覆盖表口径一致）", () => {
    const drawn = mask(4, 4, (x, y) => (x === 1 && y === 1 ? 9 : 0));
    expect(pointOnMap(drawn, 0.3, 0.3)).toBe(true);
    const faint = mask(4, 4, (x, y) => (x === 1 && y === 1 ? 8 : 0));
    expect(pointOnMap(faint, 0.3, 0.3)).toBe(false);
  });

  it("采样点周围 3×3 取最大值，缩绘后细窄地形不算留白", () => {
    // 中心 (2,2) 透明，只有右下 (3,3) 有内容。
    const thin = mask(4, 4, (x, y) => (x === 3 && y === 3 ? 255 : 0));
    expect(pointOnMap(thin, 0.55, 0.55)).toBe(true);
  });

  it("坐标越界按留白处理", () => {
    const drawn = mask(4, 4, () => 255);
    expect(pointOnMap(drawn, 1.2, 0.5)).toBe(false);
    expect(pointOnMap(drawn, 0.5, -0.1)).toBe(false);
    expect(pointOnMap(drawn, Number.NaN, 0.5)).toBe(false);
  });
});

describe("readMapMask", () => {
  it("没有图片或图片未就绪时返回 null", () => {
    expect(readMapMask(null)).toBeNull();
  });
});
