// 类别图标（2026-10-31 地图查询视图）：图鉴类别 id → Lucide 图标组件。
//
// 这 12 个 id 与 `data/catalog/collectibles_catalog.json` 的 `categories` 一一对应；
// 图鉴新增类别时必须同步这里（`tests/category-icon.spec.ts` 会核对映射与图鉴类别集合一致）。
// 未知类别回退 `MapPin`，不猜、不随机分配：形状只做辅助辨认，类别名始终以文字出现。
import type { Component } from "vue";
import {
  Backpack,
  Briefcase,
  Coins,
  Columns3,
  DoorOpen,
  Dumbbell,
  FlaskConical,
  Gem,
  Home,
  Landmark,
  Map,
  MapPin,
  Package,
} from "@lucide/vue";

const CATEGORY_ICONS: Record<string, Component> = {
  archivist_maps: Map,
  artifacts: Gem,
  challenge_tombs: Landmark,
  croft_manor_extra: Home,
  crypt_entrances: DoorOpen,
  crypt_treasures: Coins,
  explorer_backpacks: Backpack,
  gym_training: Dumbbell,
  monoliths: Columns3,
  strongboxes: Briefcase,
  survival_caches: Package,
  test_items: FlaskConical,
};

/** 未知类别的回退图标。 */
export const fallbackCategoryIcon: Component = MapPin;

/** 类别 id → 图标组件（未知类别回退 `MapPin`）。 */
export function categoryIcon(categoryId: string | null | undefined): Component {
  return (categoryId && CATEGORY_ICONS[categoryId]) || fallbackCategoryIcon;
}

/** 已映射的类别 id（供测试核对与图鉴类别集合一致）。 */
export function iconCategoryIds(): string[] {
  return Object.keys(CATEGORY_ICONS);
}
