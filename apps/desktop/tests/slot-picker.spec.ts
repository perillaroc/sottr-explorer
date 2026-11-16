// 导入与槽位选择测试（SX-015 验收：中文路径、长名称、空列表说明、取消与失败状态）。
import { describe, expect, it } from "vitest";
import { flushPromises } from "@vue/test-utils";
import SlotPicker from "../src/components/SlotPicker.vue";
import { createTestStore, candidatePath, mountWithStore, ControlledSource, analyzeCandidate, MOCK_ROOT_A, MOCK_ROOT_B } from "./helpers";
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
  async fingerprintInput(): Promise<never> {
    throw new Error("不应调用");
  }
  async exportReport(): Promise<never> {
    throw new Error("不应调用");
  }
  async exportPlan(): Promise<never> {
    throw new Error("不应调用");
  }
  async openExportFolder(): Promise<never> {
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
    const progress = wrapper.get('[data-testid="analysis-progress"]');
    expect(progress.text()).toContain("正在分析 save1.dat");
    expect(progress.text()).toContain("模拟：不定进度");
    expect(progress.find('[role="progressbar"]').classes()).toContain("ui-progress--indeterminate");
    expect(progress.find(".ui-progress__fill--indeterminate").exists()).toBe(true);
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

  it("按账户子目录分组展示候选文件，同名文件不混在一起", async () => {
    const { store } = await createTestStore();
    const accountA = `${MOCK_ROOT_A}\\292733975847239680`;
    const accountB = `${MOCK_ROOT_A}\\76561198105637627`;
    const file = (dir: string, name: string, modified: string) => ({
      name,
      path: `${dir}\\${name}`,
      size_bytes: 597_590,
      modified_millis: Date.parse(modified),
      matches_observed_names: true,
      status: "ready" as const,
      detail: null,
      note: null,
    });
    store.state.candidates = [
      file(accountA, "profile.dat", "2026-10-10T21:14:00+08:00"),
      file(accountA, "save202.dat", "2026-10-10T21:14:05+08:00"),
      file(accountB, "profile.dat", "2026-10-09T18:00:00+08:00"),
      file(accountB, "save1.dat", "2026-10-09T18:02:31+08:00"),
    ];
    const wrapper = mountWithStore(SlotPicker, store);

    // 不同账户各自成组，标题写明账户数字与文件数（账户数字按数值排序）。
    const groups = wrapper.findAll('[data-testid="candidate-group"]');
    expect(groups).toHaveLength(2);
    expect(groups[0].text()).toContain("账户 76561198105637627");
    expect(groups[0].text()).toContain("2 个文件");
    expect(groups[1].text()).toContain("账户 292733975847239680");
    expect(groups[1].text()).toContain("2 个文件");

    // 同名文件分别落在各自分组内，单选仍按完整路径区分。
    expect(groups[0].text()).toContain("save1.dat");
    expect(groups[0].text()).toContain("profile.dat");
    expect(groups[0].text()).not.toContain("save202.dat");
    expect(groups[1].text()).toContain("save202.dat");
    expect(groups[1].text()).not.toContain("save1.dat");
    expect(wrapper.findAll('input[name="candidate"]')).toHaveLength(4);
    expect(wrapper.get('[data-testid="candidate-group-hint"]').text()).toContain("不同账户下的同名存档各自独立");
  });

  it("单一目录（无子目录）不显示分组标题", async () => {
    const { store } = await createTestStore();
    const wrapper = mountWithStore(SlotPicker, store);
    expect(wrapper.find('[data-testid="candidate-group-title"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="candidate-group-hint"]').exists()).toBe(false);
  });
});
