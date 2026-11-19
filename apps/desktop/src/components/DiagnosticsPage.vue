<script setup lang="ts">
// 诊断（SX-015；2026-11-05 界面美化阶段五）：输入与解析 / 支持能力 / 计数与参考 / 定位与元数据；
// 未知 ID 可见但不把应用变成十六进制浏览器；每条关联当前槽位。
// 视觉：统一页头与 15px 区块标题，技术信息使用同一套键值 / 表格工作面。
import { computed } from "vue";
import { CircleAlert, Info, TriangleAlert } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat, tFormatPair } from "../app/i18n";
import { capabilityLabel, categoryLabel, copy, reasonLabel, severityLabel } from "../app/labels";
import { formatBytes, formatSnapshotTime } from "../app/format";
import { pagePaddingClass } from "../app/layout";
import { regionName } from "../app/collection";
import { diagnosticGroups, unclassifiedRecords } from "../app/catalog";
import { catalogScopeNotes, productionCatalog } from "../data/catalog";
import AppPageHeader from "./AppPageHeader.vue";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiEvidenceDetails from "./ui/UiEvidenceDetails.vue";

const store = useStore();
const report = computed(() => store.report.value);
const slot = computed(() => store.currentSlot.value);
const layout = computed(() => store.layout.value);
const pageX = computed(() => pagePaddingClass(layout.value));

const slotContext = computed(() => {
  if (slot.value?.name) {
    return tFormatPair(`diagnosticsPage.current-slot-snapshot`, { v0: slot.value.name, v1: formatSnapshotTime(slot.value.result?.snapshot_time ?? null) });
  }
  return t("diagnosticsPage.no-slot-selected");
});

const capabilityRows = computed(() => {
  if (!report.value) {
    return [];
  }
  const capabilities = report.value.capabilities;
  return [
    { key: "container", label: t("diagnosticsPage.container-zlib"), capability: capabilities.container },
    { key: "sections", label: t("diagnosticsPage.section-stream"), capability: capabilities.sections },
    { key: "inventory", label: t("diagnosticsPage.inventory-table"), capability: capabilities.inventory },
    { key: "counters", label: t("diagnosticsPage.region-counters"), capability: capabilities.counters },
    { key: "reference_difference", label: t("diagnosticsPage.reference-difference"), capability: capabilities.reference_difference },
    { key: "location", label: t("diagnosticsPage.location"), capability: capabilities.location },
  ];
});

const groups = computed(() => (report.value ? diagnosticGroups(report.value) : []));
const records = computed(() => (report.value ? unclassifiedRecords(store.metadata, report.value) : []));

/** 图鉴口径（P2-5）：特殊区域与挑战古墓共用 ID 的静态说明。 */
const scopeNotes = catalogScopeNotes(productionCatalog);

// 当前选中条目的原始证据（从详情面板移入；详情只保留「如何获取」与外部参考）。
const selected = computed(() => store.selectedEntry.value);
const selectedRegionRecord = computed(() => {
  const region = selected.value?.region;
  return region ? (store.atlas.regions.find((entry) => entry.region_hash === region.region_hash) ?? null) : null;
});
const selectedCategoryName = computed(() => {
  const category = selected.value?.group.category;
  return category ? categoryLabel(category, store.atlas.categories.find((entry) => entry.id === category)?.name ?? null) : null;
});

/** 诊断条目的状态图标：形状 + 文字，不单独依赖颜色。 */
function severityIcon(severity: "info" | "warning" | "error") {
  return severity === "error" ? CircleAlert : severity === "warning" ? TriangleAlert : Info;
}

function severityClass(severity: "info" | "warning" | "error"): string {
  return severity === "error" ? "text-danger" : severity === "warning" ? "text-caution" : "text-info";
}
</script>

