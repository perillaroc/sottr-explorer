<script setup lang="ts">
// 收集计划清单导出（P1-2）：可打印 HTML / CSV、分享脱敏、目标新目录。
//
// 边界与报告导出（SX-016）一致：无报告或分析未完成时不能导出；目标必须是
// 尚不存在的新目录；取消不产生任何文件；成功后显示实际产物与目录。
// 导出内容按**当前筛选**生成，对话框里显式写出范围，避免“以为导出了全部”。
import { computed } from "vue";
import { FolderOpen, FolderOpenDot, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
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
  { value: "both", label: t("HTML + CSV", "HTML + CSV") },
  { value: "html", label: t("仅 HTML（可打印清单）", "HTML only (printable checklist)") },
  { value: "csv", label: t("仅 CSV（表格）", "CSV only (table)") },
]);

function close(): void {
  store.actions.closePlanExportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    :title="t('导出收集清单', 'Export collection checklist')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("导出的是当前筛选下的未收集条目；写入新目录，不覆盖原存档或已有报告。取消不会产生任何文件。", "The export contains uncollected items under the current filters. It is written to a new folder and never overwrites the save or existing reports. Cancelling creates no files.") }}
      </p>

      <section class="rounded-[6px] border border-line bg-raised/40 px-3 py-2 text-[13px]">
        <p class="font-medium text-ink">{{ t("本次范围", "Current scope") }}</p>
        <p class="mt-0.5 text-muted" data-testid="plan-export-scope">{{ store.planScopeText.value }}</p>
        <p class="mt-0.5 text-xs text-muted" data-testid="plan-export-count">
          {{ t(`未收集 ${checklist.items} 项（其中必收 ${checklist.required}）`, `${checklist.items} uncollected (${checklist.required} required)`) }}
        </p>
      </section>

      <CoverageNotice
        v-if="currentResult?.freshness === 'stale'"
        tone="warning"
        test-id="plan-export-stale-notice"
        :title="t('当前是旧快照：清单可能不反映最新状态', 'The current snapshot is stale: the checklist may not reflect the latest state')"
        :lines="[
          t(`快照时间：${formatSnapshotTime(currentResult.snapshot_time)}`, `Snapshot time: ${formatSnapshotTime(currentResult.snapshot_time)}`),
          t('请先刷新/重新分析，或接受清单可能过期。', 'Refresh or re-analyze first, or accept that the checklist may be out of date.'),
        ]"
      />

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("格式", "Format") }}</h3>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" :aria-label="t('导出格式', 'Export format')">
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
        <h3 class="text-sm font-medium">{{ t("分享脱敏", "Share redaction") }}</h3>
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
            {{ t("不写输入文件名、快照时间与任何路径，产物可直接发给别人（默认开启）。", "Omit the input file name, snapshot time and all paths so the artifacts can be shared directly (enabled by default).") }}
            <span class="block text-xs text-muted">
              {{ t("取消勾选会写入存档文件名与快照时间；导出仍不等于内容可公开。", "Unchecking writes the save file name and snapshot time; an export still does not mean the content is safe to publish.") }}
            </span>
          </span>
        </label>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("目标位置（新目录，不覆盖）", "Target location (new folder, no overwrite)") }}</h3>
        <div class="flex flex-wrap items-center gap-2">
          <input
            :value="exportState.parent ?? ''"
            type="text"
            :placeholder="t('选择目标位置的父目录', 'Choose the parent folder for the target')"
            :aria-label="t('目标位置父目录', 'Target parent folder')"
            data-testid="plan-export-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setPlanExportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickPlanExportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("选择位置", "Choose location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("新目录名", "New folder name") }}</span>
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
          {{ t("实际目标：", "Actual target: ") }}{{ target ?? t("（待填写）", "(to be filled)") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="plan-export-error"
        :title="t(`导出失败：${exportState.error.code}`, `Export failed: ${exportState.error.code}`)"
        :lines="[
          exportState.error.message,
          errorHint ?? t('可修正后重试；取消不会产生文件。', 'Fix the issue and retry; cancelling creates no files.'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedPlanExportFolder()">
          {{ t(`采用建议目录名 ${exportState.suggestedFolder}`, `Use suggested folder name ${exportState.suggestedFolder}`) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runPlanExport()">{{ t("重试导出", "Retry export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="plan-export-success">
        <h3 class="text-sm font-medium">{{ t("导出完成", "Export complete") }}</h3>
        <p class="text-[13px] text-muted">
          {{ t(`分享脱敏：${exportState.outcome.share ? "开启" : "关闭"}；条目数：${checklist.items}`, `Share redaction: ${exportState.outcome.share ? "on" : "off"}; items: ${checklist.items}`) }}
        </p>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <UiButton size="sm" data-testid="plan-export-open-folder" @click="store.actions.openExportFolder(exportState.outcome.output_dir)">
          <FolderOpenDot :size="15" aria-hidden="true" />
          {{ t("打开导出目录", "Open export folder") }}
        </UiButton>
        <p class="text-xs text-muted">{{ t("HTML 可直接用浏览器打开或打印；本应用不会修改或删除既有文件。", "The HTML opens or prints directly in a browser; this app never modifies or deletes existing files.") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("正在写出清单…", "Writing the checklist…") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("关闭", "Close") : t("取消", "Cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="plan-export-submit"
        @click="store.actions.runPlanExport()"
      >
        {{ t("开始导出", "Start export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
