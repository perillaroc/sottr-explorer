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

const formatOptions: { value: "json" | "csv" | "both"; label: string }[] = [
  { value: "both", label: "JSON + CSV" },
  { value: "json", label: "仅 JSON" },
  { value: "csv", label: "仅 CSV（分享用表格）" },
];

function close(): void {
  store.actions.closeExportDialog();
}
</script>

<template>
  <UiDialog
    v-if="exportState.open"
    :open="true"
    title="导出当前快照"
    @close="close"
  >
    <div class="space-y-4">
      <p class="text-[13px] text-muted">
        导出引用当前报告的快照身份与元数据版本，不覆盖原存档或已有报告。取消不会产生任何文件。
      </p>

      <CoverageNotice
        v-if="currentResult?.freshness === 'stale'"
        tone="warning"
        test-id="export-stale-notice"
        title="当前是旧快照：导出会携带过期标记"
        :lines="[
          `快照时间：${formatSnapshotTime(currentResult.snapshot_time)}`,
          `刷新错误：${currentResult.last_refresh_error?.code ?? '未知'}`,
          '产物内标记 stale 与错误码；不代表当前状态。',
        ]"
      />

      <section class="space-y-2">
        <h3 class="text-sm font-medium">格式</h3>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" aria-label="导出格式">
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
        <h3 class="text-sm font-medium">分享脱敏</h3>
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
            去除个人绝对路径，保留业务状态、证据与定位精度（默认开启）。
            <span class="block text-xs text-muted">
              取消勾选会写入完整本地路径；导出仍不等于内容可公开。
            </span>
          </span>
        </label>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-medium">目标位置（新目录，不覆盖）</h3>
        <div class="flex flex-wrap items-center gap-2">
          <input
            :value="exportState.parent ?? ''"
            type="text"
            placeholder="选择目标位置的父目录"
            aria-label="目标位置父目录"
            data-testid="export-parent"
            :disabled="busy"
            class="h-9 min-w-64 flex-1 rounded-[6px] border border-line bg-surface px-3 text-[13px] text-ink mono"
            @input="store.actions.setExportParent(($event.target as HTMLInputElement).value || null)"
          />
          <UiButton :disabled="busy" @click="store.actions.pickExportParent()">
            <FolderOpen :size="16" aria-hidden="true" />
            选择位置
          </UiButton>
        </div>
        <label class="block space-y-1 text-[13px]">
          <span>新目录名</span>
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
          实际目标：{{ target ?? "（待填写）" }}
        </p>
      </section>

      <CoverageNotice
        v-if="exportState.status === 'error' && exportState.error"
        tone="danger"
        test-id="export-error"
        :title="`导出失败：${exportState.error.code}`"
        :lines="[
          exportState.error.message,
          errorHint ?? '可修正后重试；取消不会产生文件。',
        ]"
      >
        <UiButton v-if="exportState.suggestedFolder" size="sm" @click="store.actions.applySuggestedExportFolder()">
          采用建议目录名 {{ exportState.suggestedFolder }}
        </UiButton>
        <UiButton size="sm" variant="ghost" :disabled="busy" @click="store.actions.runExport()">重试导出</UiButton>
      </CoverageNotice>

      <section v-if="exportState.status === 'success' && exportState.outcome" class="space-y-1" data-testid="export-success">
        <h3 class="text-sm font-medium">
          导出完成{{ exportState.outcome.freshness === "stale" ? "（旧快照，已含过期标记）" : "" }}
        </h3>
        <p class="text-[13px] text-muted">
          分享脱敏：{{ exportState.outcome.share ? "开启" : "关闭" }}；快照：{{
            formatSnapshotTime(currentResult?.snapshot_time ?? null)
          }}
        </p>
        <ul class="space-y-0.5 text-[13px]">
          <li v-for="artifact in exportState.outcome.artifacts" :key="artifact.path" class="mono break-all">
            {{ artifact.name }}：{{ artifact.path }}
          </li>
        </ul>
        <p class="text-xs text-muted">产物已写入上述目标目录；本应用不会修改或删除既有文件。</p>
      </section>
    </div>

    <template #footer>
      <span v-if="busy" class="mr-auto flex items-center gap-2 text-[13px] text-muted">
        <HardDriveDownload :size="16" aria-hidden="true" />
        正在写出导出文件…
      </span>
      <UiButton :disabled="busy" @click="close">{{ exportState.status === "success" ? "关闭" : "取消" }}</UiButton>
      <UiButton
        variant="primary"
        :disabled="busy || exportState.status === 'success'"
        data-testid="export-submit"
        @click="store.actions.runExport()"
      >
        开始导出
      </UiButton>
    </template>
  </UiDialog>
</template>
