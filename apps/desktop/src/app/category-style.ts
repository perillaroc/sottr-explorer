// 类别着色（2026-10-17 详情重设计阶段五）：类别 id → styles.css 里的工具类名。
//
// 颜色本身（`--cat-<id>` 令牌，明暗两套 + 对比度核算）定义在 `src/styles.css`；
// 这里只做映射，不猜色、不兜底分配：未知类别返回空串，界面回退到中性样式。
//
// 这 12 个 id 与 `data/catalog/collectibles_catalog.json` 的 `categories` 一一对应；
// 图鉴新增类别时必须同步这里与 styles.css 的令牌/工具类（`tests/category-style.spec.ts`
// 会核对映射与图鉴类别集合一致）。
const CATEGORY_STYLES: Record<string, { text: string; dot: string }> = {
  archivist_maps: { text: "cat-text-archivist_maps", dot: "cat-dot-archivist_maps" },
  artifacts: { text: "cat-text-artifacts", dot: "cat-dot-artifacts" },
  challenge_tombs: { text: "cat-text-challenge_tombs", dot: "cat-dot-challenge_tombs" },
  croft_manor_extra: { text: "cat-text-croft_manor_extra", dot: "cat-dot-croft_manor_extra" },
  crypt_entrances: { text: "cat-text-crypt_entrances", dot: "cat-dot-crypt_entrances" },
  crypt_treasures: { text: "cat-text-crypt_treasures", dot: "cat-dot-crypt_treasures" },
  explorer_backpacks: { text: "cat-text-explorer_backpacks", dot: "cat-dot-explorer_backpacks" },
  gym_training: { text: "cat-text-gym_training", dot: "cat-dot-gym_training" },
  monoliths: { text: "cat-text-monoliths", dot: "cat-dot-monoliths" },
  strongboxes: { text: "cat-text-strongboxes", dot: "cat-dot-strongboxes" },
  survival_caches: { text: "cat-text-survival_caches", dot: "cat-dot-survival_caches" },
  test_items: { text: "cat-text-test_items", dot: "cat-dot-test_items" },
};

/** 类别文字着色类名（未知类别为空串）。 */
export function categoryClass(categoryId: string | null | undefined): string {
  return (categoryId && CATEGORY_STYLES[categoryId]?.text) || "";
}

/** 类别圆点着色类名（与文字同一色；未知类别为空串）。 */
export function categoryDotClass(categoryId: string | null | undefined): string {
  return (categoryId && CATEGORY_STYLES[categoryId]?.dot) || "";
}

/** 已着色的类别 id（供测试核对与图鉴类别集合一致）。 */
export function styledCategoryIds(): string[] {
  return Object.keys(CATEGORY_STYLES);
}
