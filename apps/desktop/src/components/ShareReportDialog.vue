<script setup lang="ts">
// 分享报告导出（P1-5）：自包含 HTML，固定分享口径。
//
// 不提供“关闭脱敏”选项：报告只含数字与条目清单，不含路径、用户名或存档哈希。
// 地图缩略图在 D7（vi7ual 素材非商业使用边界）决策前不包含。
import { computed } from "vue";
import { FolderOpen, FolderOpenDot, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { recoveryHint } from "../data/errors";
import { shareReportModel } from "../app/share-report";
import UiButton from "./ui/UiButton.vue";
import UiDialog from "./ui/UiDialog.vue";
import CoverageNotice from "./CoverageNotice.vue";

const store = useStore();

const exportState = computed(() => store.state.shareReportState);
const target = computed(() => store.shareReportTargetPath.value);
const busy = computed(() => exportState.value.status === "exporting");
const model = computed(() =>
  store.report.value ? shareReportModel(store.plan.value, store.report.value) : null,
);
const errorHint = computed(() =>
  exportState.value.error ? recoveryHint(exportState.value.error) : null,
);

function close(): void {
  store.actions.closeShareReportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    :title="t('分享报告', 'Share report')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("生成一个自包含的 HTML：总进度、区域进度与必收缺失清单，离线可读，可直接发给别人。写入新目录，不覆盖原存档或已有报告。", "Generates a self-contained HTML with overall progress, region progress and the required missing list. It reads offline and can be shared directly. It is written to a new folder and never overwrites the save or existing reports.") }}
      </p>

      <section v-if="model" class="rounded-[6px] border border-line bg-raised/40 px-3 py-2 text-[13px]" data-testid="share-report-summary">
        <p class="font-medium text-ink">{{ t("报告内容", "Report content") }}</p>
        <p class="mt-0.5 text-muted">
          {{ t(`主区域 ${model.mainCollected}/${model.mainTotal} · 必收 ${model.requiredCollected}/${model.requiredTotal} · 缺失必收 ${model.missingRequired}`, `Main ${model.mainCollected}/${model.mainTotal} · required ${model.requiredCollected}/${model.requiredTotal} · required missing ${model.missingRequired}`) }}
        </p>
      </section>

      <CoverageNotice
        test-id="share-report-redaction"
        tone="info"
        :title="t('固定分享口径', 'Fixed share profile')"
        :lines="[
          t('不含存档路径、用户名、输入摘要/哈希与快照时间。', 'No save paths, user names, input digests/hashes or snapshot times.'),
          t('不包含地图缩略图：D7 已决定暂不包含（2026-11-16）。', 'Map thumbnails are not included: D7 was decided as “not for now” (2026-11-16).'),
        ]"
      />

      <CoverageNotice
        v-if="store.isStale.value"
        tone="warning"
        test-id="share-report-stale"
        :title="t('当前是旧快照：报告可能不反映最新状态', 'The current snapshot is stale: the report may not reflect the latest state')"
        :lines="[t('请先刷新/重新分析，或接受报告可能过期。', 'Refresh or re-analyze first, or accept that the report may be out of date.')]"
      />

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("目标位置（新目录，不覆盖）", "Target location (new folder, no overwrite)") }}</h3>
        <div class="flex flex-wrap items-center gap-2">
          <input
            :value="exportState.parent ?? ''"
            type="text"
            :placeholder="t('选择目标位置的父目录', 'Choose the parent folder for the target')"
            :aria-label="t('目标位置父目录', 'Target parent folder')"
            data-testid="share-report-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setShareReportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickShareReportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("选择位置", "Choose location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("新目录名", "New folder name") }}</span>
          <input
            :value="exportState.folder"
            type="text"
            data-testid="share-report-folder"
            :disabled="busy"
            class="mono h-9 w-full rounded-[6px] border border-line bg-surface px-3 text-ink"
            @input="store.actions.setShareReportFolder(($event.target as HTMLInputElement).value)"
          />
        </label>
        <p class="mono text-xs break-all text-muted" data-testid="share-report-target">
          {{ t("实际目标：", "Actual target: ") }}{{ target ?? t("（待填写）", "(to be filled)") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="share-report-error"
        :title="t(`导出失败：${exportState.error.code}`, `Export failed: ${exportState.error.code}`)"
        :lines="[
          exportState.error.message,
          errorHint ?? t('可修正后重试；取消不会产生文件。', 'Fix the issue and retry; cancelling creates no files.'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedShareReportFolder()">
          {{ t(`采用建议目录名 ${exportState.suggestedFolder}`, `Use suggested folder name ${exportState.suggestedFolder}`) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runShareReport()">{{ t("重试导出", "Retry export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="share-report-success">
        <h3 class="text-sm font-medium">{{ t("导出完成", "Export complete") }}</h3>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <UiButton size="sm" data-testid="share-report-open-folder" @click="store.actions.openExportFolder(exportState.outcome.output_dir)">
          <FolderOpenDot :size="15" aria-hidden="true" />
          {{ t("打开导出目录", "Open export folder") }}
        </UiButton>
        <p class="text-xs text-muted">{{ t("HTML 可直接用浏览器打开或发送；本应用不会修改或删除既有文件。", "The HTML opens in a browser or can be sent as-is; this app never modifies or deletes existing files.") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("正在生成分享报告…", "Generating the share report…") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("关闭", "Close") : t("取消", "Cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="share-report-submit"
        @click="store.actions.runShareReport()"
      >
        {{ t("开始导出", "Start export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
