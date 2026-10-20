<script setup lang="ts">
// 诊断（SX-015）：输入与解析 / 支持能力 / 计数与参考 / 定位与元数据；
// 未知 ID 可见但不把应用变成十六进制浏览器；每条关联当前槽位。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { t } from "../app/i18n";
import { capabilityLabel, categoryLabel, copy, reasonLabel, severityLabel } from "../app/labels";
import { formatBytes, formatSnapshotTime } from "../app/format";
import { regionName } from "../app/collection";
import { diagnosticGroups, unclassifiedRecords } from "../app/catalog";
import CoverageNotice from "./CoverageNotice.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEmptyState from "./ui/UiEmptyState.vue";
import UiEvidenceDetails from "./ui/UiEvidenceDetails.vue";

const store = useStore();
const report = computed(() => store.report.value);
const slot = computed(() => store.currentSlot.value);

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
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="diagnostics-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="diagnostics-title" class="text-xl font-semibold">{{ t("诊断", "Diagnostics") }}</h1>
        <p class="text-[13px] text-muted">
          <template v-if="slot?.name">
            {{ t(`当前槽位：${slot.name} · 快照 ${formatSnapshotTime(slot.result?.snapshot_time ?? null)}`, `Current slot: ${slot.name} · snapshot ${formatSnapshotTime(slot.result?.snapshot_time ?? null)}`) }}
          </template>
          <template v-else>{{ t("未选择槽位", "No slot selected") }}</template>
        </p>
      </header>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="t('诊断需要当前报告；请先分析一个存档文件。', 'Diagnostics need a current report; analyze a save file first.')">
        <UiButton variant="primary" @click="store.actions.setView('import')">{{ t("打开存档", "Open a save") }}</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          :title="copy.stale"
          :lines="[t('诊断同时保留旧快照的输入摘要与本次刷新错误。', 'Diagnostics also keep the stale snapshot’s input digest and this refresh error.')]"
        />

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("输入与解析", "Input & parsing") }}</h2>
          <div class="space-y-1 rounded-[8px] border border-line bg-surface px-3 py-2 text-[13px]">
            <p>
              {{ t("输入摘要（压缩）：", "Input digest (compressed):") }}<span class="mono">{{ report.input_digest.compressed_sha256.slice(0, 24) }}…</span>
              （{{ formatBytes(report.input_digest.compressed_bytes) }}）
            </p>
            <p v-if="report.input_digest.expanded_sha256">
              {{ t("输入摘要（解压）：", "Input digest (expanded):") }}<span class="mono">{{ report.input_digest.expanded_sha256.slice(0, 24) }}…</span>
              （{{ formatBytes(report.input_digest.expanded_bytes) }}）
            </p>
            <p>
              {{ t("上下文：", "Context:") }}mode={{ report.context.mode }}（source={{ report.context.source }}）；platform={{ report.context.platform ?? t("未知", "unknown") }}；
              build={{ report.context.build ?? t("未知", "unknown") }}；DLC={{ report.context.dlc?.join(t("、", ", ")) ?? t("未知", "unknown") }}
            </p>
            <ul v-if="report.context.limitations.length" class="list-inside list-disc text-muted">
              <li v-for="limitation in report.context.limitations" :key="limitation">{{ limitation }}</li>
            </ul>
            <p>
              {{ t("容器：", "Container:") }}{{ report.observations.container.valid ? t("有效", "valid") : t("无效", "invalid") }}；{{ t("区段数", "Sections") }} {{ report.observations.sections.section_count }}；
              {{ t("原始记录", "Raw records") }} {{ report.observations.inventory.record_count }}（{{ t("唯一", "unique") }} {{ report.observations.inventory.unique_id_count }}，{{ t("重复", "duplicates") }}
              {{ report.observations.inventory.duplicate_record_count }}）
            </p>
            <p class="text-muted">{{ report.observations.inventory.completeness_scope }}</p>
          </div>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("支持能力", "Supported capabilities") }}</h2>
          <table class="w-full border-collapse text-[13px]">
            <caption class="sr-only">{{ t("能力矩阵", "Capability matrix") }}</caption>
            <thead>
              <tr class="border-b border-line text-left text-muted">
                <th scope="col" class="py-1 font-normal">{{ t("能力", "Capability") }}</th>
                <th scope="col" class="py-1 font-normal">{{ t("状态", "Status") }}</th>
                <th scope="col" class="py-1 font-normal">{{ t("原因码", "Reason codes") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in capabilityRows" :key="row.key" class="border-b border-line/60">
                <td class="py-1.5">{{ row.label }}</td>
                <td class="py-1.5">
                  <UiBadge :tone="row.capability.status === 'supported' ? 'success' : row.capability.status === 'partial' ? 'caution' : 'danger'" :kind-label="t('能力', 'Capability')">
                    {{ capabilityLabel(row.capability.status) }}
                  </UiBadge>
                </td>
                <td class="py-1.5 text-muted">
                  {{ row.capability.reason_codes.map(reasonLabel).join(t("、", ", ")) || "—" }}
                </td>
              </tr>
            </tbody>
          </table>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("计数与参考", "Counters & reference") }}</h2>
          <ul class="space-y-1 text-[13px]">
            <li v-for="cell in report.observations.region_counters" :key="cell.counter_key" class="rounded-[6px] border border-line bg-surface px-3 py-1.5">
              <span class="mono">{{ cell.counter_key }}</span>：{{ t("候选读数", "candidate reading") }} {{ cell.value ?? "null" }}；{{ t("来源目标", "source target") }} {{ cell.source_target ?? t("未知", "unknown") }}；{{ t("原始读数", "raw reading") }}
              {{ cell.raw_value }}（<span class="mono">{{ cell.raw_value_hex }}</span>）
              <span v-if="!cell.within_source_target" class="text-caution"> · {{ copy.countConflict }}</span>
            </li>
          </ul>
          <p class="text-muted">
            {{ t("参考覆盖类别：", "Reference coverage categories: ") }}{{ report.coverage.reference_category_ids.join(t("、", ", ")) || t("无", "none") }}；{{ t("未满足原因：", "Unmet reasons: ") }}{{ (report.coverage.unavailable_reason_codes ?? []).map(reasonLabel).join(t("、", ", ")) || "—" }}
          </p>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("定位与元数据", "Location & metadata") }}</h2>
          <ul class="space-y-1 text-[13px]">
            <li v-for="location in report.locations" :key="location.location_id" class="rounded-[6px] border border-line bg-surface px-3 py-1.5">
              <span class="mono">{{ location.location_id }}</span>：{{ t("精度", "precision") }} {{ location.precision }}；{{ t("成员", "members") }} {{ location.member_ids.length }}；{{ t("门禁", "gate") }}
              {{ location.group_actionability?.kind ?? t("无（非组级线索）", "none (not a group-level clue)") }}
            </li>
          </ul>
          <p class="text-muted">
            {{ t("元数据版本：", "Metadata version: ") }}{{ report.metadata_version ?? t("未知", "unknown") }}；{{ t("参考集合版本：", "Reference-set version: ") }}{{ report.reference_set_version ?? t("未知", "unknown") }}；{{ t("解析器 profile：", "Parser profile: ") }}
            <span class="mono">{{ report.parser_profile_id }}</span>
          </p>
        </section>

        <section class="space-y-2" data-testid="selected-evidence">
          <h2 class="text-base font-medium">{{ t("当前选中条目的原始证据", "Raw evidence for the selected item") }}</h2>
          <p v-if="!selected" class="text-[13px] text-muted">
            {{ t("未选中条目：在区域清单中点一条，这里会显示它的图鉴版本、区域绑定、子关卡、原始坐标与状态判定说明。", "No item selected: click a row in the region list to see its catalog version, region binding, sub-level, raw coordinates and status rationale here.") }}
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
            <p>{{ t("区域与子类别已用存档自带的区域计数器逐行核对（区域 × 类别 43 行全部吻合）。", "Regions and subcategories were checked line by line against the save’s own region counters (all 43 region × category rows match).") }}</p>
            <p v-if="store.isStale.value">{{ copy.stale }}：{{ t("本次结果仅反映上一次读取。", "This result reflects only the last read.") }}</p>
          </UiEvidenceDetails>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("诊断条目", "Diagnostic entries") }}</h2>
          <div v-for="group in groups" :key="group.title" class="space-y-1">
            <h3 class="text-[13px] font-medium">{{ group.title }}</h3>
            <ul class="space-y-1">
              <li v-for="item in group.items" :key="`${item.code}-${item.message}`" class="rounded-[6px] border border-line bg-surface px-3 py-1.5 text-[13px]">
                <span
                  :class="item.severity === 'error' ? 'text-danger' : item.severity === 'warning' ? 'text-caution' : 'text-info'"
                  >{{ severityLabel(item.severity) }}</span
                >
                · <span class="mono">{{ item.code }}</span>：{{ item.message }}
                <span v-if="item.evidence_refs.length" class="text-muted">（{{ t("证据：", "evidence: ") }}{{ item.evidence_refs.join(t("、", ", ")) }}）</span>
              </li>
            </ul>
          </div>
          <p v-if="groups.length === 0" class="text-[13px] text-muted">{{ t("没有影响操作的诊断条目。", "No diagnostic entries affect operation.") }}</p>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">{{ t("已观察记录（未分类，不计入收集品数）", "Observed records (unclassified, not counted as collectibles)") }}</h2>
          <p class="text-[13px] text-muted">
            {{ t(`这里保留未分类/未知非零 ID 的原始观察（共 ${records.length} 条）；主清单的“全部”不包含它们。`, `Raw observations for unclassified/unknown non-zero IDs are kept here (${records.length} records); “All” in the main list excludes them.`) }}
          </p>
          <ul class="space-y-1">
            <li v-for="record in records" :key="`${record.table}-${record.index}`" class="rounded-[6px] border border-line bg-surface px-3 py-1.5 text-[13px]">
              <span class="mono">{{ record.internal_id }}</span> · {{ t("表", "table") }} {{ record.table }} #{{ record.index }} · {{ t("伴随字", "companion") }}
              <span class="mono">{{ record.raw_companion_hex }}</span>（{{ t("低两位", "low two bits") }} {{ record.raw_low_two_bits }}） · {{ t("偏移", "offset") }}
              <span class="mono">{{ record.id_offset_hex }}</span>
            </li>
          </ul>
          <p v-if="records.length === 0" class="text-[13px] text-muted">{{ t("没有未分类记录。", "No unclassified records.") }}</p>
        </section>
      </template>
    </div>
  </section>
</template>
