// 进度增量与历史（P1-3）：快照摘要、差分与不可比原因的纯函数测试。
//
// 差分只依赖观察集合与图鉴：这里用真实图鉴 ID，报告本体取夹具（campaign）。
import { describe, expect, it } from "vitest";
import { productionAtlas } from "../src/data/atlas";
import { buildCompleteReport } from "../src/data/mockReports";
import {
  appendProgressSnapshot,
  computeProgressDelta,
  defaultBaselineId,
  MAX_PROGRESS_HISTORY,
  progressSnapshotOf,
  type ProgressSnapshot,
} from "../src/app/progress";

const SEED = { compressedSha256: "ab".repeat(32), compressedBytes: 597_590 };
const BASE_REPORT = buildCompleteReport(SEED);

/** 三个真实图鉴 ID：两个圣胡安条目 + 一个帕依提提条目（均为必收）。 */
const A = "0x0F28B6EC";
const B = "0x439B19A6";
const C = "0x851B4915";

interface SnapshotOptions {
  inputRef?: string;
  snapshotId?: string;
  snapshotTime?: string;
  mode?: "campaign" | "unknown" | "profile" | "unsupported";
  inventoryComplete?: boolean;
}

function snapshotOf(ids: string[], options: SnapshotOptions = {}): ProgressSnapshot {
  const report = {
    ...BASE_REPORT,
    context: { ...BASE_REPORT.context, mode: options.mode ?? "campaign" },
    observations: {
      ...BASE_REPORT.observations,
      inventory: {
        ...BASE_REPORT.observations.inventory,
        inventory_complete: options.inventoryComplete ?? true,
      },
    },
  };
  return progressSnapshotOf(
    {
      inputRef: options.inputRef ?? "save1.dat",
      snapshotId: options.snapshotId ?? "snap-1",
      snapshotTime: options.snapshotTime ?? "2026-11-16T12:00:00.000Z",
    },
    report,
    ids,
    productionAtlas,
  );
}

describe("进度快照", () => {
  it("计数来自图鉴计划：主区域与必收总数固定，已收集随观察集合变化", () => {
    const empty = snapshotOf([]);
    expect(empty.mainCollected).toBe(0);
    expect(empty.requiredCollected).toBe(0);
    expect(empty.requiredTotal).toBe(352);
    expect(empty.mainTotal).toBe(370);
    expect(empty.observed).toEqual([]);

    const some = snapshotOf([A, B, A]);
    expect(some.observed).toEqual([A, B].sort());
    expect(some.mainCollected).toBe(2);
    expect(some.requiredCollected).toBe(2);
    // 目录外 ID 不计入条目，只记录数量。
    expect(snapshotOf([A, "0xDEADBEEF"]).observedNotInCatalog).toBe(1);
  });

  it("观察集合顺序不影响结果（确定性）", () => {
    const left = snapshotOf([A, B, C]);
    const right = snapshotOf([C, A, B, A]);
    expect(right).toEqual(left);
  });
});

