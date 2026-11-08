<script setup lang="ts">
// 导入与槽位选择（SX-015；2026-11-05 界面美化阶段三）：候选目录 → 文件行 → 用途确认 → 开始分析/取消。
//
// 文案与门禁遵循开发方案：修改时间按系统本地时间显示并注明时区；不把最新时间
// 当作正确存档；未选文件或用途时开始分析禁用并有就地说明；模式声明不是格式识别证明。
import { computed, ref } from "vue";
import { FolderOpen, RefreshCw, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
import { fileNameOf, formatBytes, formatLocalTime, formatPercent } from "../app/format";
import { pagePaddingClass } from "../app/layout";
import AppPageHeader from "./AppPageHeader.vue";
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
const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));

/**
 * 候选文件分组：真实存档位于 `Shadow of the Tomb Raider\<账户数字>\saveN.dat`，
 * 同一根目录下可能有多个账户子目录、彼此存在同名文件。这里按所在子目录分组，
 * 不把不同账户的文件混到同一张清单里（组内顺序沿用后端按路径排序的结果）。
 */
interface CandidateGroup {
  key: string;
  /** 相对活动根目录的子目录；`""` 为根目录本身，`null` 为根目录之外（手动选择）。 */
  relative: string | null;
  /** 该组文件所在目录的完整路径（用于提示）。 */
  dir: string;
  files: CandidateFileDto[];
}

