<script setup lang="ts">
// 分享报告导出（P1-5）：自包含 HTML，固定分享口径。
//
// 不提供“关闭脱敏”选项：报告只含数字与条目清单，不含路径、用户名或存档哈希。
// 地图缩略图在 D7（vi7ual 素材非商业使用边界）决策前不包含。
import { computed } from "vue";
import { FolderOpen, FolderOpenDot, HardDriveDownload } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormatPair } from "../app/i18n";
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
    :title="t('overviewPage.share-report')"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        {{ t("shareReportDialog.generates-a-self-contained-html-with-overall") }}
      </p>

      <section v-if="model" class="rounded-[6px] border border-line bg-raised/40 px-3 py-2 text-[13px]" data-testid="share-report-summary">
        <p class="font-medium text-ink">{{ t("shareReportDialog.report-content") }}</p>
        <p class="mt-0.5 text-muted">
          {{ tFormatPair(`shareReportDialog.main-required-required-missing`, { v0: model.mainCollected, v1: model.mainTotal, v2: model.requiredCollected, v3: model.requiredTotal, v4: model.missingRequired }) }}
        </p>
      </section>

      <CoverageNotice
        test-id="share-report-redaction"
        tone="info"
        :title="t('shareReportDialog.fixed-share-profile')"
        :lines="[
          t('shareReportDialog.no-save-paths-user-names-input-digests'),
          t('shareReportDialog.map-thumbnails-are-not-included-d7'),
        ]"
      />

      <CoverageNotice
        v-if="store.isStale.value"
        tone="warning"
        test-id="share-report-stale"
        :title="t('shareReportDialog.the-current-snapshot-is-stale-the-report')"
        :lines="[t('shareReportDialog.refresh-or-re-analyze-first-or-accept')]"
      />

      <section class="space-y-2">
        <h3 class="text-sm font-medium">{{ t("common.target-location-new-folder-no-overwrite") }}</h3>
        <div class="flex flex-wrap items-center gap-2">
          <input
            :value="exportState.parent ?? ''"
            type="text"
            :placeholder="t('common.choose-the-parent-folder-for-the-target')"
            :aria-label="t('common.target-parent-folder')"
            data-testid="share-report-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setShareReportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickShareReportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            {{ t("common.choose-location") }}
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>{{ t("common.new-folder-name") }}</span>
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
          {{ t("common.actual-target") }}{{ target ?? t("common.to-be-filled") }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="share-report-error"
        :title="tFormatPair(`common.export-failed`, { v0: exportState.error.code })"
        :lines="[
          exportState.error.message,
          errorHint ?? t('common.fix-the-issue-and-retry-cancelling-creates'),
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedShareReportFolder()">
          {{ tFormatPair(`common.use-suggested-folder-name`, { v0: exportState.suggestedFolder }) }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runShareReport()">{{ t("common.retry-export") }}</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="share-report-success">
        <h3 class="text-sm font-medium">{{ t("common.export-complete") }}</h3>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <UiButton size="sm" data-testid="share-report-open-folder" @click="store.actions.openExportFolder(exportState.outcome.output_dir)">
          <FolderOpenDot :size="15" aria-hidden="true" />
          {{ t("common.open-export-folder") }}
        </UiButton>
        <p class="text-xs text-muted">{{ t("shareReportDialog.the-html-opens-in-a-browser") }}</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        {{ t("shareReportDialog.generating-the-share-report") }}
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? t("common.close") : t("common.cancel") }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="share-report-submit"
        @click="store.actions.runShareReport()"
      >
        {{ t("common.start-export") }}
      </UiButton>
    </template>
  </UiDialog>
</template>