describe("进度差分", () => {
  it("新增条目 → 总进度、必收与区域增量一致", () => {
    const before = snapshotOf([A, B], { snapshotId: "snap-1" });
    const after = snapshotOf([A, B, C], { snapshotId: "snap-2" });
    const delta = computeProgressDelta(before, after, productionAtlas);
    expect(delta.status).toBe("comparable");
    expect(delta.comparable).toBe(true);
    expect(delta.totalDelta).toBe(1);
    expect(delta.requiredDelta).toBe(1);
    expect(delta.added.map((item) => item.id)).toEqual([C]);
    expect(delta.removed).toEqual([]);
    expect(delta.regions.length).toBeGreaterThan(0);
    expect(delta.regions.every((region) => region.delta === region.after - region.before)).toBe(true);
    expect(delta.regions.some((region) => region.delta === 1)).toBe(true);
  });

  it("不再出现的条目进入 removed，不改变“丢失”措辞之外的状态", () => {
    const before = snapshotOf([A, B, C], { snapshotId: "snap-1" });
    const after = snapshotOf([A], { snapshotId: "snap-2" });
    const delta = computeProgressDelta(before, after, productionAtlas);
    expect(delta.totalDelta).toBe(-2);
    expect(delta.requiredDelta).toBe(-2);
    expect(delta.added).toEqual([]);
    expect(delta.removed.map((item) => item.id)).toEqual([C, B]);
  });

  it("新增/丢失按区域顺序、再按 ID 排序（与输入顺序无关）", () => {
    const before = snapshotOf([], { snapshotId: "snap-1" });
    const after = snapshotOf([C, B, A], { snapshotId: "snap-2" });
    const delta = computeProgressDelta(before, after, productionAtlas);
    // 图鉴区域顺序：帕依提提在圣胡安之前。
    expect(delta.added.map((item) => item.id)).toEqual([C, A, B]);
    expect(delta.added[0].regionSlug).toBe("paititi");
    expect(delta.added[2].regionSlug).toBe("mission_of_san_juan");
  });

  it("同一快照：可比但无增量", () => {
    const before = snapshotOf([A, B], { snapshotId: "snap-1" });
    const after = snapshotOf([B, A], { snapshotId: "snap-1" });
    const delta = computeProgressDelta(before, after, productionAtlas);
    expect(delta.comparable).toBe(true);
    expect(delta.totalDelta).toBe(0);
    expect(delta.added).toEqual([]);
    expect(delta.removed).toEqual([]);
    expect(delta.regions).toEqual([]);
  });

  it("不同输入文件：明确不可比，不推断新增/丢失", () => {
    const before = snapshotOf([A], { snapshotId: "snap-1", inputRef: "save1.dat" });
    const after = snapshotOf([A, B], { snapshotId: "snap-2", inputRef: "save202.dat" });
    const delta = computeProgressDelta(before, after, productionAtlas);
    expect(delta.status).toBe("different_save");
    expect(delta.comparable).toBe(false);
    expect(delta.totalDelta).toBe(0);
    expect(delta.added).toEqual([]);
    expect(delta.removed).toEqual([]);
    expect(delta.regions).toEqual([]);
    expect(delta.before.inputRef).toBe("save1.dat");
    expect(delta.after.inputRef).toBe("save202.dat");
  });

  it("非战役上下文：明确不可比", () => {
    const before = snapshotOf([A], { snapshotId: "snap-1" });
    const after = snapshotOf([A, B], { snapshotId: "snap-2", mode: "unknown" });
    expect(computeProgressDelta(before, after, productionAtlas).status).toBe("not_campaign");
  });

  it("物品表不完整：明确不可比，不推断新增/丢失", () => {
    const before = snapshotOf([A], { snapshotId: "snap-1" });
    const after = snapshotOf([A, B], { snapshotId: "snap-2", inventoryComplete: false });
    const delta = computeProgressDelta(before, after, productionAtlas);
    expect(delta.status).toBe("inventory_incomplete");
    expect(delta.comparable).toBe(false);
    expect(delta.added).toEqual([]);
    expect(delta.removed).toEqual([]);
  });
});

describe("会话历史", () => {
  it("同一快照 ID 不重复记录；超出上限丢弃最旧", () => {
    const first = snapshotOf([A], { snapshotId: "snap-1" });
    let history = appendProgressSnapshot([], first);
    expect(history).toHaveLength(1);
    expect(appendProgressSnapshot(history, { ...first })).toBe(history);

    for (let index = 2; index <= MAX_PROGRESS_HISTORY + 2; index += 1) {
      history = appendProgressSnapshot(history, {
        ...first,
        snapshotId: `snap-${index}`,
      });
    }
    expect(history).toHaveLength(MAX_PROGRESS_HISTORY);
    expect(history[history.length - 1].snapshotId).toBe(`snap-${MAX_PROGRESS_HISTORY + 2}`);
  });

  it("默认基线是最近一次之前的那次分析", () => {
    expect(defaultBaselineId([])).toBeNull();
    const a = snapshotOf([A], { snapshotId: "snap-1" });
    const b = snapshotOf([A, B], { snapshotId: "snap-2" });
    expect(defaultBaselineId([a])).toBeNull();
    expect(defaultBaselineId([a, b])).toBe("snap-1");
  });
});
