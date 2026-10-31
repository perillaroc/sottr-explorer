// 详情弹出窗口（2026-10-31 地图查询与详情增强·阶段三）：
// payload 构建（无路径/无证据、stale）、版本校验、弹窗渲染与切换、外链动作、
// 缺 payload、inline 模式、打开失败时的应用内兜底。
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import App from "../src/App.vue";
import DetailWindow from "../src/components/DetailWindow.vue";
import ItemDetailView from "../src/components/ItemDetailView.vue";
import CollectionDetail from "../src/components/CollectionDetail.vue";
import { storeKey } from "../src/app/inject";
import {
  DETAIL_PAYLOAD_EVENT,
  DETAIL_WINDOW_STORAGE_KEY,
  DetailPayloadError,
  buildDetailWindowPayload,
  parseDetailWindowPayload,
  resetDetailWindowHandle,
  subscribeDetailWindowUpdates,
  type DetailWindowPayload,
  type DetailWindowSource,
} from "../src/app/detail-window";
import { analyzeCandidate, createTestStore, mountWithStore } from "./helpers";
import type { AppStore } from "../src/app/store";

/** 帕依提提主城的遗物（同子关卡条目多，用于 payload 体积/清单断言）。 */
const PAITITI_ITEM = "0xF1FAE49A";
/** 特殊区域条目（无子关卡）：payload 只含该条目、无底图。 */
const NO_UNIT_ITEM = "0xF97006F6";

async function storeWithReport(): Promise<AppStore> {
  const { store } = await createTestStore();
  await analyzeCandidate(store, "save1.dat");
  return store;
}

function sourceOf(store: AppStore): DetailWindowSource {
  return {
    plan: store.plan.value,
    atlas: store.atlas,
    slotName: store.currentSlot.value?.name ?? "",
    snapshotTime: store.currentSlot.value?.result?.snapshot_time ?? null,
    stale: store.isStale.value,
    searchEngine: store.state.searchEngine,
    linkOpenMode: store.state.linkOpenMode,
    directLinksForItem: (item) => store.directLinksForItem(item),
  };
}

async function payloadFor(id: string): Promise<{ store: AppStore; payload: DetailWindowPayload }> {
  const store = await storeWithReport();
  store.actions.openRow(id);
  const entry = store.selectedEntry.value;
  if (!entry) {
    throw new Error(`找不到条目 ${id}`);
  }
  return { store, payload: buildDetailWindowPayload(sourceOf(store), entry) };
}

describe("payload 构建", () => {
  it("只含展示所需字段：文件名而非完整路径、无证据/诊断数据", async () => {
    const { payload } = await payloadFor(PAITITI_ITEM);
    expect(payload.version).toBe(1);
    expect(payload.selectedId).toBe(PAITITI_ITEM);
    expect(payload.slotName).toBe("save1.dat");
    expect(payload.regionName).toBe("帕依提提");
    expect(payload.unitLabel.length).toBeGreaterThan(0);
    expect(payload.basemap).not.toBeNull();
    // 同子关卡全部条目（含选中项）。
    expect(payload.items.length).toBeGreaterThan(1);
    expect(payload.items.some((item) => item.id === PAITITI_ITEM)).toBe(true);

    const item = payload.items.find((entry) => entry.id === PAITITI_ITEM)!;
    expect(item.searchTextUrl).toMatch(/^https:\/\/www\.bing\.com\/search\?q=/);
    expect(item.searchVideoUrl).toMatch(/^https:\/\/www\.bing\.com\/search\?q=/);
    expect(item.categoryLabel.length).toBeGreaterThan(0);

    // 隐私边界：不含完整路径、诊断/证据字段（底图的 map_path 是游戏资源名，不是用户路径）。
    const json = JSON.stringify(payload);
    expect(json).not.toContain("C:");
    expect(json).not.toContain("Users");
    expect(json).not.toContain("diagnostics");
    expect(json).not.toContain("evidence");
  });

  it("无子关卡条目：只含自身、无底图、子关卡显示占位符", async () => {
    const { payload } = await payloadFor(NO_UNIT_ITEM);
    expect(payload.items.length).toBe(1);
    expect(payload.items[0].id).toBe(NO_UNIT_ITEM);
    expect(payload.basemap).toBeNull();
    expect(payload.unitLabel).toBe("—");
  });

  it("stale 标记随报告状态进入 payload", async () => {
    const { store, payload } = await payloadFor(PAITITI_ITEM);
    const stale = buildDetailWindowPayload({ ...sourceOf(store), stale: true }, store.selectedEntry.value!);
    expect(payload.stale).toBe(false);
    expect(stale.stale).toBe(true);
  });

  it("浏览器模式只接受同源且来自 opener 的 postMessage 更新", async () => {
    const { payload } = await payloadFor(PAITITI_ITEM);
    Object.defineProperty(window, "opener", { value: window, configurable: true });
    const callback = vi.fn();
    const unsubscribe = await subscribeDetailWindowUpdates(callback);

    window.dispatchEvent(
      new MessageEvent("message", {
        data: { type: DETAIL_PAYLOAD_EVENT, payload },
        origin: location.origin,
        source: window,
      }),
    );
    expect(callback).toHaveBeenCalledTimes(1);

    // 非本源的更新一律忽略。
    window.dispatchEvent(
      new MessageEvent("message", {
        data: { type: DETAIL_PAYLOAD_EVENT, payload },
        origin: "https://evil.example",
        source: window,
      }),
    );
    expect(callback).toHaveBeenCalledTimes(1);
    unsubscribe();
  });

  it("解析与版本校验：缺失 / 损坏 / 版本不符分别报错", async () => {
    const { payload } = await payloadFor(PAITITI_ITEM);
    expect(parseDetailWindowPayload(JSON.stringify(payload)).selectedId).toBe(PAITITI_ITEM);

    const expectKind = (raw: string | null, kind: string) => {
      try {
        parseDetailWindowPayload(raw);
        throw new Error("应当失败");
      } catch (error) {
        expect(error).toBeInstanceOf(DetailPayloadError);
        expect((error as DetailPayloadError).kind).toBe(kind);
      }
    };
    expectKind(null, "missing");
    expectKind("{ 不是 JSON", "invalid");
    expectKind(JSON.stringify({ ...payload, version: 2 }), "version");
  });
});

