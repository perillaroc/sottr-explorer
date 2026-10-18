// 导入与槽位选择测试（SX-015 验收：中文路径、长名称、空列表说明、取消与失败状态）。
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import SlotPicker from "../src/components/SlotPicker.vue";
import { createTestStore, candidatePath, mountWithStore, ControlledSource, analyzeCandidate, MOCK_ROOT_B } from "./helpers";
import type { DataSource, DiscoverOutcomeDto } from "../src/data/source";

class NoGameSource implements DataSource {
  readonly kind = "mock" as const;
  async discover(): Promise<DiscoverOutcomeDto> {
    return {
      candidates: [
        {
          path: "D:\\Games\\Documents",
          source: "user_profile",
          source_label: "%USERPROFILE%\\Documents（回退候选）",
          status: "missing",
          detail: null,
        },
      ],
      game_roots: [],
    };
  }
  async listCandidates(root: string) {
    return { root, files: [], skipped: [], depth_limited_dirs: [] };
  }
  async pick(): Promise<string | null> {
    return null;
  }
  async analyze(): Promise<never> {
    throw new Error("不应调用");
  }
  cancel(): void {}
  async exportReport(): Promise<never> {
    throw new Error("不应调用");
  }
  async resolveGuideLink(): Promise<never> {
    throw new Error("不应调用");
  }
  async openGuideLink(): Promise<never> {
    throw new Error("不应调用");
  }
  async openExternalUrl(): Promise<never> {
    throw new Error("不应调用");
  }
}

describe("导入与槽位选择", () => {
  it("列出候选文件（文件名/本地时间+时区/大小/状态）并区分目录状态", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SlotPicker, store);
    const text = wrapper.text();
    expect(text).toContain("打开存档");
    expect(text).toContain("save1.dat");
    expect(text).toContain("583.6 KiB"); // 597,590 字节
    expect(text).toContain("（UTC");
    expect(text).toContain("待分析");
    expect(text).toContain("资料文件");
    // 目录状态分开说明：缺失与权限失败各自可读。
    expect(text).toContain("目录不存在");
    expect(text).toContain("无法读取：拒绝访问");
    // 不把修改时间当作正确存档；有“待分析”而不是占位百分比。
    expect(text).not.toContain("0.00%");
  });

  it("未选文件或用途时开始分析禁用并有就地说明", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SlotPicker, store);
    const button = wrapper.get('[data-testid="start-analysis"]');
    expect(button.attributes("disabled")).toBeDefined();
    expect(wrapper.get('[data-testid="start-hint"]').text()).toContain("请选择一个候选文件");

    // 选择文件后仍缺用途。
    await wrapper.findAll('input[type="radio"]')[0].setValue();
    expect(wrapper.get('[data-testid="start-hint"]').text()).toContain("请确认这个文件用于");

    // 选择用途后可用。
    await wrapper.get('[data-testid="purpose-campaign"]').setValue();
    expect(wrapper.get('[data-testid="start-analysis"]').attributes("disabled")).toBeUndefined();
  });

  it("分析中显示不定进度与取消；取消后保留原视图", async () => {
    const source = new ControlledSource();
    const { store } = await createTestStore({ source });
    const wrapper = mountWithStore(SlotPicker, store);
    await wrapper.findAll('input[type="radio"]')[0].setValue();
    await wrapper.get('[data-testid="purpose-campaign"]').setValue();
    await wrapper.get('[data-testid="start-analysis"]').trigger("click");
    expect(wrapper.get('[data-testid="analysis-progress"]').text()).toContain("正在分析 save1.dat");
    expect(wrapper.get('[data-testid="analysis-progress"]').text()).toContain("模拟：不定进度");
    // 不重复提交同一任务：开始按钮被进度状态取代。
    expect(wrapper.find('[data-testid="start-analysis"]').exists()).toBe(false);

    await wrapper.get('[data-testid="analysis-progress"] button').trigger("click");
    expect(store.state.analysis).toBeNull();
    expect(store.state.notice?.text).toContain("已取消");
    expect(wrapper.find('[data-testid="start-analysis"]').exists()).toBe(true);
  });

  it("首次分析失败显示错误与恢复入口，不出现正常结果假象", async () => {
    const { store } = await createTestStore();
    // 模拟：挑战古墓/损坏文件解压失败（不可重试），无正常结果假象。
    await analyzeCandidate(store, "challenge_tomb.dat");
    const wrapper = mountWithStore(SlotPicker, store);
    const text = wrapper.text();
    expect(text).toContain("分析失败");
    expect(text).toContain("DECOMPRESSION_FAILED");
    expect(text).toContain("不可重试");
    expect(text).toContain("失败（DECOMPRESSION_FAILED）");
  });

  it("被占用（可重试）场景：首次失败提示稍后重试，重试成功", async () => {
    const { store } = await createTestStore();
    // 点击第二个候选目录（Documents 回退候选）：应列出其游戏目录下的文件而不是空清单。
    const wrapper = mountWithStore(SlotPicker, store);
    await wrapper.get('[data-testid="root-1"]').trigger("click");
    await flushPromises();
    expect(store.state.activeRoot).toBe(MOCK_ROOT_B);
    expect(wrapper.text()).toContain("save_locked.dat");

    store.actions.selectCandidate(candidatePath("save_locked.dat", MOCK_ROOT_B));
    store.actions.setPurpose("campaign");
    await store.actions.startAnalysis();
    await flushPromises();
    const text = wrapper.text();
    expect(text).toContain("失败（INPUT_UNREADABLE）");
    expect(text).toContain("正在保存或被其他程序占用");
    expect(text).toContain("可重试");

    await store.actions.startAnalysis();
    expect(store.state.slots[candidatePath("save_locked.dat", MOCK_ROOT_B)]?.result?.freshness).toBe("current");
    expect(store.currentSlot.value?.result?.report).not.toBeNull();
  });

  it("未找到游戏目录时给出明确说明与手动入口", async () => {
    const { store } = await createTestStore({ source: new NoGameSource() });
    const wrapper = mountWithStore(SlotPicker, store);
    expect(wrapper.text()).toContain("未找到可读取的存档");
    expect(wrapper.text()).toContain("选择文件");
  });

  it("中文长文件名完整显示（不截断替换为省略号）", async () => {
    const { store } = await createTestStore();
    const longName = "这是一份名称非常长的中文存档文件用于检查换行与完整显示.dat";
    store.state.candidates = [
      ...store.state.candidates,
      {
        name: longName,
        path: `C:\\Users\\演示用户\\Documents\\Shadow of the Tomb Raider\\${longName}`,
        size_bytes: 123_456,
        modified_millis: Date.parse("2026-10-01T10:00:00+08:00"),
        matches_observed_names: true,
        status: "ready",
        detail: null,
        note: null,
      },
    ];
    const wrapper = mountWithStore(SlotPicker, store);
    expect(wrapper.text()).toContain(longName);
  });
});
