// 底图留白检测（2026-11-01）：判断归一化坐标是否落在底图真正绘制出来的区域。
//
// 背景：图鉴 `position` 是每单位归一化的取景框坐标，而底图素材的取景框里有一部分是
// 透明留白（多为地下、隧道或地图边缘）。全数据集 294 条带位置条目中有 14 条落在留白区，
// 按变换绘制时看起来像「跑出了地图」。数据集口径见 `data/map/README.md`。
//
// 判定口径与覆盖表一致：把底图缩绘到离屏画布后采样 alpha，`alpha > 8` 视为已绘制。
// 采样失败（环境不支持画布、图片跨域污染）时返回 null，调用方按「已绘制」处理，
// 即保持原有渲染，不因检测不到而改动点位。
export interface MapMask {
  width: number;
  height: number;
  /** 逐像素 alpha，按行优先存储。 */
  alpha: Uint8ClampedArray;
}

/** 采样用的最长边；远小于原图即可，只在点位附近判断有没有内容。 */
const MAX_SIDE = 1024;
/** 与底图处理脚本一致的不透明阈值（alpha > 8）。 */
const ALPHA_MIN = 8;

function normalize(value: number, size: number): number {
  const pixel = Math.floor(value * size);
  return Math.min(size - 1, Math.max(0, pixel));
}

/** 把底图缩绘到离屏画布并读出 alpha；不可用时返回 null。 */
export function readMapMask(
  image: HTMLImageElement | null,
  maxSide: number = MAX_SIDE,
): MapMask | null {
  const sourceWidth = image?.naturalWidth ?? 0;
  const sourceHeight = image?.naturalHeight ?? 0;
  if (!image || sourceWidth <= 0 || sourceHeight <= 0) {
    return null;
  }
  const scale = Math.min(1, maxSide / Math.max(sourceWidth, sourceHeight));
  const width = Math.max(1, Math.round(sourceWidth * scale));
  const height = Math.max(1, Math.round(sourceHeight * scale));
  try {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) {
      return null;
    }
    context.drawImage(image, 0, 0, width, height);
    const data = context.getImageData(0, 0, width, height).data;
    const alpha = new Uint8ClampedArray(width * height);
    for (let index = 0; index < alpha.length; index += 1) {
      alpha[index] = data[index * 4 + 3];
    }
    return { width, height, alpha };
  } catch {
    // 画布被污染（SecurityError）或环境没有 2D 上下文：放弃检测。
    return null;
  }
}

/**
 * 归一化坐标是否落在已绘制区域。
 *
 * 没有遮罩（无法检测）时返回 true，保持原有渲染；坐标越界时返回 false。
 * 采样点周围 3×3 取最大值，避免缩绘后细窄地形被判成留白。
 */
export function pointOnMap(mask: MapMask | null, x: number, y: number): boolean {
  if (!mask) {
    return true;
  }
  if (!Number.isFinite(x) || !Number.isFinite(y) || x < 0 || x > 1 || y < 0 || y > 1) {
    return false;
  }
  const centerX = normalize(x, mask.width);
  const centerY = normalize(y, mask.height);
  for (let offsetY = -1; offsetY <= 1; offsetY += 1) {
    const py = centerY + offsetY;
    if (py < 0 || py >= mask.height) {
      continue;
    }
    for (let offsetX = -1; offsetX <= 1; offsetX += 1) {
      const px = centerX + offsetX;
      if (px < 0 || px >= mask.width) {
        continue;
      }
      if (mask.alpha[py * mask.width + px] > ALPHA_MIN) {
        return true;
      }
    }
  }
  return false;
}
