<script setup lang="ts">
// 导出对话框（SX-016）：JSON/CSV、分享脱敏（默认开启）、目标新目录选择，
// 以及成功（显示实际目标）/失败（原因 + 恢复）/旧快照（过期标记）三态。
//
// 边界（开发方案“诊断 设置与导出”）：
// - 无报告、分析未完成或报告属于其他槽位时不能导出（入口禁用并解释）；
// - 不覆盖原存档或已有报告：目标必须是尚不存在的新目录；
// - 取消导出静默返回；写盘失败给原因与重试；成功后显示实际目标；
// - 旧报告只可在明确标记快照身份与 stale 的情况下导出（结果区常驻标记）。
import { computed } from "vue";
import { FolderOpen, FolderOpenDot, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormatPair } from "../app/i18n";
import { recoveryHint } from "../data/errors";
import { formatSnapshotTime } from "../app/format";
import UiButton from "./ui/UiButton.vue";
import UiDialog from "./ui/UiDialog.vue";
import CoverageNotice from "./CoverageNotice.vue";

const store = useStore();

const exportState = computed(() => store.state.exportState);
const target = computed(() => store.exportTargetPath.value);
const currentResult = computed(() => store.currentSlot.value?.result ?? null);
const busy = computed(() => exportState.value.status === "exporting");
const errorHint = computed(() =>
  exportState.value.error ? recoveryHint(exportState.value.error) : null,
);

const formatOptions = computed<{ value: "json" | "csv" | "both"; label: string }[]>(() => [
  { value: "both", label: "JSON + CSV" },
  { value: "json", label: t("exportDialog.json-only") },
  { value: "csv", label: t("exportDialog.csv-only-shareable-table") },
]);

function close(): void {
  store.actions.closeExportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    :title="t('exportDialog.export-current-snapshot')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("exportDialog.the-export-references-the-current-report-s") }}
      </p>

      <CoverageNotice
        v-if="currentResult?.freshness === 'stale'"
        tone="warning"
        test-id="export-stale-notice"
        :title="t('exportDialog.the-current-snapshot-is-stale-the-export')"
        :lines="[
          tFormatPair(`common.snapshot-time`, { v0: formatSnapshotTime(currentResult.snapshot_time) }),
          tFormatPair(`exportDialog.refresh-error`, { v0: currentResult.last_refresh_error?.code ?? '未知' }),
          t('exportDialog.the-artifacts-are-marked-stale'),
        ]"
      />

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("exportDialog.format") }}</h3>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('exportDialog.export-format')">
          <label
            v-for="option in formatOptions"
            :key="option.value"
            class="inline-flex cursor-pointer items-center gap-2 rounded-[6px] border px-3 py-1.5 text-[13px]"
            :class="
              exportState.format === option.value
                ? 'border-accent bg-accent-bg text-accent-strong'
                : 'border-line bg-surface text-ink'
            "
          >
            <input
              type="radio"
              name="export-format"
              :value="option.value"
              :checked="exportState.format === option.value"
              :disabled="busy"
              :data-testid="`export-format-${option.value}`"
              @change="store.actions.setExportFormat(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("exportDialog.share-redaction") }}</h3>
        <label class="flex cursor-pointer items-start gap-2 text-[13px]">
          <input
            type="checkbox"
            :checked="exportState.share"
            :disabled="busy"
            data-testid="export-share"
            class="mt-0.5"
            @change="store.actions.setExportShare(($event.target as HTMLInputElement).checked)"
          />
          <span>
            {{ t("exportDialog.remove-absolute-personal-paths-while-keeping-business") }}
            <span class="block text-xs text-muted">
              {{ t("exportDialog.unchecking-writes-full-local-paths-an-export") }}
            </span>
          </span>
        </label>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("common.target-location-new-folder-no-overwrite") }}</h3>
        <div class="flex flex-wrap items-center gap-2">
          <input
            :value="exportState.parent ?? ''"
            type="text"
            :placeholder="t('common.choose-the-parent-folder-for-the-target')"
            :aria-label="t('common.target-parent-folder')"
            data-testid="export-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setExportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickExportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("common.choose-location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("common.new-folder-name") }}</span>
          <input
            :value="exportState.folder"
            type="text"
            data-testid="export-folder"
            :disabled="busy"
            class="mono h-9 w-full rounded-[6px] border border-line bg-surface px-3 text-ink"
            @input="store.actions.setExportFolder(($event.target as HTMLInputElement).value)"
          />
        </label>
        <p class="mono text-xs break-all text-muted" data-testid="export-target">
          {{ t("common.actual-target") }}{{ target ?? t("common.to-be-filled") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="export-error"
        :title="tFormatPair(`common.export-failed`, { v0: exportState.error.code })"
        :lines="[
          exportState.error.message,
          errorHint ?? t('common.fix-the-issue-and-retry-cancelling-creates'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedExportFolder()">
          {{ tFormatPair(`common.use-suggested-folder-name`, { v0: exportState.suggestedFolder }) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runExport()">{{ t("common.retry-export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="export-success">
        <h3 class="text-sm font-medium">
          {{ exportState.outcome.freshness === "stale" ? t("exportDialog.export-complete-stale-snapshot-marked-as-stale") : t("common.export-complete") }}
        </h3>
        <p class="text-[13px] text-muted">
          {{ tFormatPair(`exportDialog.share-redaction-snapshot`, { v0: exportState.outcome.share ? "开启" : "关闭", v1: formatSnapshotTime(currentResult?.snapshot_time ?? null) }) }}
        </p>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <UiButton size="sm" data-testid="export-open-folder" @click="store.actions.openExportFolder(exportState.outcome.output_dir)">
          <FolderOpenDot :size="15" aria-hidden="true" />
          {{ t("common.open-export-folder") }}
        </UiButton>
        <p class="text-xs text-muted">{{ t("exportDialog.the-artifacts-were-written-to-the-target") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("exportDialog.writing-export-files") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("common.close") : t("common.cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="export-submit"
        @click="store.actions.runExport()"
      >
        {{ t("common.start-export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
