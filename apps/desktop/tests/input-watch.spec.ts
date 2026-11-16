// 存档监视（P1-4）：去抖、更新/替换/占用/删除场景、只提示不自动分析、
// 监视器生命周期与关闭窗口清理。
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import App from "../src/App.vue";
import { MockDataSource } from "../src/data/mock";
import { INPUT_WATCH_INTERVAL_MS } from "../src/app/store";
import type { InputFingerprintDto } from "../src/data/source";
import type { AppStore } from "../src/app/store";
import { analyzeCandidate, candidatePath, createTestStore } from "./helpers";

const SAVE = "save202.dat";
const SAVE_PATH = candidatePath(SAVE);

function fingerprintOf(overrides: Partial<InputFingerprintDto> = {}): InputFingerprintDto {
  return {
    status: "present",
    size_bytes: 597_590,
    last_write_time: 1_760_000_000_000,
    file_identity: "mock-volume:save202.dat",
    code: null,
    message: null,
    ...overrides,
  };
}

interface Harness {
  store: AppStore;
  source: MockDataSource;
  analyzeSpy: ReturnType<typeof vi.spyOn>;
}

async function startWatching(): Promise<Harness> {
  const source = new MockDataSource({ delayMs: 0 });
  const { store } = await createTestStore({ source, watchInput: true });
  await analyzeCandidate(store, SAVE);
  await flushPromises();
  const analyzeSpy = vi.spyOn(source, "analyze");
  return { store, source, analyzeSpy };
}

/** 推进两个轮询周期：第一次进入 pending，第二次确认并提示。 */
async function advanceTwoPolls(): Promise<void> {
  await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
  await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
}

describe("存档监视", () => {
  let store: AppStore | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    store?.actions.dispose();
    store = null;
    vi.useRealTimers();
  });

  it("更新经约 500ms 去抖后提示；只提示，不自动分析", async () => {
    const harness = await startWatching();
    store = harness.store;
    expect(store.state.inputWatch.status).toBe("watching");

    harness.source.setFingerprint(SAVE_PATH, fingerprintOf({ last_write_time: 1_760_000_100_000 }));
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
    expect(store.state.inputWatch.status).toBe("watching");

    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
    expect(store.state.inputWatch.status).toBe("changed");
    expect(store.state.inputWatch.detectedAt).not.toBeNull();
    // 监视器启动后（此 spy 之后）没有发生第二次分析。
    expect(harness.analyzeSpy).toHaveBeenCalledTimes(0);

    // 提示期间不重复触发，也不自动分析。
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS * 3);
    expect(harness.analyzeSpy).toHaveBeenCalledTimes(0);
    expect(store.state.inputWatch.status).toBe("changed");
  });

  it("原子替换间隙（短暂缺失后恢复原文件）不提示", async () => {
    const harness = await startWatching();
    store = harness.store;

    harness.source.setFingerprint(SAVE_PATH, { ...fingerprintOf(), status: "missing" });
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
    expect(store.state.inputWatch.status).toBe("watching");

    // 下一轮恢复为原指纹：取消 pending，不提示。
    harness.source.setFingerprint(SAVE_PATH, fingerprintOf());
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
    expect(store.state.inputWatch.status).toBe("watching");
  });

  it("文件被替换（身份变化、大小相同）也会提示", async () => {
    const harness = await startWatching();
    store = harness.store;
    harness.source.setFingerprint(
      SAVE_PATH,
      fingerprintOf({ file_identity: "mock-volume:save202-replaced" }),
    );
    await advanceTwoPolls();
    expect(store.state.inputWatch.status).toBe("changed");
  });

  it("占用：持续不可读时提示原因码；恢复同指纹后回到 watching", async () => {
    const harness = await startWatching();
    store = harness.store;
    harness.source.setFingerprint(
      SAVE_PATH,
      fingerprintOf({ status: "unreadable", size_bytes: null, last_write_time: null, file_identity: null, code: "INPUT_UNREADABLE", message: "文件可能正在保存或被其他程序占用" }),
    );
    await advanceTwoPolls();
    expect(store.state.inputWatch.status).toBe("unreadable");
    expect(store.state.inputWatch.code).toBe("INPUT_UNREADABLE");

    harness.source.setFingerprint(SAVE_PATH, fingerprintOf());
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS);
    expect(store.state.inputWatch.status).toBe("watching");
  });

  it("删除：提示重新选择；确认后停止监视并回到导入页", async () => {
    const harness = await startWatching();
    store = harness.store;
    harness.source.setFingerprint(SAVE_PATH, fingerprintOf({ status: "missing", size_bytes: null, last_write_time: null, file_identity: null }));
    await advanceTwoPolls();
    expect(store.state.inputWatch.status).toBe("missing");

    store.actions.reselectFromWatch();
    expect(store.state.inputWatch.status).toBe("idle");
    expect(store.state.view).toBe("import");

    // 监视器已停止：继续推进时间不再查询指纹。
    const spy = vi.spyOn(harness.source, "fingerprintInput");
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS * 3);
    expect(spy).not.toHaveBeenCalled();
  });

  it("稍后：清除提示但保留基线，同一变化不重复提示", async () => {
    const harness = await startWatching();
    store = harness.store;
    harness.source.setFingerprint(SAVE_PATH, fingerprintOf({ last_write_time: 1_760_000_200_000 }));
    await advanceTwoPolls();
    expect(store.state.inputWatch.status).toBe("changed");

    store.actions.dismissInputWatch();
    expect(store.state.inputWatch.status).toBe("watching");
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS * 3);
    expect(store.state.inputWatch.status).toBe("watching");
  });

  it("提示条：显示刷新与稍后；恢复前不自动分析", async () => {
    const harness = await startWatching();
    store = harness.store;
    const wrapper = mount(App, {
      props: { source: harness.source, store, layoutWidth: 1280 } as never,
      attachTo: document.body,
    });
    expect(wrapper.find('[data-testid="input-watch-alert"]').exists()).toBe(false);

    harness.source.setFingerprint(SAVE_PATH, fingerprintOf({ last_write_time: 1_760_000_300_000 }));
    await advanceTwoPolls();
    await flushPromises();
    const alert = wrapper.find('[data-testid="input-watch-alert"]');
    expect(alert.exists()).toBe(true);
    expect(alert.text()).toContain("当前存档已更新");
    expect(wrapper.find('[data-testid="input-watch-refresh"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="input-watch-dismiss"]').exists()).toBe(true);
    expect(harness.analyzeSpy).toHaveBeenCalledTimes(0);

    // 点击刷新才分析；刷新完成后提示消失。
    await wrapper.find('[data-testid="input-watch-refresh"]').trigger("click");
    await flushPromises();
    expect(harness.analyzeSpy).toHaveBeenCalledTimes(1);
    expect(store.state.inputWatch.status).toBe("watching");
    await flushPromises();
    expect(wrapper.find('[data-testid="input-watch-alert"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it("关闭窗口（dispose）后停止轮询，无残留定时器", async () => {
    const harness = await startWatching();
    const spy = vi.spyOn(harness.source, "fingerprintInput");
    harness.store.actions.dispose();
    store = null;
    await vi.advanceTimersByTimeAsync(INPUT_WATCH_INTERVAL_MS * 4);
    expect(spy).not.toHaveBeenCalled();
  });
});