<template>
  <section class="scroll-area min-h-0 flex-1" aria-labelledby="diagnostics-title">
    <div class="mx-auto w-full max-w-[960px] pb-8" :class="pageX">
      <AppPageHeader
        title-id="diagnostics-title"
        :title="t('common.diagnostics')"
        :context="slotContext"
        :description="
          t('diagnosticsPage.input-and-parsing-supported-capabilities-counters')
        "
      />

      <UiEmptyState
        v-if="!report"
        class="mt-4"
        :title="copy.noReport"
        :description="t('diagnosticsPage.diagnostics-need-a-current-report-analyze')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("common.open-a-save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          class="mt-4"
          tone="warning"
          :title="copy.stale"
          :lines="[t('diagnosticsPage.diagnostics-also-keep-the-stale-snapshot-s')]"
        />

        <div class="mt-6 space-y-6">
          <section class="space-y-2" data-testid="diagnostics-input">
            <h2 class="ui-section-title">{{ t("catalog.input-parsing") }}</h2>
            <dl class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("diagnosticsPage.input-digest-compressed") }}</dt>
                <dd class="min-w-0 flex-1 break-all">
                  <span class="mono">{{ report.input_digest.compressed_sha256.slice(0, 24) }}…</span>
                  （{{ formatBytes(report.input_digest.compressed_bytes) }}）
                </dd>
              </div>
              <div v-if="report.input_digest.expanded_sha256" class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("diagnosticsPage.input-digest-expanded") }}</dt>
                <dd class="min-w-0 flex-1 break-all">
                  <span class="mono">{{ report.input_digest.expanded_sha256.slice(0, 24) }}…</span>
                  （{{ formatBytes(report.input_digest.expanded_bytes) }}）
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("diagnosticsPage.context") }}</dt>
                <dd class="min-w-0 flex-1 break-words">
                  mode={{ report.context.mode }}（source={{ report.context.source }}）；platform={{ report.context.platform ?? t("app.unknown") }}；
                  build={{ report.context.build ?? t("app.unknown") }}；DLC={{ report.context.dlc?.join(t("common.list-separator")) ?? t("app.unknown") }}
                  <ul v-if="report.context.limitations.length" class="mt-1 list-inside list-disc text-muted">
                    <li v-for="limitation in report.context.limitations" :key="limitation">{{ limitation }}</li>
                  </ul>
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("diagnosticsPage.container-records") }}</dt>
                <dd class="min-w-0 flex-1 break-words">
                  {{ report.observations.container.valid ? t("diagnosticsPage.valid") : t("diagnosticsPage.invalid") }}；{{ t("diagnosticsPage.sections") }} {{ report.observations.sections.section_count }}；
                  {{ t("diagnosticsPage.raw-records") }} {{ report.observations.inventory.record_count }}（{{ t("diagnosticsPage.unique") }} {{ report.observations.inventory.unique_id_count }}，{{ t("diagnosticsPage.duplicates") }}
                  {{ report.observations.inventory.duplicate_record_count }}）
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("diagnosticsPage.completeness-scope") }}</dt>
                <dd class="min-w-0 flex-1 break-words text-muted">{{ report.observations.inventory.completeness_scope }}</dd>
              </div>
            </dl>
          </section>

          <section class="space-y-2" data-testid="diagnostics-capabilities">
            <h2 class="ui-section-title">{{ t("catalog.supported-capabilities") }}</h2>
            <div class="ui-panel overflow-x-auto">
              <table class="w-full min-w-[560px] border-collapse text-[13px]">
                <caption class="sr-only">{{ t("diagnosticsPage.capability-matrix") }}</caption>
                <thead>
                  <tr class="h-9 border-b border-line bg-raised/50 text-left text-muted">
                    <th scope="col" class="px-3 font-normal">{{ t("diagnosticsPage.capability") }}</th>
                    <th scope="col" class="px-3 font-normal">{{ t("common.status") }}</th>
                    <th scope="col" class="px-3 font-normal">{{ t("diagnosticsPage.reason-codes") }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in capabilityRows" :key="row.key" class="h-11 border-b border-line/60 last:border-b-0">
                    <td class="px-3 py-2 align-top">{{ row.label }}</td>
                    <td class="px-3 py-2 align-top">
                      <UiBadge :tone="row.capability.status === 'supported' ? 'success' : row.capability.status === 'partial' ? 'caution' : 'danger'" :kind-label="t('diagnosticsPage.capability')">
                        {{ capabilityLabel(row.capability.status) }}
                      </UiBadge>
                    </td>
                    <td class="px-3 py-2 align-top break-words text-muted">
                      {{ row.capability.reason_codes.map(reasonLabel).join(t("common.list-separator")) || "—" }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section class="space-y-2" data-testid="diagnostics-counters">
            <h2 class="ui-section-title">{{ t("catalog.counters-reference") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="cell in report.observations.region_counters" :key="cell.counter_key" class="px-3 py-2 break-words">
                <span class="mono">{{ cell.counter_key }}</span>：{{ t("diagnosticsPage.candidate-reading") }} {{ cell.value ?? "null" }}；{{ t("diagnosticsPage.source-target") }} {{ cell.source_target ?? t("app.unknown") }}；{{ t("diagnosticsPage.raw-reading") }}
                {{ cell.raw_value }}（<span class="mono">{{ cell.raw_value_hex }}</span>）
                <span v-if="!cell.within_source_target" class="text-caution"> · {{ copy.countConflict }}</span>
              </li>
            </ul>
            <p class="text-[13px] leading-5 break-words text-muted">
              {{ t("diagnosticsPage.reference-coverage-categories") }}{{ report.coverage.reference_category_ids.join(t("common.list-separator")) || t("diagnosticsPage.none") }}；{{ t("diagnosticsPage.unmet-reasons") }}{{ (report.coverage.unavailable_reason_codes ?? []).map(reasonLabel).join(t("common.list-separator")) || "—" }}
            </p>
          </section>

          <section class="space-y-2" data-testid="diagnostics-catalog-scope">
            <h2 class="ui-section-title">{{ t("diagnosticsPage.catalog-scope") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li class="px-3 py-2 break-words">
                <span class="font-medium">{{ t("diagnosticsPage.special-region-special") }}</span>：
                {{ tFormat("diagnosticsPage.items-with-a-sub-level-without", 
                  { total: scopeNotes.specialTotal, withUnit: scopeNotes.specialWithUnit, withoutUnit: scopeNotes.specialWithoutUnit }) }}
                {{ t("diagnosticsPage.these-are-cross-region-collection-items-path") }}
              </li>
              <li v-if="scopeNotes.sharedChallengeId" class="px-3 py-2 break-words">
                <span class="font-medium">{{ t("diagnosticsPage.shared-challenge-tomb-id") }}</span>：
                {{ tFormat("diagnosticsPage.ids-names-names-share", 
                  { ids: scopeNotes.challengeTombIds, names: scopeNotes.challengeTombNames, shared: scopeNotes.sharedChallengeNames.length }) }}
                <span class="mono">{{ scopeNotes.sharedChallengeId }}</span>
                （{{ scopeNotes.sharedChallengeNames.join(" / ") }}）。
                {{ t("diagnosticsPage.counted-by-id-without-per-name-splitting") }}
              </li>
            </ul>
          </section>

          <section class="space-y-2" data-testid="diagnostics-locations">
            <h2 class="ui-section-title">{{ t("catalog.location-metadata") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="location in report.locations" :key="location.location_id" class="px-3 py-2 break-words">
                <span class="mono">{{ location.location_id }}</span>：{{ t("diagnosticsPage.precision") }} {{ location.precision }}；{{ t("diagnosticsPage.members") }} {{ location.member_ids.length }}；{{ t("diagnosticsPage.gate") }}
                {{ location.group_actionability?.kind ?? t("diagnosticsPage.none-not-a-group-level-clue") }}
              </li>
            </ul>
            <p class="text-[13px] leading-5 break-words text-muted">
              {{ t("diagnosticsPage.metadata-version") }}{{ report.metadata_version ?? t("app.unknown") }}；{{ t("diagnosticsPage.reference-set-version") }}{{ report.reference_set_version ?? t("app.unknown") }}；{{ t("diagnosticsPage.parser-profile") }}
              <span class="mono">{{ report.parser_profile_id }}</span>
            </p>
          </section>

          <section class="space-y-2" data-testid="selected-evidence">
            <h2 class="ui-section-title">{{ t("diagnosticsPage.raw-evidence-for-the-selected-item") }}</h2>
            <p v-if="!selected" class="text-[13px] text-muted">
              {{ t("diagnosticsPage.no-item-selected-choose-an-item") }}
            </p>
            <UiEvidenceDetails
              v-else
              :summary="t('diagnosticsPage.raw-evidence-and-dataset-information')"
              :hint="t('diagnosticsPage.catalog-version-region-binding-sub-level-coordinates')"
            >
              <p>
                {{ t("diagnosticsPage.item") }}{{ selected.item.display_name }}（<span class="mono">{{ selected.item.id }}</span>）
              </p>
              <p>{{ t("diagnosticsPage.catalog-version") }}{{ store.atlas.catalog_version }}（schema {{ store.atlas.catalog_schema_version }}）</p>
              <p>
                {{ t("diagnosticsPage.region") }}{{ regionName(selected.region) }}（<span class="mono">{{ selected.region.region_hash }}</span>）<span
                  v-if="selectedRegionRecord?.binding"
                >
                  · {{ tFormatPair(`diagnosticsPage.binding-signature-distance-runner-up`, { v0: selectedRegionRecord.binding.signature_distance, v1: selectedRegionRecord.binding.runner_up_distance ?? "无" }) }}
                </span>
              </p>
              <p v-if="selectedRegionRecord?.binding">
                {{ t("diagnosticsPage.counter-check") }}<span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.signature) }}</span>
                vs {{ t("diagnosticsPage.save-target") }} <span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.source_targets) }}</span>
              </p>
              <p v-if="selected.item.unit">
                {{ t("diagnosticsPage.sub-level") }}<span class="mono">{{ selected.item.unit.unit }}</span> · zone
                <span class="mono">{{ selected.item.unit.zone ?? t("app.unknown") }}</span>（{{ t("diagnosticsPage.name-source") }} {{ selected.item.unit.name_source }}）
              </p>
              <p v-if="selected.item.position">
                {{ t("diagnosticsPage.raw-coordinates") }}x={{ selected.item.position.x }}，y={{ selected.item.position.y }}，{{ t("diagnosticsPage.space") }}
                <span class="mono">{{ selected.item.position.space }}</span>（{{ t("diagnosticsPage.uncalibrated") }}）
              </p>
              <p>{{ t("diagnosticsPage.internal-candidate-name") }}<span class="mono">{{ selected.item.internal_name ?? t("diagnosticsPage.none-2") }}</span></p>
              <p v-if="selected.item.collection">{{ t("diagnosticsPage.story-collection") }}<span class="mono">{{ selected.item.collection }}</span></p>
              <p>
                {{ t("diagnosticsPage.category") }}{{ selectedCategoryName }}<template v-if="selected.item.name_source"> · {{ t("diagnosticsPage.name-source") }} {{ selected.item.name_source }}</template>
              </p>
              <p>{{ t("diagnosticsPage.acquisition-rule") }}<span class="mono">{{ selected.group.acquisition_rule_key }}</span></p>
              <p data-testid="selected-status-reason">
                <template v-if="selected.item.status === 'collected'">
                  {{ t("diagnosticsPage.this-item-id-appears-in-the-current") }}
                </template>
                <template v-else>
                  {{ t("diagnosticsPage.this-item-id-does-not-appear") }}
                </template>
              </p>
              <p v-if="store.isStale.value">{{ copy.stale }}：{{ t("diagnosticsPage.this-result-reflects-only-the-last-read") }}</p>
            </UiEvidenceDetails>
          </section>

          <section class="space-y-2" data-testid="diagnostics-entries">
            <h2 class="ui-section-title">{{ t("diagnosticsPage.diagnostic-entries") }}</h2>
            <div v-for="group in groups" :key="group.title" class="space-y-1.5">
              <h3 class="text-[13px] font-medium text-muted">{{ group.title }}</h3>
              <ul class="ui-panel divide-y divide-line/60">
                <li v-for="item in group.items" :key="`${item.code}-${item.message}`" class="flex items-start gap-2 px-3 py-2 text-[13px] leading-5">
                  <component :is="severityIcon(item.severity)" :size="16" :stroke-width="2" class="mt-0.5 shrink-0" :class="severityClass(item.severity)" aria-hidden="true" />
                  <div class="min-w-0 flex-1 break-words">
                    <span class="font-medium" :class="severityClass(item.severity)">{{ severityLabel(item.severity) }}</span>
                    · <span class="mono break-all">{{ item.code }}</span>：{{ item.message }}
                    <span v-if="item.evidence_refs.length" class="block text-xs text-muted">（{{ t("diagnosticsPage.evidence") }}{{ item.evidence_refs.join(t("common.list-separator")) }}）</span>
                  </div>
                </li>
              </ul>
            </div>
            <p v-if="groups.length === 0" class="text-[13px] text-muted">{{ t("diagnosticsPage.no-diagnostic-entries-affect-operation") }}</p>
          </section>

          <section class="space-y-2" data-testid="diagnostics-observed">
            <h2 class="ui-section-title">{{ t("diagnosticsPage.observed-records-unclassified-not-counted-as-collectibles") }}</h2>
            <ul v-if="records.length" class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="record in records" :key="`${record.table}-${record.index}`" class="px-3 py-2 break-words">
                <span class="mono">{{ record.internal_id }}</span> · {{ t("diagnosticsPage.table") }} {{ record.table }} #{{ record.index }} · {{ t("diagnosticsPage.companion") }}
                <span class="mono">{{ record.raw_companion_hex }}</span>（{{ t("diagnosticsPage.low-two-bits") }} {{ record.raw_low_two_bits }}） · {{ t("diagnosticsPage.offset") }}
                <span class="mono">{{ record.id_offset_hex }}</span>
              </li>
            </ul>
            <p v-else class="text-[13px] text-muted">{{ t("diagnosticsPage.no-unclassified-records") }}</p>
          </section>
        </div>
      </template>
    </div>
  </section>
</template>
