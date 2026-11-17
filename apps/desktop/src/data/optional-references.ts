// 可选类别的参考集合状态（P2-4）：
// - 可选类别不进入必收 352，也不参与个人完成度（D3 = 仅展示，2026-11-17）。
// - 只有能对上存档区域计数器的类别才显示 `已收/总数`；其余只显示已观察数，
//   并标「参考数未知」，不拿目录条目数冒充分母。
// - 证据来源：
//   * strongboxes：`research/save-inspector/data/region_counter_candidates.json`
//     的 5 个 Strongboxes 计数器（合计 8），
//     `research/game-data/validate_region_counters.py` 逐行对账通过；
//   * explorer_backpacks / archivist_maps / crypt_entrances：存档区域计数器与
//     `reference_category_ids.json` 都没有对应集合，无法验证总数；
//   * gym_training / test_items / croft_manor_extra：不属于主线收集品，
//     没有参考集合，也不建议当成完成度指标。

export type OptionalReferenceStatus = "verified" | "unknown" | "not_applicable";

export interface OptionalReference {
  status: OptionalReferenceStatus;
  /** 已对账的存档区域计数器键；未知/不适用时省略。 */
  counterKey?: string;
  /** 开发者可读的依据；不直接展示给用户。 */
  source: string;
}

export const OPTIONAL_REFERENCES: Record<string, OptionalReference> = {
  strongboxes: {
    status: "verified",
    counterKey: "Strongboxes",
    source: "5 个存档区域计数器（合计 8）；validate_region_counters.py 对账通过",
  },
  explorer_backpacks: {
    status: "unknown",
    source: "无存档区域计数器，也无 100% 参考集合；15 只是目录条目数",
  },
  archivist_maps: {
    status: "unknown",
    source: "无存档区域计数器，也无 100% 参考集合；19 只是目录条目数",
  },
  crypt_entrances: {
    status: "unknown",
    source: "无存档区域计数器，也无 100% 参考集合；9 只是目录条目数",
  },
  gym_training: {
    status: "not_applicable",
    source: "训练场记录；不计入收集品",
  },
  test_items: {
    status: "not_applicable",
    source: "测试项；不计入收集品",
  },
  croft_manor_extra: {
    status: "not_applicable",
    source: "克劳馥庄园额外记录；该区域没有存档计数器",
  },
};

/** 未知类别按 `unknown` 处理，避免把目录条目数当成已验证分母。 */
export function optionalReferenceStatus(id: string): OptionalReferenceStatus {
  return OPTIONAL_REFERENCES[id]?.status ?? "unknown";
}
