<script setup lang="ts">
// 导入与槽位选择（SX-015；2026-11-05 界面美化阶段三）：候选目录 → 文件行 → 用途确认 → 开始分析/取消。
//
// 文案与门禁遵循开发方案：修改时间按系统本地时间显示并注明时区；不把最新时间
// 当作正确存档；未选文件或用途时开始分析禁用并有就地说明；模式声明不是格式识别证明。
import { computed, ref } from "vue";
import { FolderOpen, RefreshCw, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, tFormatPair } from "../app/i18n";
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
    return tFormatPair(`slotPicker.outside-the-root-folder`, { v0: group.dir });
  }
  if (group.relative === "") {
    return t("slotPicker.game-root-folder-not-inside-an-account");
  }
  if (/^\d+$/.test(group.relative)) {
    return tFormatPair(`slotPicker.account`, { v0: group.relative });
  }
  return tFormatPair(`slotPicker.subfolder`, { v0: group.relative });
}

function groupFileCount(group: CandidateGroup): string {
  return tFormatPair(`slotPicker.text`, { v0: group.files.length });
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
    return t("slotPicker.listing-candidate-files");
  }
  if (!selected.value) {
    return t("slotPicker.select-a-candidate-file-or-use-choose");
  }
  if (!store.state.purpose) {
    return t("slotPicker.confirm-how-this-file-is-used-campaign");
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
      return t("slotPicker.game-folder-found");
    case "no_game_dir":
      return t("slotPicker.no-game-folder-found-not-installed");
    case "link_not_followed":
      return t("slotPicker.the-folder-is-a-link-reparse-point");
    case "missing":
      return t("slotPicker.folder-does-not-exist");
    case "not_directory":
      return t("slotPicker.path-is-not-a-folder");
    case "unreadable":
      return tFormat("slotPicker.cannot-read",  { detail: detail ?? t("slotPicker.permissions-etc") });
    default:
      return t("slotPicker.unsupported-location-unc-device-namespace");
  }
}

