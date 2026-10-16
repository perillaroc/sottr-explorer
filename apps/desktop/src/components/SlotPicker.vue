<script setup lang="ts">
// 导入与槽位选择（SX-015）：候选目录 → 文件行 → 用途确认 → 开始分析/取消。
//
// 文案与门禁遵循开发方案：修改时间按系统本地时间显示并注明时区；不把最新时间
// 当作正确存档；未选文件或用途时开始分析禁用并有就地说明；模式声明不是格式识别证明。
import { computed, ref } from "vue";
import { FolderOpen, Info, RefreshCw, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { fileNameOf, formatBytes, formatLocalTime, formatPercent } from "../app/format";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import CoverageNotice from "./CoverageNotice.vue";
import type { CandidateFileDto } from "../data/source";

const store = useStore();
const expandedPath = ref<string | null>(null);

const candidates = computed(() => store.state.candidates);
const discovery = computed(() => store.state.discovery);
const analyzing = computed(() => store.state.analysis !== null);
const selected = computed(() => store.state.selectedPath);

/**
 * 候选 Documents 目录 → 游戏目录（用于列出文件与高亮当前目录）。
 * `discover_saves` 的 game_roots 与 game_found 候选按同一顺序返回，这里按序配对；
 * 未配对的候选退回使用自身路径（发现层仍会做有界枚举）。
 */
const listRootForCandidate = computed(() => {
  const map = new Map<string, string>();
  let index = 0;
  for (const candidate of discovery.value?.candidates ?? []) {
    if (candidate.status === "game_found") {
      const root = discovery.value?.game_roots[index];
      if (root) {
        map.set(candidate.path, root);
      }
      index += 1;
    }
  }
  return map;
});

function openCandidate(candidate: { path: string }): void {
  void store.actions.openRoot(listRootForCandidate.value.get(candidate.path) ?? candidate.path);
}

function isActiveCandidate(candidate: { path: string }): boolean {
  const root = listRootForCandidate.value.get(candidate.path) ?? candidate.path;
  return store.state.activeRoot === root;
}

const canStart = computed(
  () => selected.value !== null && store.state.purpose !== null && !analyzing.value && !store.state.loadingCandidates,
);

const startHint = computed(() => {
  if (store.state.loadingCandidates) {
    return "正在列出候选文件…";
  }
  if (!selected.value) {
    return "请选择一个候选文件（或使用“选择文件”）。";
  }
  if (!store.state.purpose) {
    return "请确认这个文件用于：普通战役存档 / 不确定，仅查看可用观察。";
  }
  return null;
});

const startError = computed(() => {
  const path = selected.value;
  if (!path) {
    return null;
  }
  return store.state.slots[path]?.startError ?? null;
});

function rootStatusText(status: string, detail: string | null): string {
  switch (status) {
    case "game_found":
      return "发现游戏目录";
    case "no_game_dir":
      return "未发现游戏目录（可能未安装或未使用该位置）";
    case "link_not_followed":
      return "目录是链接/重解析点，发现层不跟随；可用“选择文件夹”手动选择";
    case "missing":
      return "目录不存在";
    case "not_directory":
      return "路径不是目录";
    case "unreadable":
      return `无法读取：${detail ?? "权限等"}`;
    default:
      return "不支持的位置（UNC/设备命名空间）";
  }
}

function rowStatus(candidate: CandidateFileDto): { text: string; tone: "muted" | "info" | "caution" | "danger" } {
  const slot = store.state.slots[candidate.path];
  if (store.state.analysis?.path === candidate.path) {
    return { text: "分析中…", tone: "info" };
  }
  if (slot?.result) {
    const overall = slot.result.report?.observations.overall_completion.value ?? null;
    if (slot.result.freshness === "stale") {
      return { text: "旧快照（刷新失败）", tone: "caution" };
    }
    if (overall === null) {
      return { text: "已分析（整体候选不可用）", tone: "info" };
    }
    return { text: `已分析（整体推定 ${formatPercent(overall)}）`, tone: "info" };
  }
  if (slot?.startError) {
    return { text: `失败（${slot.startError.code}）`, tone: "danger" };
  }
  if (candidate.note?.includes("资料文件")) {
    return { text: "资料文件（待分析）", tone: "muted" };
  }
  return { text: "待分析", tone: "muted" };
}

function toggleDetail(path: string): void {
  expandedPath.value = expandedPath.value === path ? null : path;
}

function displayName(candidate: CandidateFileDto): string {
  return candidate.name || fileNameOf(candidate.path);
}

/** 行详情：完整路径、备注（如同内容关系）与已分析快照摘要。 */
function rowSummary(candidate: CandidateFileDto) {
  const slot = store.state.slots[candidate.path];
  const result = slot?.result ?? null;
  return {
    path: candidate.path,
    note: candidate.note,
    sizeBytes: candidate.size_bytes,
    snapshotTime: result?.snapshot_time ?? null,
    digestPrefix: result?.report?.input_digest.compressed_sha256.slice(0, 16) ?? null,
    capabilities: result?.report
      ? [
          `物品表：${result.report.observations.inventory.inventory_complete ? "完整" : "不完整"}`,
          `参考差集：${result.report.capabilities.reference_difference.status}`,
          `定位：${result.report.capabilities.location.status}`,
        ]
      : null,
  };
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="import-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="import-title" class="text-xl font-semibold">打开存档</h1>
        <p class="text-[13px] text-muted">选择文件后，查看当前支持类别中的参考遗漏。原始文件不会被改写。</p>
      </header>

      <div class="flex flex-wrap items-center gap-2">
        <UiButton variant="primary" @click="store.actions.pickInput('save-file')">
          <FolderOpen :size="16" aria-hidden="true" />
          选择文件
        </UiButton>
        <UiButton @click="store.actions.pickInput('save-directory')">
          <Search :size="16" aria-hidden="true" />
          选择文件夹
        </UiButton>
        <UiButton :disabled="analyzing" title="重新扫描受限候选位置（不做全盘扫描）" @click="store.actions.rescan()">
          <RefreshCw :size="16" aria-hidden="true" />
          重新查找默认目录
        </UiButton>
      </div>

      <CoverageNotice
        v-if="store.state.discoveryError"
        tone="danger"
        title="目录发现失败"
        :lines="[store.state.discoveryError.message, '本工具不会提权或修改权限；可用“选择文件/文件夹”手动指定。']"
      />

      <div v-if="discovery" class="space-y-2">
        <h2 class="text-base font-medium">候选目录</h2>
        <ul class="space-y-1">
          <li v-for="(candidate, index) in discovery.candidates" :key="candidate.path">
            <button
              type="button"
              class="flex w-full flex-wrap items-center gap-2 rounded-[8px] border px-3 py-2 text-left text-[13px]"
              :class="
                isActiveCandidate(candidate) && candidate.status === 'game_found'
                  ? 'border-accent bg-accent-bg text-accent-strong'
                  : candidate.status === 'game_found'
                    ? 'cursor-pointer border-line bg-surface text-ink hover:bg-raised'
                    : 'cursor-not-allowed border-line bg-raised text-muted'
              "
              :disabled="candidate.status !== 'game_found' || analyzing"
              :data-testid="`root-${index}`"
              :title="listRootForCandidate.get(candidate.path) ?? candidate.path"
              @click="openCandidate(candidate)"
            >
              <span class="font-medium break-all">{{ candidate.path }}</span>
              <span class="text-muted">· {{ candidate.source_label }}</span>
              <span class="ml-auto">{{ rootStatusText(candidate.status, candidate.detail) }}</span>
            </button>
          </li>
        </ul>
        <p v-if="discovery.game_roots.length === 0" class="text-[13px] text-muted">
          未找到可读取的存档。可尝试“选择文件/文件夹”，或确认游戏是否安装在本机。
        </p>
      </div>

      <div v-if="candidates.length > 0" class="space-y-2">
        <h2 class="text-base font-medium">文件</h2>
        <table class="w-full border-collapse text-[13px]">
          <caption class="sr-only">候选存档文件与读取状态</caption>
          <thead>
            <tr class="border-b border-line text-left text-muted">
              <th scope="col" class="w-8 py-1 font-normal">
                <span class="sr-only">选择</span>
              </th>
              <th scope="col" class="py-1 font-normal">文件</th>
              <th scope="col" class="py-1 font-normal">修改时间</th>
              <th scope="col" class="py-1 font-normal">大小</th>
              <th scope="col" class="py-1 font-normal">状态</th>
              <th scope="col" class="w-16 py-1 font-normal">
                <span class="sr-only">详情</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="candidate in candidates" :key="candidate.path">
              <tr class="border-b border-line/60 align-top">
                <td class="py-2">
                  <input
                    type="radio"
                    name="candidate"
                    :value="candidate.path"
                    :checked="selected === candidate.path"
                    :disabled="analyzing"
                    :aria-label="`选择 ${displayName(candidate)}`"
                    @change="store.actions.selectCandidate(candidate.path)"
                  />
                </td>
                <td class="py-2 pr-3">
                  <button
                    type="button"
                    class="cursor-pointer break-all text-left hover:underline"
                    @click="store.actions.selectCandidate(candidate.path)"
                  >
                    {{ displayName(candidate) }}
                  </button>
                  <p v-if="candidate.note" class="text-xs text-muted">{{ candidate.note }}</p>
                </td>
                <td class="py-2 pr-3 whitespace-nowrap">{{ formatLocalTime(candidate.modified_millis) }}</td>
                <td class="py-2 pr-3 whitespace-nowrap">{{ formatBytes(candidate.size_bytes) }}</td>
                <td class="py-2 pr-3">
                  <span
                    :class="
                      rowStatus(candidate).tone === 'danger'
                        ? 'text-danger'
                        : rowStatus(candidate).tone === 'caution'
                          ? 'text-caution'
                          : rowStatus(candidate).tone === 'info'
                            ? 'text-info'
                            : 'text-muted'
                    "
                    >{{ rowStatus(candidate).text }}</span
                  >
                </td>
                <td class="py-2">
                  <button
                    type="button"
                    class="cursor-pointer text-xs text-muted hover:text-ink hover:underline"
                    :aria-expanded="expandedPath === candidate.path"
                    @click="toggleDetail(candidate.path)"
                  >
                    详情
                  </button>
                </td>
              </tr>
              <tr v-if="expandedPath === candidate.path" class="border-b border-line/60 bg-raised/60">
                <td />
                <td colspan="5" class="py-2 text-xs text-muted">
                  <p class="break-all">完整路径：<span class="mono">{{ rowSummary(candidate).path }}</span></p>
                  <p v-if="rowSummary(candidate).note">备注：{{ rowSummary(candidate).note }}</p>
                  <p v-if="rowSummary(candidate).snapshotTime">
                    已分析快照：{{ formatLocalTime(Date.parse(rowSummary(candidate).snapshotTime ?? "")) }}；摘要前缀
                    <span class="mono">{{ rowSummary(candidate).digestPrefix }}…</span>
                  </p>
                  <p v-if="rowSummary(candidate).capabilities">
                    能力摘要：{{ rowSummary(candidate).capabilities?.join("；") }}
                  </p>
                  <p v-if="!rowSummary(candidate).snapshotTime">尚未分析：状态为“待分析”，不提前显示占位百分比。</p>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
        <p v-if="candidates.some((item) => item.note?.includes('相同'))" class="text-xs text-muted">
          提示：内容相同的文件不会自动删除、合并或替用户选定。
        </p>
      </div>

      <UiEmptyState
        v-else-if="discovery && discovery.game_roots.length > 0 && !store.state.loadingCandidates"
        title="该目录下没有可列出的候选文件"
        description="发现层只列出 *.dat（最多 128 个、深度 4 层）；可用“选择文件”手动选择其他位置。"
      />

      <div class="space-y-2">
        <h2 class="text-base font-medium">这个文件用于</h2>
        <div class="flex flex-col gap-1 text-[13px]">
          <label class="inline-flex cursor-pointer items-center gap-2">
            <input
              type="radio"
              name="purpose"
              value="campaign"
              :checked="store.state.purpose === 'campaign'"
              :disabled="analyzing"
              data-testid="purpose-campaign"
              @change="store.actions.setPurpose('campaign')"
            />
            普通战役存档
          </label>
          <label class="inline-flex cursor-pointer items-center gap-2">
            <input
              type="radio"
              name="purpose"
              value="unknown"
              :checked="store.state.purpose === 'unknown'"
              :disabled="analyzing"
              data-testid="purpose-unknown"
              @change="store.actions.setPurpose('unknown')"
            />
            不确定，仅查看可用观察
          </label>
        </div>
        <p class="flex items-start gap-1 text-xs text-muted">
          <Info :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
          模式声明不是格式识别证明；profile.dat 始终按资料文件处理；原始文件不会被改写。
        </p>
      </div>

      <div class="space-y-2">
        <div v-if="analyzing" class="flex flex-wrap items-center gap-3" data-testid="analysis-progress">
          <span class="text-[13px]">正在分析 {{ fileNameOf(store.state.analysis?.path ?? "") }}…（模拟：不定进度）</span>
          <UiButton variant="danger" size="sm" @click="store.actions.cancelAnalysis()">取消</UiButton>
        </div>
        <template v-else>
          <UiButton
            variant="primary"
            :disabled="!canStart"
            :title="canStart ? '开始分析（只读）' : (startHint ?? '')"
            data-testid="start-analysis"
            @click="store.actions.startAnalysis()"
          >
            开始分析
          </UiButton>
          <p v-if="startHint" class="text-xs text-muted" data-testid="start-hint">{{ startHint }}</p>
        </template>

        <CoverageNotice
          v-if="startError"
          tone="danger"
          title="分析失败"
          :lines="[`${startError.code}：${startError.message}`, startError.retryable ? '可重试：稍后再次开始分析，或选择稳定副本。' : '该输入不可重试；可另选文件或展开诊断查看原因。']"
        >
          <UiButton size="sm" @click="store.actions.startAnalysis()">重试分析</UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.setView('diagnostics')">展开诊断</UiButton>
        </CoverageNotice>
      </div>
    </div>
  </section>
</template>
