<script setup lang="ts">
// 导入与槽位选择（SX-015）：候选目录 → 文件行 → 用途确认 → 开始分析/取消。
//
// 文案与门禁遵循开发方案：修改时间按系统本地时间显示并注明时区；不把最新时间
// 当作正确存档；未选文件或用途时开始分析禁用并有就地说明；模式声明不是格式识别证明。
import { computed, ref } from "vue";
import { FolderOpen, Info, RefreshCw, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
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
    return t("正在列出候选文件…", "Listing candidate files…");
  }
  if (!selected.value) {
    return t("请选择一个候选文件（或使用“选择文件”）。", "Select a candidate file (or use “Choose file”).");
  }
  if (!store.state.purpose) {
    return t(
      "请确认这个文件用于：普通战役存档 / 不确定，仅查看可用观察。",
      "Confirm how this file is used: campaign save / unsure, view available observations only.",
    );
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
      return t("发现游戏目录", "Game folder found");
    case "no_game_dir":
      return t("未发现游戏目录（可能未安装或未使用该位置）", "No game folder found (not installed, or not using this location)");
    case "link_not_followed":
      return t(
        "目录是链接/重解析点，发现层不跟随；可用“选择文件夹”手动选择",
        "The folder is a link/reparse point, which discovery does not follow; use “Choose folder” to select it manually",
      );
    case "missing":
      return t("目录不存在", "Folder does not exist");
    case "not_directory":
      return t("路径不是目录", "Path is not a folder");
    case "unreadable":
      return tFormat("无法读取：{detail}", "Cannot read: {detail}", { detail: detail ?? t("权限等", "permissions, etc.") });
    default:
      return t("不支持的位置（UNC/设备命名空间）", "Unsupported location (UNC/device namespace)");
  }
}