function rowStatus(candidate: CandidateFileDto): { text: string; tone: "muted" | "info" | "caution" | "danger" } {
  const slot = store.state.slots[candidate.path];
  if (store.state.analysis?.path === candidate.path) {
    return { text: t("slotPicker.analyzing"), tone: "info" };
  }
  if (slot?.result) {
    const overall = slot.result.report?.observations.overall_completion.value ?? null;
    if (slot.result.freshness === "stale") {
      return { text: t("slotPicker.stale-snapshot-refresh-failed"), tone: "caution" };
    }
    if (overall === null) {
      return { text: t("slotPicker.analyzed-overall-candidate-unavailable"), tone: "info" };
    }
    return {
      text: tFormatPair(`slotPicker.analyzed-overall-estimate`, { v0: formatPercent(overall) }),
      tone: "info",
    };
  }
  if (slot?.startError) {
    return { text: tFormatPair(`slotPicker.failed`, { v0: slot.startError.code }), tone: "danger" };
  }
  if (candidate.note?.includes("资料文件")) {
    return { text: t("slotPicker.profile-file-not-analyzed"), tone: "muted" };
  }
  return { text: t("slotPicker.not-analyzed"), tone: "muted" };
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
          tFormatPair(`slotPicker.inventory`, { v0: result.report.observations.inventory.inventory_complete ? "完整" : "不完整" }),
          tFormatPair(`slotPicker.reference-difference`, { v0: result.report.capabilities.reference_difference.status }),
          tFormatPair(`slotPicker.location`, { v0: result.report.capabilities.location.status }),
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
        :title="t('common.open-a-save')"
        :description="t('slotPicker.choose-a-steam-campaign-save-analysis-only')"
      >
        <template #actions>
          <UiButton variant="primary" @click="store.actions.pickInput('save-file')">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("slotPicker.choose-file") }}
          </UiButton>
          <UiButton @click="store.actions.pickInput('save-directory')">
            <Search :size="16" aria-hidden="true" />
            {{ t("slotPicker.choose-folder") }}
          </UiButton>
          <UiButton
            variant="ghost"
            :disabled="analyzing"
            :title="t('slotPicker.rescan-limited-candidate-locations-no-full-disk')"
            @click="store.actions.rescan()"
          >
            <RefreshCw :size="16" aria-hidden="true" />
            {{ t("appTopBar.rescan-default-folders") }}
          </UiButton>
        </template>
      </AppPageHeader>

      <CoverageNotice
        v-if="store.state.discoveryError"
        tone="danger"
        :title="t('slotPicker.folder-discovery-failed')"
        :lines="[
          store.state.discoveryError.message,
          t('slotPicker.this-tool-never-elevates-privileges-or-changes'),
        ]"
      />

      <div v-if="discovery" class="space-y-2">
        <h2 class="ui-section-title">{{ t("slotPicker.candidate-folders") }}</h2>
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
          :title="t('slotPicker.no-readable-saves-found')"
          :description="t('slotPicker.try-choose-file-folder-or-check-whether')"
        >
          <UiButton size="sm" @click="store.actions.pickInput('save-file')">{{ t("slotPicker.choose-file") }}</UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.pickInput('save-directory')">{{ t("slotPicker.choose-folder") }}</UiButton>
        </UiEmptyState>
      </div>

      <div v-if="candidates.length > 0" class="space-y-2">
        <h2 class="ui-section-title">{{ t("slotPicker.files") }}</h2>
        <p v-if="candidateGroups.length > 1" class="text-xs text-muted" data-testid="candidate-group-hint">
          {{
            t("slotPicker.grouped-by-subfolder-account-same-named-saves")
          }}
        </p>
        <div class="ui-panel overflow-x-auto" data-testid="file-table">
          <table class="w-full min-w-[760px] border-collapse text-[13px]">
            <caption class="sr-only">{{ t("slotPicker.candidate-save-files-and-read-status") }}</caption>
            <thead>
              <tr class="border-b border-line bg-surface text-left text-xs text-muted">
                <th scope="col" class="h-9 w-10 px-2 font-medium">
                  <span class="sr-only">{{ t("slotPicker.select") }}</span>
                </th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("slotPicker.file") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("slotPicker.modified") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("slotPicker.size") }}</th>
                <th scope="col" class="h-9 px-2 font-medium">{{ t("common.status") }}</th>
                <th scope="col" class="h-9 w-16 px-2 font-medium">
                  <span class="sr-only">{{ t("common.details") }}</span>
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
                      :aria-label="tFormatPair(`slotPicker.select-2`, { v0: displayName(candidate) })"
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
                      {{ t("common.details") }}
                    </button>
                  </td>
                </tr>
                <tr v-if="expandedPath === candidate.path" class="border-b border-line/60 bg-raised/60">
                  <td class="border-l-[3px] border-transparent" />
                  <td colspan="5" class="px-3 py-2 text-xs text-muted">
                    <p class="break-all">
                      {{ t("slotPicker.full-path") }}：<span class="mono">{{ rowSummary(candidate).path }}</span>
                    </p>
                    <p v-if="rowSummary(candidate).note">{{ t("slotPicker.note") }}：{{ rowSummary(candidate).note }}</p>
                    <p v-if="rowSummary(candidate).snapshotTime">
                      {{ t("slotPicker.analyzed-snapshot") }}：{{ formatLocalTime(Date.parse(rowSummary(candidate).snapshotTime ?? "")) }}；{{ t("slotPicker.digest-prefix") }}
                      <span class="mono">{{ rowSummary(candidate).digestPrefix }}…</span>
                    </p>
                    <p v-if="rowSummary(candidate).capabilities">
                      {{ t("slotPicker.capabilities") }}：{{ rowSummary(candidate).capabilities?.join(t("common.path-separator")) }}
                    </p>
                    <p v-if="!rowSummary(candidate).snapshotTime">
                      {{ t("slotPicker.not-analyzed-yet-shown-as-not-analyzed") }}
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
        :title="t('slotPicker.no-candidate-files-to-list')"
        :description="t('slotPicker.discovery-lists-only-dat-files-up')"
      />

      <section class="ui-panel p-4" data-testid="purpose-panel">
        <h2 class="ui-section-title">{{ t("slotPicker.this-file-is-used") }}</h2>
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
            <span>{{ t("slotPicker.campaign-save") }}</span>
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
            <span>{{ t("slotPicker.not-sure-view-available-observations-only") }}</span>
          </label>
        </div>

        <div class="mt-4 border-t border-line pt-3">
          <div v-if="analyzing" class="space-y-2" data-testid="analysis-progress">
            <p class="text-[13px]">
              {{ tFormatPair(`slotPicker.analyzing-mock-indeterminate-progress`, { v0: fileNameOf(store.state.analysis?.path ?? "") }) }}
            </p>
            <div
              class="ui-progress ui-progress--indeterminate"
              role="progressbar"
              :aria-label="t('slotPicker.analyzing-save')"
            >
              <div class="ui-progress__fill ui-progress__fill--indeterminate"></div>
            </div>
            <UiButton variant="danger" size="sm" @click="store.actions.cancelAnalysis()">{{ t("common.cancel") }}</UiButton>
          </div>
          <template v-else>
            <div class="flex flex-wrap items-center gap-3">
              <UiButton
                variant="primary"
                :disabled="!canStart"
                :title="canStart ? t('slotPicker.start-analysis-read-only') : (startHint ?? '')"
                data-testid="start-analysis"
                @click="store.actions.startAnalysis()"
              >
                {{ t("slotPicker.start-analysis") }}
              </UiButton>
              <p v-if="startHint" class="text-xs text-muted" data-testid="start-hint">{{ startHint }}</p>
            </div>
          </template>
        </div>

        <CoverageNotice
          v-if="startError"
          class="mt-3"
          tone="danger"
          :title="t('appTopBar.analysis-failed-2')"
          :lines="[
            `${startError.code}：${startError.message}`,
            startError.retryable
              ? t('slotPicker.retryable-start-the-analysis-again-later')
              : t('slotPicker.this-input-is-not-retryable-choose-another'),
          ]"
        >
          <UiButton size="sm" @click="store.actions.startAnalysis()">{{ t("slotPicker.retry-analysis") }}</UiButton>
          <UiButton size="sm" variant="ghost" @click="store.actions.setView('diagnostics')">{{ t("slotPicker.open-diagnostics") }}</UiButton>
        </CoverageNotice>
      </section>
    </div>
  </section>
</template>
