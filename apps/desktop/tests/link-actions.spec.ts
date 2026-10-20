// 外链动作（2026-10-17 详情重设计阶段三）：按「打开方式」分派打开或仅复制、
// 直链候选来自报告位置线索 × role 为 guide 且有 URL 的元数据来源。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MockDataSource } from "../src/data/mock";
import { externalLinkKey, guideLinkKey } from "../src/app/store";
import type { PlanItem } from "../src/data/atlas";
import { analyzeCandidate, createTestStore } from "./helpers";

/** 圣胡安贮藏点组成员（`data/metadata/reference_sets.json` 的 `stella:san-juan-caches`）。 */
const CACHES_ITEM_ID = "0x0F28B6EC";
const SEARCH_URL = "https://www.bing.com/search?q=%E5%8F%A4%E5%A2%93%E4%B8%BD%E5%BD%B1";

function item(id = CACHES_ITEM_ID): PlanItem {
  return {
    id,
    display_name: "圣胡安贮藏点",
    status: "not_collected",
    is_dlc: false,
    required_for_100: true,
    acquisition_text: "类别规则原文",
    acquisition_source: "category_rule",
    location_hint: "圣胡安 · 贮藏点",
  };
}

/** 分析完成的 store（模拟数据源提供报告与位置线索）。 */
async function storeWithReport(options: { failGuideOpen?: boolean } = {}) {
  const source = new MockDataSource({ delayMs: 0, failGuideOpen: options.failGuideOpen });
  const { store } = await createTestStore({ source });
  await analyzeCandidate(store, "save1.dat");
  return { store, source };
}

describe("store：外链动作", () => {
  const writeText = vi.fn();

  beforeEach(() => {
    writeText.mockReset();
    writeText.mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
  });

  it("打开方式为「打开浏览器」：合成外链交给 openExternalUrl，不写剪贴板", async () => {
    const { store, source } = await storeWithReport();
    await store.actions.openExternalLink(SEARCH_URL, "搜索图文攻略");

    expect(source.openedUrls).toEqual([SEARCH_URL]);
    expect(writeText).not.toHaveBeenCalled();
    expect(store.state.linkAction).toMatchObject({
      key: externalLinkKey(SEARCH_URL),
      kind: "open",
      status: "opened",
      url: SEARCH_URL,
    });
    expect(store.state.notice?.text).toContain("已在系统浏览器打开");
  });

  it("打开方式为「仅复制链接」：只复制地址，不触发打开", async () => {
    const { store, source } = await storeWithReport();
    store.actions.setLinkOpenMode("copy");
    await store.actions.openExternalLink(SEARCH_URL, "搜索图文攻略");

    expect(source.openedUrls).toEqual([]);
    expect(writeText).toHaveBeenCalledWith(SEARCH_URL);
    expect(store.state.linkAction).toMatchObject({
      key: externalLinkKey(SEARCH_URL),
      kind: "copy",
      status: "copied",
      url: SEARCH_URL,
    });
  });

  it("直链：按定位 + 来源解析出元数据地址；「仅复制」时同样不打开", async () => {
    const { store, source } = await storeWithReport();
    const link = store.directLinksForItem(item())[0];
    expect(link).toBeDefined();

    // 默认「打开浏览器」：后端打开并回传地址。
    await store.actions.openGuideLink(link.locationId, link.sourceRef, "攻略页");
    expect(source.openedUrls).toEqual([link.url]);
    expect(store.state.linkAction).toMatchObject({
      key: guideLinkKey(link.locationId, link.sourceRef),
      status: "opened",
    });

    // 切到「仅复制」：不打开，只解析 + 复制。
    store.actions.setLinkOpenMode("copy");
    store.actions.clearLinkAction();
    source.openedUrls.length = 0;
    await store.actions.openGuideLink(link.locationId, link.sourceRef, "攻略页");
    expect(source.openedUrls).toEqual([]);
    expect(writeText).toHaveBeenCalledWith(link.url);
    expect(store.state.linkAction).toMatchObject({ kind: "copy", status: "copied" });
  });

  it("打开失败：保留目标与错误码，便于重试", async () => {
    const { store } = await storeWithReport({ failGuideOpen: true });
    await store.actions.openExternalLink(SEARCH_URL, "搜索图文攻略");
    expect(store.state.linkAction).toMatchObject({
      key: externalLinkKey(SEARCH_URL),
      status: "error",
      error: { code: "LINK_OPEN_FAILED" },
    });
  });

  it("没有报告时直链动作不发起，也不进入错误态", async () => {
    const source = new MockDataSource({ delayMs: 0 });
    const { store } = await createTestStore({ source });
    await store.actions.openGuideLink("loc:x", "src:y", "攻略页");
    expect(store.state.linkAction).toBeNull();
    expect(store.state.notice?.text).toContain("请先分析");
  });
});

describe("store：直链候选", () => {
  it("只为 role 为 guide 的来源产生候选，图文与视频分别保留", async () => {
    const { store } = await storeWithReport();
    const links = store.directLinksForItem(item());

    // 圣胡安贮藏点线索里，stella 是主图文来源；ASL 有 URL 但 role=tool，
    // plan-doc / research-evidence 没有 URL——都不产生候选。
    expect(links[0]).toMatchObject({
      locationId: "loc:stella:san-juan-caches",
      sourceRef: "src:stella-san-juan",
      medium: "text",
    });
    expect(links[0].url).toMatch(/^https:\/\//);
    const refs = links.map((link) => link.sourceRef);
    expect(refs).not.toContain("src:plan-doc");
    expect(refs).not.toContain("src:asl-sottr");
    expect(links.some((link) => link.medium === "video")).toBe(true);
    expect(new Set(links.map((link) => link.url)).size).toBe(links.length);
  });

  it("同一载体的多个 guide 来源按 source_refs 顺序返回，首个在前", async () => {
    const { store } = await storeWithReport();
    const links = store.directLinksForItem(item("0x439B19A6"));

    // 圣胡安石碑组的 source_refs 顺序：stella-san-juan → stella-san-juan-monolith。
    expect(links.slice(0, 2).map((link) => link.sourceRef)).toEqual([
      "src:stella-san-juan",
      "src:stella-san-juan-monolith",
    ]);
    expect(links.slice(0, 2).every((link) => link.medium === "text")).toBe(true);
  });

  it("不在任何线索里的条目没有候选", async () => {
    const { store } = await storeWithReport();
    expect(store.directLinksForItem(item("0xDEADBEEF"))).toEqual([]);
  });

  it("没有报告时没有候选", async () => {
    const { store } = await createTestStore({ source: new MockDataSource({ delayMs: 0 }) });
    expect(store.directLinksForItem(item())).toEqual([]);
  });
});
