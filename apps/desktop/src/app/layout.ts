// 页面布局辅助（2026-11-05 界面美化阶段三）：把 §3.3 / §4.5 的宽度与栅格口径
// 收敛到一处，避免每个页面各写一套断点。断点来自 store.layout，不使用视口 media query。
import type { LayoutMode } from "./store";

/** 页面水平内边距：宽 / 中 24px，紧凑 20px，窄窗 16px。 */
export function pagePaddingClass(layout: LayoutMode): string {
  switch (layout) {
    case "narrow":
      return "px-4";
    case "compact":
      return "px-5";
    default:
      return "px-6";
  }
}

/** 区域索引列数：宽 / 中两列，紧凑 / 窄窗单列。 */
export function regionGridClass(layout: LayoutMode): string {
  return layout === "wide" || layout === "medium" ? "grid-cols-2" : "grid-cols-1";
}

/** 概览摘要工作面：宽 / 中三列横排，紧凑 / 窄窗单列纵排。 */
export function summaryGridClass(layout: LayoutMode): string {
  return layout === "wide" || layout === "medium"
    ? "grid-cols-3 divide-x divide-line"
    : "grid-cols-1 divide-y divide-line";
}