describe("DetailWindow 渲染", () => {
  let payload: DetailWindowPayload;

  beforeEach(async () => {
    ({ payload } = await payloadFor(PAITITI_ITEM));
  });

  it("窗口模式从存储读取 payload：显示条目、同子关卡清单可切换", async () => {
    sessionStorage.setItem(DETAIL_WINDOW_STORAGE_KEY, JSON.stringify(payload));
    const wrapper = mount(DetailWindow, { props: { mode: "window" }, attachTo: document.body });
    await flushPromises();

    expect(wrapper.find('[data-testid="detail-window"]').exists()).toBe(true);
    const heading = wrapper.get('[data-testid="detail-heading"]').text();
    expect(heading).toContain(payload.items.find((item) => item.id === payload.selectedId)!.displayName);
    expect(wrapper.find('[data-testid="detail-window-list"]').exists()).toBe(true);
    // 弹窗不显示「在新窗口中打开」。
    expect(wrapper.find('[data-testid="detail-popout"]').exists()).toBe(false);

    const other = payload.items.find((item) => item.id !== payload.selectedId)!;
    await wrapper.get(`[data-testid="detail-window-item-${other.id}"]`).trigger("click");
    await flushPromises();
    expect(wrapper.get('[data-testid="detail-heading"]').text()).toContain(other.displayName);
    sessionStorage.removeItem(DETAIL_WINDOW_STORAGE_KEY);
  });

  it("缺 payload 与版本不符显示明确状态", async () => {
    sessionStorage.removeItem(DETAIL_WINDOW_STORAGE_KEY);
    const missing = mount(DetailWindow, { props: { mode: "window" }, attachTo: document.body });
    await flushPromises();
    expect(missing.get('[data-testid="detail-window-error"]').text()).toContain("没有可显示的详情");

    sessionStorage.setItem(DETAIL_WINDOW_STORAGE_KEY, JSON.stringify({ ...payload, version: 2 }));
    const mismatch = mount(DetailWindow, { props: { mode: "window" }, attachTo: document.body });
    await flushPromises();
    expect(mismatch.get('[data-testid="detail-window-error"]').text()).toContain("版本不兼容");
    sessionStorage.removeItem(DETAIL_WINDOW_STORAGE_KEY);
  });

  it("inline 模式直接使用传入 payload（兜底浮层）", async () => {
    const wrapper = mount(DetailWindow, {
      props: { mode: "inline", payload },
      attachTo: document.body,
    });
    await flushPromises();
    expect(wrapper.get('[data-testid="detail-heading"]').text()).toContain(
      payload.items.find((item) => item.id === payload.selectedId)!.displayName,
    );
    // inline 模式没有自己的标题栏与关闭按钮（由宿主对话框提供）。
    expect(wrapper.find('[data-testid="detail-window-close"]').exists()).toBe(false);
  });

  it("「仅复制链接」设置随 payload 生效：只写剪贴板、不打开浏览器", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    const open = vi.spyOn(window, "open");
    const wrapper = mount(DetailWindow, {
      props: { mode: "inline", payload: { ...payload, linkOpenMode: "copy" } },
      attachTo: document.body,
    });
    await flushPromises();
    const searchButton = wrapper
      .get('[data-testid="external-links"]')
      .findAll("button")
      .find((button) => button.text().includes("搜索视频"));
    await searchButton!.trigger("click");
    await flushPromises();

    expect(open).not.toHaveBeenCalled();
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(wrapper.get('[data-testid="link-status"]').text()).toContain("已复制链接");
  });

  it("外链按钮在弹窗内打开地址并显示已打开", async () => {
    const open = vi.spyOn(window, "open").mockReturnValue({
      closed: false,
      focus: vi.fn(),
      postMessage: vi.fn(),
    } as unknown as Window);

    const wrapper = mount(DetailWindow, {
      props: { mode: "inline", payload },
      attachTo: document.body,
    });
    await flushPromises();
    const searchButton = wrapper
      .get('[data-testid="external-links"]')
      .findAll("button")
      .find((button) => button.text().includes("搜索图文攻略"));
    expect(searchButton).toBeDefined();
    await searchButton!.trigger("click");
    await flushPromises();

    const item = payload.items.find((entry) => entry.id === payload.selectedId)!;
    expect(open).toHaveBeenCalledWith(item.searchTextUrl, "_blank");
    expect(wrapper.get('[data-testid="link-status"]').text()).toContain("已在浏览器打开");
  });
});

