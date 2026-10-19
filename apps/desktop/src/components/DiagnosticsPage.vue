<script setup lang="ts">
// 诊断（SX-015）：输入与解析 / 支持能力 / 计数与参考 / 定位与元数据；
// 未知 ID 可见但不把应用变成十六进制浏览器；每条关联当前槽位。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { capabilityLabel, copy, reasonLabel, severityLabel } from "../app/labels";
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
    { key: "container", label: "容器（zlib）", capability: capabilities.container },
    { key: "sections", label: "区段流", capability: capabilities.sections },
    { key: "inventory", label: "物品表", capability: capabilities.inventory },
    { key: "counters", label: "区域计数", capability: capabilities.counters },
    { key: "reference_difference", label: "参考差集", capability: capabilities.reference_difference },
    { key: "location", label: "定位", capability: capabilities.location },
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
  return category ? (store.atlas.categories.find((entry) => entry.id === category)?.name ?? category) : null;
});
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="diagnostics-title">
    <div class="mx-auto max-w-4xl space-y-4">
      <header class="space-y-1">
        <h1 id="diagnostics-title" class="text-xl font-semibold">诊断</h1>
        <p class="text-[13px] text-muted">
          <template v-if="slot?.name">当前槽位：{{ slot.name }} · 快照 {{ formatSnapshotTime(slot.result?.snapshot_time ?? null) }}</template>
          <template v-else>未选择槽位</template>
        </p>
      </header>

      <UiEmptyState v-if="!report" :title="copy.noReport" :description="'诊断需要当前报告；请先分析一个存档文件。'">
        <UiButton variant="primary" @click="store.actions.setView('import')">打开存档</UiButton>
      </UiEmptyState>

      <template v-else>
        <CoverageNotice
          v-if="store.isStale.value"
          tone="warning"
          :title="copy.stale"
          :lines="['诊断同时保留旧快照的输入摘要与本次刷新错误。']"
        />

        <section class="space-y-2">
          <h2 class="text-base font-medium">输入与解析</h2>
          <div class="space-y-1 rounded-[8px] border border-line bg-surface px-3 py-2 text-[13px]">
            <p>
              输入摘要（压缩）：<span class="mono">{{ report.input_digest.compressed_sha256.slice(0, 24) }}…</span>
              （{{ formatBytes(report.input_digest.compressed_bytes) }}）
            </p>
            <p v-if="report.input_digest.expanded_sha256">
              输入摘要（解压）：<span class="mono">{{ report.input_digest.expanded_sha256.slice(0, 24) }}…</span>
              （{{ formatBytes(report.input_digest.expanded_bytes) }}）
            </p>
            <p>
              上下文：mode={{ report.context.mode }}（source={{ report.context.source }}）；platform={{ report.context.platform ?? "未知" }}；
              build={{ report.context.build ?? "未知" }}；DLC={{ report.context.dlc?.join("、") ?? "未知" }}
            </p>
            <ul v-if="report.context.limitations.length" class="list-inside list-disc text-muted">
              <li v-for="limitation in report.context.limitations" :key="limitation">{{ limitation }}</li>
            </ul>
            <p>
              容器：{{ report.observations.container.valid ? "有效" : "无效" }}；区段数 {{ report.observations.sections.section_count }}；
              原始记录 {{ report.observations.inventory.record_count }}（唯一 {{ report.observations.inventory.unique_id_count }}，重复
              {{ report.observations.inventory.duplicate_record_count }}）
            </p>
            <p class="text-muted">{{ report.observations.inventory.completeness_scope }}</p>
          </div>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">支持能力</h2>
          <table class="w-full border-collapse text-[13px]">
            <caption class="sr-only">能力矩阵</caption>
            <thead>
              <tr class="border-b border-line text-left text-muted">
                <th scope="col" class="py-1 font-normal">能力</th>
                <th scope="col" class="py-1 font-normal">状态</th>
                <th scope="col" class="py-1 font-normal">原因码</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in capabilityRows" :key="row.key" class="border-b border-line/60">
                <td class="py-1.5">{{ row.label }}</td>
                <td class="py-1.5">
                  <UiBadge :tone="row.capability.status === 'supported' ? 'success' : row.capability.status === 'partial' ? 'caution' : 'danger'" kind-label="能力">
                    {{ capabilityLabel(row.capability.status) }}
                  </UiBadge>
                </td>
                <td class="py-1.5 text-muted">
                  {{ row.capability.reason_codes.map(reasonLabel).join("、") || "—" }}
                </td>
              </tr>
            </tbody>
          </table>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">计数与参考</h2>
          <ul class="space-y-1 text-[13px]">
            <li v-for="cell in report.observations.region_counters" :key="cell.counter_key" class="rounded-[6px] border border-line bg-surface px-3 py-1.5">
              <span class="mono">{{ cell.counter_key }}</span>：候选读数 {{ cell.value ?? "null" }}；来源目标 {{ cell.source_target ?? "未知" }}；原始读数
              {{ cell.raw_value }}（<span class="mono">{{ cell.raw_value_hex }}</span>）
              <span v-if="!cell.within_source_target" class="text-caution"> · {{ copy.countConflict }}</span>
            </li>
          </ul>
          <p class="text-muted">参考覆盖类别：{{ report.coverage.reference_category_ids.join("、") || "无" }}；未满足原因：{{ (report.coverage.unavailable_reason_codes ?? []).map(reasonLabel).join("、") || "—" }}</p>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">定位与元数据</h2>
          <ul class="space-y-1 text-[13px]">
            <li v-for="location in report.locations" :key="location.location_id" class="rounded-[6px] border border-line bg-surface px-3 py-1.5">
              <span class="mono">{{ location.location_id }}</span>：精度 {{ location.precision }}；成员 {{ location.member_ids.length }}；门禁
              {{ location.group_actionability?.kind ?? "无（非组级线索）" }}
            </li>
          </ul>
          <p class="text-muted">
            元数据版本：{{ report.metadata_version ?? "未知" }}；参考集合版本：{{ report.reference_set_version ?? "未知" }}；解析器 profile：
            <span class="mono">{{ report.parser_profile_id }}</span>
          </p>
        </section>

        <section class="space-y-2" data-testid="selected-evidence">
          <h2 class="text-base font-medium">当前选中条目的原始证据</h2>
          <p v-if="!selected" class="text-[13px] text-muted">
            未选中条目：在区域清单中点一条，这里会显示它的图鉴版本、区域绑定、子关卡、原始坐标与状态判定说明。
          </p>
          <UiEvidenceDetails
            v-else
            summary="原始证据与数据集信息"
            hint="图鉴版本 / 区域绑定 / 子关卡 / 坐标 / 内部名 / 状态判定"
          >
            <p>
              条目：{{ selected.item.display_name }}（<span class="mono">{{ selected.item.id }}</span>）
            </p>
            <p>图鉴版本：{{ store.atlas.catalog_version }}（schema {{ store.atlas.catalog_schema_version }}）</p>
            <p>
              区域：{{ regionName(selected.region) }}（<span class="mono">{{ selected.region.region_hash }}</span>）<span
                v-if="selectedRegionRecord?.binding"
              >
                · 绑定签名距离 {{ selectedRegionRecord.binding.signature_distance }}，次优
                {{ selectedRegionRecord.binding.runner_up_distance ?? "无" }}
              </span>
            </p>
            <p v-if="selectedRegionRecord?.binding">
              计数核对：<span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.signature) }}</span>
              vs 存档目标 <span class="mono">{{ JSON.stringify(selectedRegionRecord.binding.source_targets) }}</span>
            </p>
            <p v-if="selected.item.unit">
              子关卡：<span class="mono">{{ selected.item.unit.unit }}</span> · zone
              <span class="mono">{{ selected.item.unit.zone ?? "未知" }}</span>（名称来源 {{ selected.item.unit.name_source }}）
            </p>
            <p v-if="selected.item.position">
              原始坐标：x={{ selected.item.position.x }}，y={{ selected.item.position.y }}，空间
              <span class="mono">{{ selected.item.position.space }}</span>（未校准）
            </p>
            <p>内部候选名：<span class="mono">{{ selected.item.internal_name ?? "（无）" }}</span></p>
            <p v-if="selected.item.collection">故事合集：<span class="mono">{{ selected.item.collection }}</span></p>
            <p>
              类别：{{ selectedCategoryName }}<template v-if="selected.item.name_source"> · 名称来源 {{ selected.item.name_source }}</template>
            </p>
            <p>获取方式规则：<span class="mono">{{ selected.group.acquisition_rule_key }}</span></p>
            <p data-testid="selected-status-reason">
              <template v-if="selected.item.status === 'collected'">
                该条目 ID 出现在当前存档的物品表中（观察集合），因此判为已收集。
              </template>
              <template v-else>
                该条目 ID 未出现在当前存档的物品表中，因此判为未收集。若你在游戏中刚取得，请刷新（F5）后再看。
              </template>
            </p>
            <p>区域与子类别已用存档自带的区域计数器逐行核对（区域 × 类别 43 行全部吻合）。</p>
            <p v-if="store.isStale.value">{{ copy.stale }}：本次结果仅反映上一次读取。</p>
          </UiEvidenceDetails>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">诊断条目</h2>
          <div v-for="group in groups" :key="group.title" class="space-y-1">
            <h3 class="text-[13px] font-medium">{{ group.title }}</h3>
            <ul class="space-y-1">
              <li v-for="item in group.items" :key="`${item.code}-${item.message}`" class="rounded-[6px] border border-line bg-surface px-3 py-1.5 text-[13px]">
                <span
                  :class="item.severity === 'error' ? 'text-danger' : item.severity === 'warning' ? 'text-caution' : 'text-info'"
                  >{{ severityLabel(item.severity) }}</span
                >
                · <span class="mono">{{ item.code }}</span>：{{ item.message }}
                <span v-if="item.evidence_refs.length" class="text-muted">（证据：{{ item.evidence_refs.join("、") }}）</span>
              </li>
            </ul>
          </div>
          <p v-if="groups.length === 0" class="text-[13px] text-muted">没有影响操作的诊断条目。</p>
        </section>

        <section class="space-y-2">
          <h2 class="text-base font-medium">已观察记录（未分类，不计入收集品数）</h2>
          <p class="text-[13px] text-muted">
            这里保留未分类/未知非零 ID 的原始观察（共 {{ records.length }} 条）；主清单的“全部”不包含它们。
          </p>
          <ul class="space-y-1">
            <li v-for="record in records" :key="`${record.table}-${record.index}`" class="rounded-[6px] border border-line bg-surface px-3 py-1.5 text-[13px]">
              <span class="mono">{{ record.internal_id }}</span> · 表 {{ record.table }} #{{ record.index }} · 伴随字
              <span class="mono">{{ record.raw_companion_hex }}</span>（低两位 {{ record.raw_low_two_bits }}） · 偏移
              <span class="mono">{{ record.id_offset_hex }}</span>
            </li>
          </ul>
          <p v-if="records.length === 0" class="text-[13px] text-muted">没有未分类记录。</p>
        </section>
      </template>
    </div>
  </section>
</template>