/** 统一分隔符并去掉末尾分隔符：只用于比较/拆分，不改变原始路径的显示。 */
function toWindowsPath(path: string): string {
  return path.replace(/\//g, "\\").replace(/\\+$/, "");
}

function parentDirOf(path: string): string {
  const normalized = toWindowsPath(path);
  const index = normalized.lastIndexOf("\\");
  return index < 0 ? normalized : normalized.slice(0, index);
}

/** 相对活动根目录的子目录；不在该根目录下时返回 `null`。 */
function relativeDirOf(path: string, root: string | null): string | null {
  if (!root) {
    return null;
  }
  const dir = parentDirOf(path);
  const normalizedRoot = toWindowsPath(root);
  const dirLower = dir.toLowerCase();
  const rootLower = normalizedRoot.toLowerCase();
  if (dirLower === rootLower) {
    return "";
  }
  if (dirLower.startsWith(`${rootLower}\\`)) {
    return dir.slice(normalizedRoot.length + 1);
  }
  return null;
}

const candidateGroups = computed<CandidateGroup[]>(() => {
  const groups = new Map<string, CandidateGroup>();
  for (const candidate of candidates.value) {
    const dir = parentDirOf(candidate.path);
    const relative = relativeDirOf(candidate.path, store.state.activeRoot);
    const key = relative === null ? `outside:${dir.toLowerCase()}` : `under:${relative.toLowerCase()}`;
    let group = groups.get(key);
    if (!group) {
      group = { key, relative, dir, files: [] };
      groups.set(key, group);
    }
    group.files.push(candidate);
  }
  // 排序：根目录文件 → 各子目录（账户数字按数值）→ 根目录之外；组数不影响候选顺序。
  const rank = (group: CandidateGroup): number =>
    group.relative === "" ? 0 : group.relative === null ? 2 : 1;
  return [...groups.values()].sort((a, b) => {
    const byRank = rank(a) - rank(b);
    if (byRank !== 0) {
      return byRank;
    }
    return (a.relative ?? a.dir).localeCompare(b.relative ?? b.dir, undefined, { numeric: true });
  });
});

/** 分组标题：纯数字子目录按“账户”标注，其余按子目录显示。 */
function groupLabel(group: CandidateGroup): string {
  if (group.relative === null) {
    return t(`根目录之外：${group.dir}`, `Outside the root folder: ${group.dir}`);
  }
  if (group.relative === "") {
    return t("游戏根目录（不在账户子目录内）", "Game root folder (not inside an account subfolder)");
  }
  if (/^\d+$/.test(group.relative)) {
    return t(`账户 ${group.relative}`, `Account ${group.relative}`);
  }
  return t(`子目录 ${group.relative}`, `Subfolder ${group.relative}`);
}

function groupFileCount(group: CandidateGroup): string {
  return t(
    `${group.files.length} 个文件`,
    `${group.files.length} ${group.files.length === 1 ? "file" : "files"}`,
  );
}

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

function rootRowClass(candidate: { path: string; status: string }): string {
  const active = isActiveCandidate(candidate) && candidate.status === "game_found";
  const available = candidate.status === "game_found";
  return [
    "ui-interactive flex w-full flex-wrap items-center gap-x-3 gap-y-1 border-b border-line/60 px-3 py-2.5 text-left text-[13px] last:border-b-0",
    active
      ? "ui-row-selected text-accent-strong"
      : available
        ? "text-ink enabled:hover:bg-raised"
        : "cursor-not-allowed text-muted",
  ].join(" ");
}

function rootStatusTone(status: string): string {
  return status === "game_found" ? "text-muted" : "text-caution";
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

function statusToneClass(tone: "muted" | "info" | "caution" | "danger"): string {
  switch (tone) {
    case "danger":
      return "text-danger";
    case "caution":
      return "text-caution";
    case "info":
      return "text-info";
    default:
      return "text-muted";
  }
}

function toggleDetail(path: string): void {
  expandedPath.value = expandedPath.value === path ? null : path;
}

function displayName(candidate: CandidateFileDto): string {
  return candidate.name || fileNameOf(candidate.path);
}

function purposeClass(value: "campaign" | "unknown"): string {
  const active = store.state.purpose === value;
  const base =
    "ui-interactive flex min-h-9 cursor-pointer items-center gap-2 rounded-control border px-3 py-1.5 text-[13px]";
  if (analyzing.value) {
    return `${base} cursor-not-allowed border-line bg-surface text-muted opacity-60`;
  }
  return active
    ? `${base} border-accent bg-accent-bg text-accent-strong`
    : `${base} border-line bg-surface text-ink hover:bg-raised`;
}

function isSelectedFile(path: string): boolean {
  return selected.value === path;
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
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="import-title">
    <div class="mx-auto w-full max-w-[1120px] space-y-6 pb-8" :class="pageX">
      <AppPageHeader
        title-id="import-title"
        :title="t('打开存档', 'Open a save')"
        :description="t('选择 Steam 版战役存档；分析只读取文件，不会修改原始存档。', 'Choose a Steam campaign save; analysis only reads the file and never modifies the original.')"
      >
        <template #actions>
          <UiButton variant="primary" @click="store.actions.pickInput('save-file')">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("选择文件", "Choose file") }}
          </UiButton>
          <UiButton @click="store.actions.pickInput('save-directory')">
            <Search :size="16" aria-hidden="true" />
            {{ t("选择文件夹", "Choose folder") }}
          </UiButton>
          <UiButton
            variant="ghost"
            :disabled="analyzing"
            :title="t('重新扫描受限候选位置（不做全盘扫描）', 'Rescan limited candidate locations (no full-disk scan)')"
            @click="store.actions.rescan()"
          >
            <RefreshCw :size="16" aria-hidden="true" />
            {{ t("重新查找默认目录", "Rescan default folders") }}
          </UiButton>
        </template>
      </AppPageHeader>

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
        <h2 class="ui-section-title">{{ t("候选目录", "Candidate folders") }}</h2>
        <div class="ui-panel overflow-hidden" data-testid="candidate-roots">
          <button
            v-for="(candidate, index) in discovery.candidates"
            :key="candidate.path"
            type="button"
            :class="rootRowClass(candidate)"
            :disabled="candidate.status !== 'game_found' || analyzing"
            :data-testid="`root-${index}`"
            :title="listRootForCandidate.get(candidate.path) ?? candidate.path"
            @click="openCandidate(candidate)"
          >
            <span class="min-w-0 flex-1 break-all text-[13px] font-medium text-ink">{{ candidate.path }}</span>
            <span class="text-xs text-muted">{{ candidate.source_label }}</span>
            <span class="ml-auto text-xs" :class="rootStatusTone(candidate.status)">
              {{ rootStatusText(candidate.status, candidate.detail) }}
            </span>
          </button>
        </div>
        <UiEmptyState
          v-if="discovery.game_roots.length === 0"
          :title="t('未找到可读取的存档', 'No readable saves found')"
          :description="t('可尝试“选择文件/文件夹”，或确认游戏是否安装在本机。', 'Try “Choose file/folder”, or check whether the game is installed on this computer.')"
        >
          <UiButton size="sm" @click="store.actions.pickInput('save-file')">{{ t("选择文件", "Choose file") }}</UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.pickInput('save-directory')">{{ t("选择文件夹", "Choose folder") }}</UiButton>
        </UiEmptyState>
      </div>

      <div v-if="candidates.length > 0" class="space-y-2">
        <h2 class="ui-section-title">{{ t("文件", "Files") }}</h2>
        <p v-if="candidateGroups.length > 1" class="text-xs text-muted" data-testid="candidate-group-hint">
          {{
            t(
              "按子目录（账户）分组显示：不同账户下的同名存档各自独立，不会合并或自动选定。",
              "Grouped by subfolder (account): same-named saves in different accounts stay separate and are never merged or auto-selected.",
            )
          }}
        </p>
        <div class="ui-panel overflow-x-auto" data-testid="file-table">
          <table class="w-full min-w-[760px] border-collapse text-[13px]">
            <caption class="sr-only">{{ t("候选存档文件与读取状态", "Candidate save files and read status") }}</caption>
            <thead>
              <tr class="border-b border-line bg-surface text-left text-xs text-muted">
                <th scope="col" class="h-9 w-10 px-2 font-medium">
                  <span class="sr-only">{{ t("选择", "Select") }}</span>
                </th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("文件", "File") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("修改时间", "Modified") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("大小", "Size") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("状态", "Status") }}</th>
                <th scope="col" class="h-9 w-16 px-2 font-medium">
                  <span class="sr-only">{{ t("详情", "Details") }}</span>
                </th>
              </tr>
            </thead>
            <tbody v-for="group in candidateGroups" :key="group.key" data-testid="candidate-group">
              <tr v-if="candidateGroups.length > 1" class="border-b border-line bg-raised" data-testid="candidate-group-title">
                <th colspan="6" scope="colgroup" class="px-3 py-2 text-left text-xs font-medium text-ink" :title="group.dir">
                  <span class="break-all">{{ groupLabel(group) }}</span>
                  <span class="ml-2 font-normal text-muted">· {{ groupFileCount(group) }}</span>
                </th>
              </tr>
              <template v-for="candidate in group.files" :key="candidate.path">
                <tr class="h-11 border-b border-line/60 align-top" :class="isSelectedFile(candidate.path) ? 'bg-accent-bg' : 'hover:bg-raised'">
                  <td
                    class="border-l-[3px] py-2 pl-2"
                    :class="isSelectedFile(candidate.path) ? 'border-accent' : 'border-transparent'"
                  >
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
                  <td class="px-2 py-2">
                    <button
                      type="button"
                      class="cursor-pointer break-all text-left font-medium text-ink hover:underline"
                      @click="store.actions.selectCandidate(candidate.path)"
                    >
                      {{ displayName(candidate) }}
                    </button>
                    <p v-if="candidate.note" class="text-xs text-muted">{{ candidate.note }}</p>
                  </td>
                  <td class="px-2 py-2 whitespace-nowrap">{{ formatLocalTime(candidate.modified_millis) }}</td>
                  <td class="px-2 py-2 whitespace-nowrap">{{ formatBytes(candidate.size_bytes) }}</td>
                  <td class="px-2 py-2 text-xs" :class="statusToneClass(rowStatus(candidate).tone)">
                    {{ rowStatus(candidate).text }}
                  </td>
                  <td class="px-2 py-2">
                    <button
                      type="button"
                      class="ui-interactive inline-flex h-8 cursor-pointer items-center rounded-control px-2 text-xs text-muted enabled:hover:bg-raised enabled:hover:text-ink"
                      :aria-expanded="expandedPath === candidate.path"
                      @click="toggleDetail(candidate.path)"
                    >
                      {{ t("详情", "Details") }}
                    </button>
                  </td>
                </tr>
                <tr v-if="expandedPath === candidate.path" class="border-b border-line/60 bg-raised/60">
                  <td class="border-l-[3px] border-transparent" />
                  <td colspan="5" class="px-3 py-2 text-xs text-muted">
                    <p class="break-all">
                      {{ t("完整路径", "Full path") }}：<span class="mono">{{ rowSummary(candidate).path }}</span>
                    </p>
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
        </div>
      </div>

      <UiEmptyState
        v-else-if="discovery && discovery.game_roots.length > 0 && !store.state.loadingCandidates"
        :title="t('该目录下没有可列出的候选文件', 'No candidate files to list in this folder')"
        :description="t('发现层只列出 *.dat（最多 128 个、深度 4 层）；可用“选择文件”手动选择其他位置。', 'Discovery lists only *.dat files (up to 128, depth 4); use “Choose file” to pick another location manually.')"
      />

      <section class="ui-panel p-4" data-testid="purpose-panel">
        <h2 class="ui-section-title">{{ t("这个文件用于", "This file is used for") }}</h2>
        <div class="mt-3 flex flex-wrap gap-2">
          <label :class="purposeClass('campaign')">
            <input
              type="radio"
              name="purpose"
              value="campaign"
              :checked="store.state.purpose === 'campaign'"
              :disabled="analyzing"
              data-testid="purpose-campaign"
              @change="store.actions.setPurpose('campaign')"
            />
            <span>{{ t("普通战役存档", "Campaign save") }}</span>
          </label>
          <label :class="purposeClass('unknown')">
            <input
              type="radio"
              name="purpose"
              value="unknown"
              :checked="store.state.purpose === 'unknown'"
              :disabled="analyzing"
              data-testid="purpose-unknown"
              @change="store.actions.setPurpose('unknown')"
            />
            <span>{{ t("不确定，仅查看可用观察", "Not sure; view available observations only") }}</span>
          </label>
        </div>

        <div class="mt-4 border-t border-line pt-3">
          <div v-if="analyzing" class="space-y-2" data-testid="analysis-progress">
            <p class="text-[13px]">
              {{ t(`正在分析 ${fileNameOf(store.state.analysis?.path ?? "")}…（模拟：不定进度）`, `Analyzing ${fileNameOf(store.state.analysis?.path ?? "")}… (mock: indeterminate progress)`) }}
            </p>
            <div
              class="ui-progress ui-progress--indeterminate"
              role="progressbar"
              :aria-label="t('正在分析存档', 'Analyzing save')"
            >
              <div class="ui-progress__fill ui-progress__fill--indeterminate"></div>
            </div>
            <UiButton variant="danger" size="sm" @click="store.actions.cancelAnalysis()">{{ t("取消", "Cancel") }}</UiButton>
          </div>
          <template v-else>
            <div class="flex flex-wrap items-center gap-3">
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
            </div>
          </template>
        </div>

        <CoverageNotice
          v-if="startError"
          class="mt-3"
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
      </section>
    </div>
  </section>
</template>
