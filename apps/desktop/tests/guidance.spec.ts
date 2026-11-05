// 必收指引覆盖（v0.2.0）：地图 + 合集/子关卡文字指引必须覆盖全部 352 条必收条目。
import { describe, expect, it } from "vitest";
import {
  collectionDisplayName,
  guidanceCoverage,
  guidanceScopeForId,
} from "../src/app/guidance";

describe("必收指引覆盖", () => {
  it("352 条必收全部有地图或文字指引", () => {
    const coverage = guidanceCoverage();
    expect(coverage).toEqual({
      required: 352,
      mapped: 243,
      item: 0,
      collection: 32,
      unit: 77,
      uncovered: 0,
    });
    expect(coverage.mapped + coverage.item + coverage.collection + coverage.unit).toBe(
      coverage.required,
    );
  });

  it("按条目区分地图、合集与子关卡", () => {
    // 帕依提提主城（unit 243，已校准底图）。
    expect(guidanceScopeForId("0xF1FAE49A")).toBe("map");
    // Path of the Stars：无坐标，按合集给指引。
    expect(guidanceScopeForId("0xF97006F6")).toBe("collection");
    // 黄金城（unit 1071）：有坐标但无底图。
    expect(guidanceScopeForId("0x659672E5")).toBe("unit");
    expect(guidanceScopeForId("0xDEADBEEF")).toBe("none");
  });

  it("合集显示名优先图鉴的 display 字段，缺失时回退内部名", () => {
    expect(collectionDisplayName("Path_of_the_Stars")).toBe("星之路");
    expect(collectionDisplayName("No_Such_Collection")).toBe("No_Such_Collection");
  });
});
