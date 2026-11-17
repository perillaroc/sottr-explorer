<script setup lang="ts">
// 诊断（SX-015；2026-11-05 界面美化阶段五）：输入与解析 / 支持能力 / 计数与参考 / 定位与元数据；
// 未知 ID 可见但不把应用变成十六进制浏览器；每条关联当前槽位。
// 视觉：统一页头与 15px 区块标题，技术信息使用同一套键值 / 表格工作面。
import { computed } from "vue";
import { CircleAlert, Info, TriangleAlert } from "@lucide/vue";
import { useStore } from "../app/inject";
import { t, tFormat } from "../app/i18n";
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
    return t(
      `当前槽位：${slot.value.name} · 快照 ${formatSnapshotTime(slot.value.result?.snapshot_time ?? null)}`,
      `Current slot: ${slot.value.name} · snapshot ${formatSnapshotTime(slot.value.result?.snapshot_time ?? null)}`,
    );
  }
  return t("未选择槽位", "No slot selected");
});

const capabilityRows = computed(() => {
  if (!report.value) {
    return [];
  }
  const capabilities = report.value.capabilities;
  return [
    { key: "container", label: t("容器（zlib）", "Container (zlib)"), capability: capabilities.container },
    { key: "sections", label: t("区段流", "Section stream"), capability: capabilities.sections },
    { key: "inventory", label: t("物品表", "Inventory table"), capability: capabilities.inventory },
    { key: "counters", label: t("区域计数", "Region counters"), capability: capabilities.counters },
    { key: "reference_difference", label: t("参考差集", "Reference difference"), capability: capabilities.reference_difference },
    { key: "location", label: t("定位", "Location"), capability: capabilities.location },
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
        :title="t('诊断', 'Diagnostics')"
        :context="slotContext"
        :description="
          t(
            '输入与解析、支持能力、计数与参考、定位与元数据；原始证据默认折叠。',
            'Input and parsing, supported capabilities, counters and reference, location and metadata; raw evidence is collapsed by default.',
          )
        "
      />

      <UiEmptyState
        v-if="!report"
        class="mt-4"
        :title="copy.noReport"
        :description="t('诊断需要当前报告；请先分析一个存档文件。', 'Diagnostics need a current report; analyze a save file first.')"
      >
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          class="mt-4"
          tone="warning"
          :title="copy.stale"
          :lines="[t('诊断同时保留旧快照的输入摘要与本次刷新错误。', 'Diagnostics also keep the stale snapshot’s input digest and this refresh error.')]"
        />

        <div class="mt-6 space-y-6">
          <section class="space-y-2" data-testid="diagnostics-input">
            <h2 class="ui-section-title">{{ t("输入与解析", "Input & parsing") }}</h2>
            <dl class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("输入摘要（压缩）", "Input digest (compressed)") }}</dt>
                <dd class="min-w-0 flex-1 break-all">
                  <span class="mono">{{ report.input_digest.compressed_sha256.slice(0, 24) }}…</span>
                  （{{ formatBytes(report.input_digest.compressed_bytes) }}）
                </dd>
              </div>
              <div v-if="report.input_digest.expanded_sha256" class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("输入摘要（解压）", "Input digest (expanded)") }}</dt>
                <dd class="min-w-0 flex-1 break-all">
                  <span class="mono">{{ report.input_digest.expanded_sha256.slice(0, 24) }}…</span>
                  （{{ formatBytes(report.input_digest.expanded_bytes) }}）
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("上下文", "Context") }}</dt>
                <dd class="min-w-0 flex-1 break-words">
                  mode={{ report.context.mode }}（source={{ report.context.source }}）；platform={{ report.context.platform ?? t("未知", "unknown") }}；
                  build={{ report.context.build ?? t("未知", "unknown") }}；DLC={{ report.context.dlc?.join(t("、", ", ")) ?? t("未知", "unknown") }}
                  <ul v-if="report.context.limitations.length" class="mt-1 list-inside list-disc text-muted">
                    <li v-for="limitation in report.context.limitations" :key="limitation">{{ limitation }}</li>
                  </ul>
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("容器与记录", "Container & records") }}</dt>
                <dd class="min-w-0 flex-1 break-words">
                  {{ report.observations.container.valid ? t("有效", "valid") : t("无效", "invalid") }}；{{ t("区段数", "Sections") }} {{ report.observations.sections.section_count }}；
                  {{ t("原始记录", "Raw records") }} {{ report.observations.inventory.record_count }}（{{ t("唯一", "unique") }} {{ report.observations.inventory.unique_id_count }}，{{ t("重复", "duplicates") }}
                  {{ report.observations.inventory.duplicate_record_count }}）
                </dd>
              </div>
              <div class="flex flex-wrap gap-x-3 gap-y-1 px-3 py-2">
                <dt class="w-32 shrink-0 text-muted">{{ t("完整性口径", "Completeness scope") }}</dt>
                <dd class="min-w-0 flex-1 break-words text-muted">{{ report.observations.inventory.completeness_scope }}</dd>
              </div>
            </dl>
          </section>

          <section class="space-y-2" data-testid="diagnostics-capabilities">
            <h2 class="ui-section-title">{{ t("支持能力", "Supported capabilities") }}</h2>
            <div class="ui-panel overflow-x-auto">
              <table class="w-full min-w-[560px] border-collapse text-[13px]">
                <caption class="sr-only">{{ t("能力矩阵", "Capability matrix") }}</caption>
                <thead>
                  <tr class="h-9 border-b border-line bg-raised/50 text-left text-muted">
                    <th scope="col" class="px-3 font-normal">{{ t("能力", "Capability") }}</th>
                    <th scope="col" class="px-3 font-normal">{{ t("状态", "Status") }}</th>
                    <th scope="col" class="px-3 font-normal">{{ t("原因码", "Reason codes") }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in capabilityRows" :key="row.key" class="h-11 border-b border-line/60 last:border-b-0">
                    <td class="px-3 py-2 align-top">{{ row.label }}</td>
                    <td class="px-3 py-2 align-top">
                      <UiBadge :tone="row.capability.status === 'supported' ? 'success' : row.capability.status === 'partial' ? 'caution' : 'danger'" :kind-label="t('能力', 'Capability')">
                        {{ capabilityLabel(row.capability.status) }}
                      </UiBadge>
                    </td>
                    <td class="px-3 py-2 align-top break-words text-muted">
                      {{ row.capability.reason_codes.map(reasonLabel).join(t("、", ", ")) || "—" }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <section class="space-y-2" data-testid="diagnostics-counters">
            <h2 class="ui-section-title">{{ t("计数与参考", "Counters & reference") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="cell in report.observations.region_counters" :key="cell.counter_key" class="px-3 py-2 break-words">
                <span class="mono">{{ cell.counter_key }}</span>：{{ t("候选读数", "candidate reading") }} {{ cell.value ?? "null" }}；{{ t("来源目标", "source target") }} {{ cell.source_target ?? t("未知", "unknown") }}；{{ t("原始读数", "raw reading") }}
                {{ cell.raw_value }}（<span class="mono">{{ cell.raw_value_hex }}</span>）
                <span v-if="!cell.within_source_target" class="text-caution"> · {{ copy.countConflict }}</span>
              </li>
            </ul>
            <p class="text-[13px] leading-5 break-words text-muted">
              {{ t("参考覆盖类别：", "Reference coverage categories: ") }}{{ report.coverage.reference_category_ids.join(t("、", ", ")) || t("无", "none") }}；{{ t("未满足原因：", "Unmet reasons: ") }}{{ (report.coverage.unavailable_reason_codes ?? []).map(reasonLabel).join(t("、", ", ")) || "—" }}
            </p>
          </section>

          <section class="space-y-2" data-testid="diagnostics-catalog-scope">
            <h2 class="ui-section-title">{{ t("图鉴口径", "Catalog scope") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li class="px-3 py-2 break-words">
                <span class="font-medium">{{ t("特殊区域 _special", "Special region _special") }}</span>：
                {{ tFormat(
                  "{total} 条（{withUnit} 条有子关卡，{withoutUnit} 条无 unit）",
                  "{total} items ({withUnit} with a sub-level, {withoutUnit} without a unit)",
                  { total: scopeNotes.specialTotal, withUnit: scopeNotes.specialWithUnit, withoutUnit: scopeNotes.specialWithoutUnit },
                ) }}
                {{ t(
                  "。这些是跨区域合集条目（Path of the Stars、Lara's Notebook 等），不属于 26 区主表；产品保留原区域，不改写为宿主区域。",
                  ". These are cross-region collection items (Path of the Stars, Lara's Notebook, etc.); they are not part of the 26-region table and are kept as-is rather than rewritten to a host region.",
                ) }}
              </li>
              <li v-if="scopeNotes.sharedChallengeId" class="px-3 py-2 break-words">
                <span class="font-medium">{{ t("挑战古墓共用 ID", "Shared challenge-tomb ID") }}</span>：
                {{ tFormat(
                  "{ids} 个 ID / {names} 个名称；其中 {shared} 个名称共用",
                  "{ids} IDs / {names} names; {shared} names share",
                  { ids: scopeNotes.challengeTombIds, names: scopeNotes.challengeTombNames, shared: scopeNotes.sharedChallengeNames.length },
                ) }}
                <span class="mono">{{ scopeNotes.sharedChallengeId }}</span>
                （{{ scopeNotes.sharedChallengeNames.join(" / ") }}）。
                {{ t(
                  "按 ID 计数、不逐项拆分；类别保持未闭合，不进入缺失结论。",
                  "Counted by ID without per-name splitting; the category stays unresolved and is not part of the missing conclusion.",
                ) }}
              </li>
            </ul>
          </section>

          <section class="space-y-2" data-testid="diagnostics-locations">
            <h2 class="ui-section-title">{{ t("定位与元数据", "Location & metadata") }}</h2>
            <ul class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="location in report.locations" :key="location.location_id" class="px-3 py-2 break-words">
                <span class="mono">{{ location.location_id }}</span>：{{ t("精度", "precision") }} {{ location.precision }}；{{ t("成员", "members") }} {{ location.member_ids.length }}；{{ t("门禁", "gate") }}
                {{ location.group_actionability?.kind ?? t("无（非组级线索）", "none (not a group-level clue)") }}
              </li>
            </ul>
            <p class="text-[13px] leading-5 break-words text-muted">
              {{ t("元数据版本：", "Metadata version: ") }}{{ report.metadata_version ?? t("未知", "unknown") }}；{{ t("参考集合版本：", "Reference-set version: ") }}{{ report.reference_set_version ?? t("未知", "unknown") }}；{{ t("解析器 profile：", "Parser profile: ") }}
              <span class="mono">{{ report.parser_profile_id }}</span>
            </p>
          </section>

          <section class="space-y-2" data-testid="selected-evidence">
            <h2 class="ui-section-title">{{ t("当前选中条目的原始证据", "Raw evidence for the selected item") }}</h2>
            <p v-if="!selected" class="text-[13px] text-muted">
              {{ t("未选中条目：在区域清单中选择一个条目查看原始证据。", "No item selected: choose an item in the region list to see its raw evidence.") }}
            </p>
            <UiEvidenceDetails
              v-else
              :summary="t('原始证据与数据集信息', 'Raw evidence and dataset information')"
              :hint="t('图鉴版本 / 区域绑定 / 子关卡 / 坐标 / 内部名 / 状态判定', 'catalog version / region binding / sub-level / coordinates / internal name / status rationale')"
            >
              <p>
                {{ t("条目：", "Item: ") }}{{ selected.item.display_name }}（<span class="mono">{{ selected.item.id }}</span>）
              </p>
              <p>{{ t("图鉴版本：", "Catalog version: ") }}{{ store.atlas.catalog_version }}（schema {{ store.atlas.catalog_schema_version }}）</p>
              <p>
                {{ t("区域：", "Region: ") }}{{ regionName(selected.region) }}（<span class="mono">{{ selected.region.region_hash }}</span>）<span
                  v-if="selectedRegionRecord?.binding"
                >
                  · {{ t(`绑定签名距离 ${selectedRegionRecord.binding.signature_distance}，次优 ${selectedRegionRecord.binding.runner_up_distance ?? "无"}`, `binding signature distance ${selectedRegionRecord.binding.signature_distance}, runner-up ${selectedRegionRecord.binding.runner_up_distance ?? "none"}`) }}
                </span>
              </p>
              <p v-if="selectedRegionRecord?.binding">
                {{ t("计数核对：", "Counter check: ") }}<span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.signature) }}</span>
                vs {{ t("存档目标", "save target") }} <span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.source_targets) }}</span>
              </p>
              <p v-if="selected.item.unit">
                {{ t("子关卡：", "Sub-level: ") }}<span class="mono">{{ selected.item.unit.unit }}</span> · zone
                <span class="mono">{{ selected.item.unit.zone ?? t("未知", "unknown") }}</span>（{{ t("名称来源", "name source") }} {{ selected.item.unit.name_source }}）
              </p>
              <p v-if="selected.item.position">
                {{ t("原始坐标：", "Raw coordinates: ") }}x={{ selected.item.position.x }}，y={{ selected.item.position.y }}，{{ t("空间", "space") }}
                <span class="mono">{{ selected.item.position.space }}</span>（{{ t("未校准", "uncalibrated") }}）
              </p>
              <p>{{ t("内部候选名：", "Internal candidate name: ") }}<span class="mono">{{ selected.item.internal_name ?? t("（无）", "(none)") }}</span></p>
              <p v-if="selected.item.collection">{{ t("故事合集：", "Story collection: ") }}<span class="mono">{{ selected.item.collection }}</span></p>
              <p>
                {{ t("类别：", "Category: ") }}{{ selectedCategoryName }}<template v-if="selected.item.name_source"> · {{ t("名称来源", "name source") }} {{ selected.item.name_source }}</template>
              </p>
              <p>{{ t("获取方式规则：", "Acquisition rule: ") }}<span class="mono">{{ selected.group.acquisition_rule_key }}</span></p>
              <p data-testid="selected-status-reason">
                <template v-if="selected.item.status === 'collected'">
                  {{ t("该条目 ID 出现在当前存档的物品表中（观察集合），因此判为已收集。", "This item ID appears in the current save’s inventory table (observed set), so it is judged collected.") }}
                </template>
                <template v-else>
                  {{ t("该条目 ID 未出现在当前存档的物品表中，因此判为未收集。若你在游戏中刚取得，请刷新（F5）后再看。", "This item ID does not appear in the current save’s inventory table, so it is judged uncollected. If you just obtained it in-game, refresh (F5) and check again.") }}
                </template>
              </p>
              <p v-if="store.isStale.value">{{ copy.stale }}：{{ t("本次结果仅反映上一次读取。", "This result reflects only the last read.") }}</p>
            </UiEvidenceDetails>
          </section>

          <section class="space-y-2" data-testid="diagnostics-entries">
            <h2 class="ui-section-title">{{ t("诊断条目", "Diagnostic entries") }}</h2>
            <div v-for="group in groups" :key="group.title" class="space-y-1.5">
              <h3 class="text-[13px] font-medium text-muted">{{ group.title }}</h3>
              <ul class="ui-panel divide-y divide-line/60">
                <li v-for="item in group.items" :key="`${item.code}-${item.message}`" class="flex items-start gap-2 px-3 py-2 text-[13px] leading-5">
                  <component :is="severityIcon(item.severity)" :size="16" :stroke-width="2" class="mt-0.5 shrink-0" :class="severityClass(item.severity)" aria-hidden="true" />
                  <div class="min-w-0 flex-1 break-words">
                    <span class="font-medium" :class="severityClass(item.severity)">{{ severityLabel(item.severity) }}</span>
                    · <span class="mono break-all">{{ item.code }}</span>：{{ item.message }}
                    <span v-if="item.evidence_refs.length" class="block text-xs text-muted">（{{ t("证据：", "evidence: ") }}{{ item.evidence_refs.join(t("、", ", ")) }}）</span>
                  </div>
                </li>
              </ul>
            </div>
            <p v-if="groups.length === 0" class="text-[13px] text-muted">{{ t("没有影响操作的诊断条目。", "No diagnostic entries affect operation.") }}</p>
          </section>

          <section class="space-y-2" data-testid="diagnostics-observed">
            <h2 class="ui-section-title">{{ t("已观察记录（未分类，不计入收集品数）", "Observed records (unclassified, not counted as collectibles)") }}</h2>
            <ul v-if="records.length" class="ui-panel divide-y divide-line/60 text-[13px] leading-5">
              <li v-for="record in records" :key="`${record.table}-${record.index}`" class="px-3 py-2 break-words">
                <span class="mono">{{ record.internal_id }}</span> · {{ t("表", "table") }} {{ record.table }} #{{ record.index }} · {{ t("伴随字", "companion") }}
                <span class="mono">{{ record.raw_companion_hex }}</span>（{{ t("低两位", "low two bits") }} {{ record.raw_low_two_bits }}） · {{ t("偏移", "offset") }}
                <span class="mono">{{ record.id_offset_hex }}</span>
              </li>
            </ul>
            <p v-else class="text-[13px] text-muted">{{ t("没有未分类记录。", "No unclassified records.") }}</p>
          </section>
        </div>
      </template>
    </div>
  </section>
</template>
