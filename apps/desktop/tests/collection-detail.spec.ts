// 收集品详情测试（2026-10-17 重设计）：头部、如何获取、外部参考（直链 + 搜索）、相对位置折叠。
// 状态判定说明与原始证据移到诊断页测试（tests/diagnostics.spec.ts）。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { mount } from "@vue/test-utils";
import CollectionDetail from "../src/components/CollectionDetail.vue";
import { storeKey } from "../src/app/inject";
import { createTestStore, analyzeCandidate } from "./helpers";
import { MockDataSource } from "../src/data/mock";

const STELLA_SAN_JUAN_URL =
  "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html";
/** 圣胡安贮藏点组成员（位置线索带来源地址）。 */
const CACHES_ITEM = "0x0F28B6EC";
/** 帕依提提主城的遗物（有坐标、无直链来源）。 */
const PAITITI_ITEM = "0xF1FAE49A";

const writeText = vi.fn();

async function setup(source = new MockDataSource({ delayMs: 0 })) {
  const { store } = await createTestStore({ source });
  await analyzeCandidate(store, "save1.dat");
  return { store, source };
}

async function detailFor(store: Awaited<ReturnType<typeof createTestStore>>["store"], id: string) {
  store.actions.openRow(id);
  const entry = store.selectedEntry.value;
  if (!entry) {
    throw new Error(`找不到计划条目：${id}`);
  }
  return mount(CollectionDetail, {
    props: { region: entry.region, group: entry.group, item: entry.item, mode: "pane" },
    global: { provide: { [storeKey]: store } },
    attachTo: document.body,
  });
}

