<script setup lang="ts">
// 收集计划清单导出（P1-2）：可打印 HTML / CSV、分享脱敏、目标新目录。
//
// 边界与报告导出（SX-016）一致：无报告或分析未完成时不能导出；目标必须是
// 尚不存在的新目录；取消不产生任何文件；成功后显示实际产物与目录。
// 导出内容按**当前筛选**生成，对话框里显式写出范围，避免“以为导出了全部”。
import { computed } from "vue";
import { FolderOpen, FolderOpenDot, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormatPair } from "../app/i18n";
import { recoveryHint } from "../data/errors";
import { formatSnapshotTime } from "../app/format";
import type { PlanExportFormat } from "../data/source";
import UiButton from "./ui/UiButton.vue";
import UiDialog from "./ui/UiDialog.vue";
import CoverageNotice from "./CoverageNotice.vue";

const store = useStore();

const exportState = computed(() => store.state.planExportState);
const target = computed(() => store.planExportTargetPath.value);
const checklist = computed(() => store.planChecklist.value);
const currentResult = computed(() => store.currentSlot.value?.result ?? null);
const busy = computed(() => exportState.value.status === "exporting");
const errorHint = computed(() =>
  exportState.value.error ? recoveryHint(exportState.value.error) : null,
);

const formatOptions = computed<{ value: PlanExportFormat; label: string }[]>(() => [
  { value: "both", label: t("planExportDialog.html-csv") },
  { value: "html", label: t("planExportDialog.html-only-printable-checklist") },
  { value: "csv", label: t("planExportDialog.csv-only-table") },
]);

function close(): void {
  store.actions.closePlanExportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    :title="t('planExportDialog.export-collection-checklist')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("planExportDialog.the-export-contains-uncollected-items-under") }}
      </p>

      <section class="rounded-[6px] border border-line bg-raised/40 px-3 py-2 text-[13px]">
        <p class="font-medium text-ink">{{ t("planExportDialog.current-scope") }}</p>
        <p class="mt-0.5 text-muted" data-testid="plan-export-scope">{{ store.planScopeText.value }}</p>
        <p class="mt-0.5 text-xs text-muted" data-testid="plan-export-count">
          {{ tFormatPair(`planExportDialog.uncollected-required`, { v0: checklist.items, v1: checklist.required }) }}
        </p>
      </section>

      <CoverageNotice
        v-if="currentResult?.freshness === 'stale'"
        tone="warning"
        test-id="plan-export-stale-notice"
        :title="t('planExportDialog.the-current-snapshot-is-stale-the-checklist')"
        :lines="[
          tFormatPair(`common.snapshot-time`, { v0: formatSnapshotTime(currentResult.snapshot_time) }),
          t('planExportDialog.refresh-or-re-analyze-first-or-accept'),
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
              name="plan-export-format"
              :value="option.value"
              :checked="exportState.format === option.value"
              :disabled="busy"
              :data-testid="`plan-export-format-${option.value}`"
              @change="store.actions.setPlanExportFormat(option.value)"
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
            data-testid="plan-export-share"
            class="mt-0.5"
            @change="store.actions.setPlanExportShare(($event.target as HTMLInputElement).checked)"
          />
          <span>
            {{ t("planExportDialog.omit-the-input-file-name-snapshot-time") }}
            <span class="block text-xs text-muted">
              {{ t("planExportDialog.unchecking-writes-the-save-file-name") }}
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
            data-testid="plan-export-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setPlanExportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickPlanExportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("common.choose-location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("common.new-folder-name") }}</span>
          <input
            :value="exportState.folder"
            type="text"
            data-testid="plan-export-folder"
            :disabled="busy"
            class="mono h-9 w-full rounded-[6px] border border-line bg-surface px-3 text-ink"
            @input="store.actions.setPlanExportFolder(($event.target as HTMLInputElement).value)"
          />
        </label>
        <p class="mono text-xs break-all text-muted" data-testid="plan-export-target">
          {{ t("common.actual-target") }}{{ target ?? t("common.to-be-filled") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="plan-export-error"
        :title="tFormatPair(`common.export-failed`, { v0: exportState.error.code })"
        :lines="[
          exportState.error.message,
          errorHint ?? t('common.fix-the-issue-and-retry-cancelling-creates'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedPlanExportFolder()">
          {{ tFormatPair(`common.use-suggested-folder-name`, { v0: exportState.suggestedFolder }) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runPlanExport()">{{ t("common.retry-export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="plan-export-success">
        <h3 class="text-sm font-medium">{{ t("common.export-complete") }}</h3>
        <p class="text-[13px] text-muted">
          {{ tFormatPair(`planExportDialog.share-redaction-items`, { v0: exportState.outcome.share ? "开启" : "关闭", v1: checklist.items }) }}
        </p>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <UiButton size="sm" data-testid="plan-export-open-folder" @click="store.actions.openExportFolder(exportState.outcome.output_dir)">
          <FolderOpenDot :size="15" aria-hidden="true" />
          {{ t("common.open-export-folder") }}
        </UiButton>
        <p class="text-xs text-muted">{{ t("planExportDialog.the-html-opens-or-prints-directly") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("planExportDialog.writing-the-checklist") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("common.close") : t("common.cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="plan-export-submit"
        @click="store.actions.runPlanExport()"
      >
        {{ t("common.start-export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
