// 详情面板测试（SX-015 验收：三组概念分开、推定字样保留、组门禁措辞、
// 证据默认折叠、复制线索不含私人路径、旧快照关闭行动摘要）。
import { describe, expect, it } from "vitest";
import ItemDetail from "../src/components/ItemDetail.vue";
import { rowById } from "../src/app/catalog";
import { createTestStore, analyzeCandidate, mountWithStore } from "./helpers";
import { MockDataSource } from "../src/data/mock";
import { flushPromises } from "@vue/test-utils";
import type { AppStore } from "../src/app/store";

function rowOf(store: AppStore, id: string) {
  const report = store.report.value;
  if (!report) {
    throw new Error("测试前置：应有报告");
  }
  const row = rowById(store.metadata, report, id);
  if (!row) {
    throw new Error(`找不到行：${id}`);
  }
  return row;
}

describe("详情面板", () => {
  it("组级整组检查：措辞、依据与定位分别标注，成员保留原始身份", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const row = rowOf(store, "loc:stella:san-juan-caches");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });

    const text = wrapper.text();
    expect(text).toContain("圣胡安贮藏点组");
    expect(text).toContain("推定遗漏");
    expect(text).toContain("社区关联");
    expect(text).toContain("攻略组");
    expect(text).toContain("可按攻略整组检查");
    expect(text).toContain("在浏览器查看本组 8 个条目的攻略");
    expect(text).toContain("未建立内部 ID 与攻略编号的一对一映射");
    // 组成员保留原始 ID，且不生成 8 个精确点位。
    expect(wrapper.text()).toContain("0x0F28B6EC");
    expect(wrapper.text()).not.toContain("导航到此点");
  });

  it("原始证据默认折叠，展开后显示候选名、偏移、来源与版本", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const row = rowOf(store, "0x0F28B6EC");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    const details = wrapper.get("details");
    expect(details.attributes("open")).toBeUndefined();
    await details.get("summary").trigger("click");
    details.element.setAttribute("open", "");
    await wrapper.vm.$nextTick();
    expect(details.attributes("open")).toBeDefined();
    expect(wrapper.text()).toContain("CollectibleTypeSurvivalCache_SpanishMission_03");
    expect(wrapper.text()).toContain("ev:refdiff:rs:stella-100pct-common");
    expect(wrapper.text()).toContain("元数据版本");
  });

  it("复制可用线索：只含可支持描述与证据精度，不含私人路径或存档内容", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const row = rowOf(store, "0x439B19A6");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    await wrapper.get('[data-testid="copy-lead"]').trigger("click");
    const lead = wrapper.get('[data-testid="lead-text"]').text();
    expect(lead).toContain("状态：推定遗漏");
    expect(lead).toContain("定位：攻略组");
    expect(lead).not.toContain("\\");
    expect(lead).not.toContain("C:");
    expect(lead).not.toContain("演示用户");
  });

  it("非零计数组撤销整组检查措辞（general_reference）", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save204.dat"); // 契约夹具：3/8 非零
    const row = rowOf(store, "loc:stella:san-juan-caches");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    const gate = wrapper.get('[data-testid="detail-gate"]').text();
    expect(gate).toContain("仅作全组参考");
    expect(gate).toContain("撤销");
    expect(wrapper.text()).not.toContain("本组 8 个条目的攻略");
  });

  it("表不完整：关闭行动摘要（unavailable）并说明仅一般阅读", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save206.dat");
    const row = rowOf(store, "loc:stella:san-juan-caches");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    expect(wrapper.get('[data-testid="detail-gate"]').text()).toContain("行动摘要已关闭");
    expect(wrapper.text()).toContain("不给整组遗漏行动结论");
  });

  it("旧快照：过期提示 + 行动摘要关闭 + 仅一般来源阅读", async () => {
    const { store } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    await store.actions.refresh(); // 模拟刷新失败 → stale
    const row = rowOf(store, "loc:stella:san-juan-caches");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    expect(wrapper.text()).toContain("旧快照，未反映本次读取");
    expect(wrapper.get('[data-testid="open-source"]').text()).toContain("阅读相关来源");
    expect(wrapper.get('[data-testid="open-source"]').attributes("title")).toContain("仅作一般来源阅读");
  });

  it("打开来源经数据源校验后交给系统浏览器（不加载 WebView、不附加存档内容）", async () => {
    const { store, source } = await createTestStore();
    await analyzeCandidate(store, "save1.dat");
    const row = rowOf(store, "loc:stella:san-juan-caches");
    const wrapper = mountWithStore(ItemDetail, store, { row, mode: "pane" });
    await wrapper.get('[data-testid="open-source"]').trigger("click");
    await flushPromises();
    // 链接来自元数据来源（不是前端拼接），交给“系统浏览器”而非 WebView。
    const opened = (source as MockDataSource).openedUrls;
    expect(opened).toHaveLength(1);
    expect(opened[0]).toBe(
      "https://tombraiders.net/stella/walks/TR11walk/collectibles/13-san-juan-murals-caches.html",
    );
    expect(store.state.linkAction?.status).toBe("opened");
    expect(store.state.notice?.text).toContain("系统浏览器");
    const openedNotice = wrapper.get('[data-testid="link-opened"]').text();
    expect(openedNotice).toContain("WebView 不加载网页");
    expect(openedNotice).toContain("https://tombraiders.net/");
    expect(openedNotice).not.toContain("演示用户");
  });
});
