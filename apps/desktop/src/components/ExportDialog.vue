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
import { FolderOpen, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
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
  { value: "json", label: t("仅 JSON", "JSON only") },
  { value: "csv", label: t("仅 CSV（分享用表格）", "CSV only (shareable table)") },
]);

function close(): void {
  store.actions.closeExportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    :title="t('导出当前快照', 'Export current snapshot')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("导出引用当前报告的快照身份与元数据版本，不覆盖原存档或已有报告。取消不会产生任何文件。", "The export references the current report’s snapshot identity and metadata version and never overwrites the original save or existing reports. Cancelling creates no files.") }}
      </p>

      <CoverageNotice
        v-if="currentResult?.freshness === 'stale'"
        tone="warning"
        test-id="export-stale-notice"
        :title="t('当前是旧快照：导出会携带过期标记', 'The current snapshot is stale: the export will carry a stale marker')"
        :lines="[
          t(`快照时间：${formatSnapshotTime(currentResult.snapshot_time)}`, `Snapshot time: ${formatSnapshotTime(currentResult.snapshot_time)}`),
          t(`刷新错误：${currentResult.last_refresh_error?.code ?? '未知'}`, `Refresh error: ${currentResult.last_refresh_error?.code ?? 'unknown'}`),
          t('产物内标记 stale 与错误码；不代表当前状态。', 'The artifacts are marked stale with the error code; they do not represent the current state.'),
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
        <h3 class="text-sm font-medium">{{ t("分享脱敏", "Share redaction") }}</h3>
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
            {{ t("去除个人绝对路径，保留业务状态、证据与定位精度（默认开启）。", "Remove absolute personal paths while keeping business status, evidence and location precision (enabled by default).") }}
            <span class="block text-xs text-muted">
              {{ t("取消勾选会写入完整本地路径；导出仍不等于内容可公开。", "Unchecking writes full local paths; an export still does not mean the content is safe to publish.") }}
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
            data-testid="export-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setExportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickExportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("选择位置", "Choose location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("新目录名", "New folder name") }}</span>
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
          {{ t("实际目标：", "Actual target: ") }}{{ target ?? t("（待填写）", "(to be filled)") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="export-error"
        :title="t(`导出失败：${exportState.error.code}`, `Export failed: ${exportState.error.code}`)"
        :lines="[
          exportState.error.message,
          errorHint ?? t('可修正后重试；取消不会产生文件。', 'Fix the issue and retry; cancelling creates no files.'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedExportFolder()">
          {{ t(`采用建议目录名 ${exportState.suggestedFolder}`, `Use suggested folder name ${exportState.suggestedFolder}`) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runExport()">{{ t("重试导出", "Retry export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="export-success">
        <h3 class="text-sm font-medium">
          {{ exportState.outcome.freshness === "stale" ? t("导出完成（旧快照，已含过期标记）", "Export complete (stale snapshot, marked as stale)") : t("导出完成", "Export complete") }}
        </h3>
        <p class="text-[13px] text-muted">
          {{ t(`分享脱敏：${exportState.outcome.share ? "开启" : "关闭"}；快照：${formatSnapshotTime(currentResult?.snapshot_time ?? null)}`, `Share redaction: ${exportState.outcome.share ? "on" : "off"}; snapshot: ${formatSnapshotTime(currentResult?.snapshot_time ?? null)}`) }}
        </p>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <p class="text-xs text-muted">{{ t("产物已写入上述目标目录；本应用不会修改或删除既有文件。", "The artifacts were written to the target folder above; this app never modifies or deletes existing files.") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("正在写出导出文件…", "Writing export files…") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("关闭", "Close") : t("取消", "Cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="export-submit"
        @click="store.actions.runExport()"
      >
        {{ t("开始导出", "Start export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