function rowStatus(candidate: CandidateFileDto): { text: string; tone: "muted" | "info" | "caution" | "danger" } {
  const slot = store.state.slots[candidate.path];
  if (store.state.analysis?.path === candidate.path) {
    return { text: t("分析中…", "Analyzing…"), tone: "info" };
  }
  if (slot?.result) {
    const overall = slot.result.report?.observations.overall_completion.value ?? null;
    if (slot.result.freshness === "stale") {
      return { text: t("旧快照（刷新失败）", "Stale snapshot (refresh failed)"), tone: "caution" };
    }
    if (overall === null) {
      return { text: t("已分析（整体候选不可用）", "Analyzed (overall candidate unavailable)"), tone: "info" };
    }
    return {
      text: t(`已分析（整体推定 ${formatPercent(overall)}）`, `Analyzed (overall estimate ${formatPercent(overall)})`),
      tone: "info",
    };
  }
  if (slot?.startError) {
    return { text: t(`失败（${slot.startError.code}）`, `Failed (${slot.startError.code})`), tone: "danger" };
  }
  if (candidate.note?.includes("资料文件")) {
    return { text: t("资料文件（待分析）", "Profile file (not analyzed)"), tone: "muted" };
  }
  return { text: t("待分析", "Not analyzed"), tone: "muted" };
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
          t(
            `物品表：${result.report.observations.inventory.inventory_complete ? "完整" : "不完整"}`,
            `Inventory: ${result.report.observations.inventory.inventory_complete ? "complete" : "incomplete"}`,
          ),
          t(`参考差集：${result.report.capabilities.reference_difference.status}`, `Reference difference: ${result.report.capabilities.reference_difference.status}`),
          t(`定位：${result.report.capabilities.location.status}`, `Location: ${result.report.capabilities.location.status}`),
        ]
      : null,
  };
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="import-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="import-title" class="text-xl font-semibold">{{ t("打开存档", "Open a save") }}</h1>
        <p class="text-[13px] text-muted">
          {{ t("选择文件后，查看当前支持类别中的参考遗漏。原始文件不会被改写。", "Choose a file to see reference gaps within the currently supported categories. The original file is never modified.") }}
        </p>
      </header>

      <div class="flex flex-wrap items-center gap-2">
        <UiButton variant="primary" @click="store.actions.pickInput('save-file')">
          <FolderOpen :size="16" aria-hidden="true" />
          {{ t("选择文件", "Choose file") }}
        </UiButton>
        <UiButton @click="store.actions.pickInput('save-directory')">
          <Search :size="16" aria-hidden="true" />
          {{ t("选择文件夹", "Choose folder") }}
        </UiButton>
        <UiButton :disabled="analyzing" :title="t('重新扫描受限候选位置（不做全盘扫描）', 'Rescan limited candidate locations (no full-disk scan)')" @click="store.actions.rescan()">
          <RefreshCw :size="16" aria-hidden="true" />
          {{ t("重新查找默认目录", "Rescan default folders") }}
        </UiButton>
      </div>

      <CoverageNotice
        v-if="store.state.discoveryError"
        tone="danger"
        :title="t('目录发现失败', 'Folder discovery failed')"
        :lines="[
          store.state.discoveryError.message,
          t('本工具不会提权或修改权限；可用“选择文件/文件夹”手动指定。', 'This tool never elevates privileges or changes permissions; use “Choose file/folder” to specify a location manually.'),
        ]"
      />

      <div v-if="discovery" class="space-y-2">
        <h2 class="text-base font-medium">{{ t("候选目录", "Candidate folders") }}</h2>
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
          {{ t("未找到可读取的存档。可尝试“选择文件/文件夹”，或确认游戏是否安装在本机。", "No readable saves found. Try “Choose file/folder”, or check whether the game is installed on this computer.") }}
        </p>
      </div>

      <div v-if="candidates.length > 0" class="space-y-2">
        <h2 class="text-base font-medium">{{ t("文件", "Files") }}</h2>
        <table class="w-full border-collapse text-[13px]">
          <caption class="sr-only">{{ t("候选存档文件与读取状态", "Candidate save files and read status") }}</caption>
          <thead>
            <tr class="border-b border-line text-left text-muted">
              <th scope="col" class="w-8 py-1 font-normal">
                <span class="sr-only">{{ t("选择", "Select") }}</span>
              </th>
              <th scope="col" class="py-1 font-normal">{{ t("文件", "File") }}</th>
              <th scope="col" class="py-1 font-normal">{{ t("修改时间", "Modified") }}</th>
              <th scope="col" class="py-1 font-normal">{{ t("大小", "Size") }}</th>
              <th scope="col" class="py-1 font-normal">{{ t("状态", "Status") }}</th>
              <th scope="col" class="w-16 py-1 font-normal">
                <span class="sr-only">{{ t("详情", "Details") }}</span>
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
                    :aria-label="t(`选择 ${displayName(candidate)}`, `Select ${displayName(candidate)}`)"
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
                    {{ t("详情", "Details") }}
                  </button>
                </td>
              </tr>
              <tr v-if="expandedPath === candidate.path" class="border-b border-line/60 bg-raised/60">
                <td />
                <td colspan="5" class="py-2 text-xs text-muted">
                  <p class="break-all">{{ t("完整路径", "Full path") }}：<span class="mono">{{ rowSummary(candidate).path }}</span></p>
                  <p v-if="rowSummary(candidate).note">{{ t("备注", "Note") }}：{{ rowSummary(candidate).note }}</p>
                  <p v-if="rowSummary(candidate).snapshotTime">
                    {{ t("已分析快照", "Analyzed snapshot") }}：{{ formatLocalTime(Date.parse(rowSummary(candidate).snapshotTime ?? "")) }}；{{ t("摘要前缀", "digest prefix") }}
                    <span class="mono">{{ rowSummary(candidate).digestPrefix }}…</span>
                  </p>
                  <p v-if="rowSummary(candidate).capabilities">
                    {{ t("能力摘要", "Capabilities") }}：{{ rowSummary(candidate).capabilities?.join(t("；", "; ")) }}
                  </p>
                  <p v-if="!rowSummary(candidate).snapshotTime">
                    {{ t("尚未分析：状态为“待分析”，不提前显示占位百分比。", "Not analyzed yet: shown as “Not analyzed”, with no placeholder percentage.") }}
                  </p>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
        <p v-if="candidates.some((item) => item.note?.includes('相同'))" class="text-xs text-muted">
          {{ t("提示：内容相同的文件不会自动删除、合并或替用户选定。", "Note: files with identical content are never automatically deleted, merged, or selected on your behalf.") }}
        </p>
      </div>

      <UiEmptyState
        v-else-if="discovery && discovery.game_roots.length > 0 && !store.state.loadingCandidates"
        :title="t('该目录下没有可列出的候选文件', 'No candidate files to list in this folder')"
        :description="t('发现层只列出 *.dat（最多 128 个、深度 4 层）；可用“选择文件”手动选择其他位置。', 'Discovery lists only *.dat files (up to 128, depth 4); use “Choose file” to pick another location manually.')"
      />

      <div class="space-y-2">
        <h2 class="text-base font-medium">{{ t("这个文件用于", "This file is used for") }}</h2>
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
            {{ t("普通战役存档", "Campaign save") }}
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
            {{ t("不确定，仅查看可用观察", "Not sure; view available observations only") }}
          </label>
        </div>
        <p class="flex items-start gap-1 text-xs text-muted">
          <Info :size="14" class="mt-0.5 shrink-0" aria-hidden="true" />
          {{ t("模式声明不是格式识别证明；profile.dat 始终按资料文件处理；原始文件不会被改写。", "The declared mode is not proof of format detection; profile.dat is always treated as a profile file; the original file is never modified.") }}
        </p>
      </div>

      <div class="space-y-2">
        <div v-if="analyzing" class="flex flex-wrap items-center gap-3" data-testid="analysis-progress">
          <span class="text-[13px]">
            {{ t(`正在分析 ${fileNameOf(store.state.analysis?.path ?? "")}…（模拟：不定进度）`, `Analyzing ${fileNameOf(store.state.analysis?.path ?? "")}… (mock: indeterminate progress)`) }}
          </span>
          <UiButton variant="danger" size="sm" @click="store.actions.cancelAnalysis()">{{ t("取消", "Cancel") }}</UiButton>
        </div>
        <template v-else>
          <UiButton
            variant="primary"
            :disabled="!canStart"
            :title="canStart ? t('开始分析（只读）', 'Start analysis (read-only)') : (startHint ?? '')"
            data-testid="start-analysis"
            @click="store.actions.startAnalysis()"
          >
            {{ t("开始分析", "Start analysis") }}
          </UiButton>
          <p v-if="startHint" class="text-xs text-muted" data-testid="start-hint">{{ startHint }}</p>
        </template>

        <CoverageNotice
          v-if="startError"
          tone="danger"
          :title="t('分析失败', 'Analysis failed')"
          :lines="[
            `${startError.code}：${startError.message}`,
            startError.retryable
              ? t('可重试：稍后再次开始分析，或选择稳定副本。', 'Retryable: start the analysis again later, or choose a stable copy.')
              : t('该输入不可重试；可另选文件或展开诊断查看原因。', 'This input is not retryable; choose another file or open Diagnostics for details.'),
          ]"
        >
          <UiButton size="sm" @click="store.actions.startAnalysis()">{{ t("重试分析", "Retry analysis") }}</UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.setView('diagnostics')">{{ t("展开诊断", "Open Diagnostics") }}</UiButton>
        </CoverageNotice>
      </div>
    </div>
  </section>
</template>