describe("收集品详情", () => {
  beforeEach(() => {
    writeText.mockReset();
    writeText.mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
  });

  it("故事条目显示官方中文名与英文名、状态与所在区域/子关卡", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, "0xF97006F6");
    const text = wrapper.text();
    expect(text).toContain("隐秘之城的谜团");
    expect(text).toContain("HIDDEN CITY RIDDLE");
    expect(text).toContain("0xF97006F6");
    expect(wrapper.get('[data-testid="detail-heading"]').text()).toContain("隐秘之城的谜团");
    expect(text).toContain("特殊 / 跨区域");
  });

  it("「如何获取」一段含区域 · 子关卡、类别规则、相对位置与同子关卡剩余", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, PAITITI_ITEM);
    const body = wrapper.get('[data-testid="detail-acquisition"]').text();
    expect(body).toContain("帕依提提");
    expect(body).toContain("在关卡内拾取");
    expect(body).toContain("关卡内相对位置约 (");
    expect(body).toContain("项未收集");
    expect(wrapper.text()).toContain("类别通用说明（人工整理");
  });

  it("非故事收集品使用类别通用获取方式", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, CACHES_ITEM);
    expect(wrapper.get('[data-testid="detail-acquisition"]').text()).toContain("箱子或背包");
    expect(wrapper.text()).toContain("生存贮藏点");
  });

  it("相对位置散点默认折叠，展开区保留未校准与「不是导航点位」标注", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, PAITITI_ITEM);
    const section = wrapper.get('[data-testid="position-section"]');
    expect(section.element.tagName).toBe("DETAILS");
    expect(section.attributes("open")).toBeUndefined();
    expect(wrapper.find('[data-testid="position-scatter"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("未校准");
    expect(wrapper.text()).toContain("不是导航点位");
  });

  it("详情只剩四段：头部、如何获取、外部参考、相对位置（折叠）", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, PAITITI_ITEM);
    const text = wrapper.text();
    expect(text).toContain("如何获取");
    expect(text).toContain("外部参考");
    expect(text).toContain("同一子关卡内的相对位置");
    // 移出详情的两节不再出现。
    expect(text).not.toContain("为什么是这个状态");
    expect(text).not.toContain("原始证据与数据集信息");
    expect(wrapper.find('[data-testid="detail-status-reason"]').exists()).toBe(false);
  });

  it("共用 ID 的挑战古墓列出全部名称且不拆分判定", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, "0xC9D62DEE");
    expect(wrapper.text()).toContain("按 ID 计数，不拆分判定");
  });

  it("外部参考：直链按钮只在该条目的位置线索有来源地址时出现", async () => {
    const { store } = await setup();
    const withLinks = await detailFor(store, CACHES_ITEM);
    expect(withLinks.find('[data-testid="link-guide"]').exists()).toBe(true);
    // 现有来源没有视频地址：视频直链不渲染（不置灰）。
    expect(withLinks.find('[data-testid="link-video"]').exists()).toBe(false);
    expect(withLinks.find('[data-testid="link-search-text"]').exists()).toBe(true);
    expect(withLinks.find('[data-testid="link-search-video"]').exists()).toBe(true);

    const withoutLinks = await detailFor(store, PAITITI_ITEM);
    expect(withoutLinks.find('[data-testid="link-guide"]').exists()).toBe(false);
    expect(withoutLinks.find('[data-testid="link-video"]').exists()).toBe(false);
  });

  it("点击「攻略页」打开元数据来源地址并显示已打开", async () => {
    const { store, source } = await setup();
    const wrapper = await detailFor(store, CACHES_ITEM);
    await wrapper.get('[data-testid="link-guide"]').trigger("click");
    await flushPromises();
    expect(source.openedUrls).toEqual([STELLA_SAN_JUAN_URL]);
    expect(wrapper.get('[data-testid="link-status"]').text()).toContain("已在浏览器打开");
  });

  it("搜索链接随搜索引擎偏好切换（Bing / Google / 百度）", async () => {
    const { store, source } = await setup();
    const wrapper = await detailFor(store, PAITITI_ITEM);
    const click = async () => {
      await wrapper.get('[data-testid="link-search-text"]').trigger("click");
      await flushPromises();
    };

    await click();
    expect(source.openedUrls[0]).toMatch(/^https:\/\/www\.bing\.com\/search\?q=/);
    expect(decodeURIComponent(source.openedUrls[0])).toContain("庆典火把");

    store.actions.setSearchEngine("google");
    await click();
    expect(source.openedUrls[1]).toMatch(/^https:\/\/www\.google\.com\/search\?q=/);

    store.actions.setSearchEngine("baidu");
    await click();
    expect(source.openedUrls[2]).toMatch(/^https:\/\/www\.baidu\.com\/s\?wd=/);
  });

  it("「仅复制链接」时搜索按钮走剪贴板而不打开浏览器", async () => {
    const { store, source } = await setup();
    store.actions.setLinkOpenMode("copy");
    const wrapper = await detailFor(store, PAITITI_ITEM);
    await wrapper.get('[data-testid="link-search-video"]').trigger("click");
    await flushPromises();

    expect(source.openedUrls).toEqual([]);
    expect(writeText).toHaveBeenCalledTimes(1);
    const copied = String(writeText.mock.calls[0][0]);
    expect(copied).toMatch(/^https:\/\/www\.bing\.com\/search\?q=/);
    expect(decodeURIComponent(copied)).toContain("视频攻略");
    expect(wrapper.get('[data-testid="link-status"]').text()).toContain("已复制链接");
  });

  it("打开失败时显示错误并提供重试入口", async () => {
    const { store } = await setup(new MockDataSource({ delayMs: 0, failGuideOpen: true }));
    const wrapper = await detailFor(store, PAITITI_ITEM);
    await wrapper.get('[data-testid="link-search-text"]').trigger("click");
    await flushPromises();
    const status = wrapper.get('[data-testid="link-status"]').text();
    expect(status).toContain("失败");
    expect(status).toContain("LINK_OPEN_FAILED");
    expect(wrapper.find('[data-testid="link-retry"]').exists()).toBe(true);
  });

  it("类别徽标按类别着色，子类别另成徽标（与主清单同一套颜色）", async () => {
    const { store } = await setup();
    const nonStory = await detailFor(store, CACHES_ITEM);
    const badges = nonStory.findAll(".cat-text-survival_caches");
    expect(badges.length).toBe(1);
    expect(badges[0].text()).toContain("生存贮藏点");

    const story = await detailFor(store, PAITITI_ITEM);
    expect(story.findAll(".cat-text-artifacts").length).toBe(1);
    expect(story.text()).toContain("Relic（遗物）");
  });

  it("复制线索只含名称/状态/获取方式/区域，不含私人路径", async () => {
    const { store } = await setup();
    const wrapper = await detailFor(store, PAITITI_ITEM);
    await wrapper.get('[data-testid="copy-lead"]').trigger("click");
    await wrapper.vm.$nextTick();
    const lead = wrapper.get('[data-testid="lead-text"]').text();
    expect(lead).toContain("庆典火把");
    expect(lead).toContain("状态：");
    expect(lead).toContain("获取：");
    expect(lead).toContain("区域：");
    expect(lead).not.toContain("C:");
  });
});