describe("应用内兜底", () => {
  beforeEach(() => {
    resetDetailWindowHandle();
  });

  it("window.open 被拦截时写入兜底浮层并提示；成功时清空兜底", async () => {
    const store = await storeWithReport();
    store.actions.openRow(PAITITI_ITEM);

    const open = vi.spyOn(window, "open").mockReturnValue(null);
    await store.actions.openDetailWindow();
    expect(store.state.detailWindowFallback.open).toBe(true);
    expect(store.state.detailWindowFallback.payload?.selectedId).toBe(PAITITI_ITEM);
    expect(store.state.notice?.text).toContain("已在应用内显示详情");

    open.mockReturnValue({
      closed: false,
      focus: vi.fn(),
      postMessage: vi.fn(),
    } as unknown as Window);
    await store.actions.openDetailWindow();
    expect(store.state.detailWindowFallback.open).toBe(false);

    store.actions.closeDetailWindowFallback();
    expect(store.state.detailWindowFallback.payload).toBeNull();
  });

  it("没有选中条目时不构建 payload", async () => {
    const store = await storeWithReport();
    store.actions.closeDetail();
    await store.actions.openDetailWindow();
    expect(store.state.detailWindowFallback.open).toBe(false);
    expect(store.state.notice?.text).toContain("请先选择一个条目");
  });

  it("兜底时主窗口渲染应用内详情对话框", async () => {
    const store = await storeWithReport();
    store.actions.openRow(PAITITI_ITEM);
    vi.spyOn(window, "open").mockReturnValue(null);
    await store.actions.openDetailWindow();
    expect(store.state.detailWindowFallback.open).toBe(true);

    mountWithStore(App, store, { store, layoutWidth: 1440 });
    await flushPromises();
    const dialog = document.querySelector('[role="dialog"]') as HTMLElement | null;
    expect(dialog).not.toBeNull();
    expect(dialog?.querySelector('[data-testid="detail-window"]')).not.toBeNull();
    expect(dialog?.textContent).toContain(store.selectedEntry.value?.item.display_name ?? "");
  });
});

describe("ItemDetailView 复用", () => {
  it("CollectionDetail 渲染纯展示层，并支持隐藏相对位置与弹出按钮", async () => {
    const store = await storeWithReport();
    store.actions.openRow(PAITITI_ITEM);
    const entry = store.selectedEntry.value!;
    const wrapper = mount(CollectionDetail, {
      props: {
        region: entry.region,
        group: entry.group,
        item: entry.item,
        mode: "pane",
        showPosition: false,
      },
      global: { provide: { [storeKey]: store } },
      attachTo: document.body,
    });
    await flushPromises();
    expect(wrapper.findComponent(ItemDetailView).exists()).toBe(true);
    expect(wrapper.find('[data-testid="position-section"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="detail-popout"]').exists()).toBe(true);
  });
});
