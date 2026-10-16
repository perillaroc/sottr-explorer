<script setup lang="ts">
// 详情面板（SX-015）：名称与状态 → 去哪里找 → 攻略动作 → 为什么 → 默认折叠证据。
// 门禁结论一律读取报告的 group_actionability；旧快照关闭行动摘要并按“仅一般阅读”措辞。
import { computed, nextTick, onMounted, ref } from "vue";
import { Copy, ExternalLink } from "@lucide/vue";
import { useStore } from "../app/inject";
import { copyText } from "../app/clipboard";
import { recoveryHint } from "../data/errors";
import {
  businessStatus,
  categoryLabel,
  copy,
  evidenceLabel,
  gateText,
  itemEvidenceLevel,
  precisionLabel,
  reasonLabel,
  regionLabel,
  sourceLabel,
} from "../app/labels";
import { formatSnapshotTime, shortId } from "../app/format";
import {
  counterReadingForGroup,
  displayNameOf,
  inventoryRecordsFor,
  referenceSetsFor,
  type CollectibleRow,
  type GroupRow,
  type ItemRow,
} from "../app/catalog";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEvidenceDetails from "./ui/UiEvidenceDetails.vue";
import CoverageNotice from "./CoverageNotice.vue";

const props = withDefaults(
  defineProps<{
    row: CollectibleRow;
    /** pane：宽屏常驻；drawer：模态抽屉；page：窄窗页面替换。 */
    mode?: "pane" | "drawer" | "page";
  }>(),
  { mode: "pane" },
);

const store = useStore();
const heading = ref<HTMLElement | null>(null);
const leadText = ref<string | null>(null);

onMounted(() => {
  if (props.mode === "page") {
    void nextTick(() => heading.value?.focus());
  }
});

const report = computed(() => store.report.value);
const currentSlot = computed(() => store.currentSlot.value);
const isStale = computed(() => store.isStale.value);

const regionId = computed(() => props.row.regionId);
const regionText = computed(() =>
  regionLabel(regionId.value, regionId.value ? (store.metadata.regions[regionId.value]?.display_name ?? null) : null),
);

function itemRow(row: CollectibleRow): ItemRow | null {
  return row.kind === "item" ? row : null;
}

const nameInfo = computed(() => {
  if (props.row.kind === "group") {
    const group = props.row.location.guide_group_id
      ? store.metadata.guide_groups[props.row.location.guide_group_id]
      : null;
    return {
      text: group?.display_name ?? props.row.location.guide_group_id ?? props.row.id,
      note: group?.name_kind === "curated_short" ? "人工简名，不是游戏正式名称" : null,
    };
  }
  return displayNameOf(store.metadata, props.row.id, shortId);
});

const item = computed(() => itemRow(props.row)?.item ?? null);
const status = computed(() => (item.value ? businessStatus(item.value) : null));
const evidenceLevel = computed(() => (item.value ? itemEvidenceLevel(item.value) : null));
const precision = computed(() =>
  props.row.kind === "group" ? ("guide_group" as const) : (props.row.location?.precision ?? null),
);

const groupActionability = computed(() => {
  if (props.row.kind === "group") {
    return props.row.location.group_actionability;
  }
  return props.row.location?.guide_group_id ? (props.row.location.group_actionability ?? null) : null;
});

const gate = computed(() => {
  const actionability = groupActionability.value;
  if (!actionability) {
    return null;
  }
  const targetCount =
    props.row.kind === "group" ? props.row.location.target_count : (props.row.location?.target_count ?? null);
  return {
    kind: actionability.kind,
    ...gateText(actionability.kind, targetCount),
    reasonCodes: actionability.reason_codes,
  };
});

/** 旧快照：关闭遗漏推定及据此生成的行动摘要（外部链接仅作一般来源阅读）。 */
const actionClosed = computed(() => isStale.value || gate.value?.closed === true);

const sourceIds = computed(() =>
  props.row.kind === "group" ? props.row.location.source_refs : (props.row.location?.source_refs ?? []),
);

const sources = computed(() =>
  sourceIds.value.map((sourceId) => {
    const source = store.metadata.sources[sourceId] ?? null;
    return {
      id: sourceId,
      label: sourceLabel(sourceId, source?.url ?? null),
      url: source?.url ?? null,
    };
  }),
);

const primarySource = computed(() => sources.value.find((source) => source.url !== null) ?? sources.value[0] ?? null);

const memberItems = computed(() => {
  if (props.row.kind !== "group") {
    return [];
  }
  const group = props.row as GroupRow;
  const result: { id: string; item: NonNullable<ItemRow["item"]> }[] = [];
  for (const id of group.location.member_ids) {
    const entry = report.value?.assessments.find((assessment) => assessment.internal_id === id) ?? null;
    if (entry) {
      result.push({ id, item: entry });
    }
  }
  return result;
});

const evidenceEntries = computed(() => {
  const refs = new Set<string>(
    props.row.kind === "group" ? (groupActionability.value?.evidence_refs ?? []) : props.row.item.evidence_refs,
  );
  return (report.value?.evidence ?? []).filter((entry) => refs.has(entry.evidence_id));
});

const evaluatedDigest = computed(() => groupActionability.value?.evaluated_input_digest.compressed_sha256 ?? null);

/** 表位置：全部原始记录（同一 ID 的多条记录不去重、不合并）。 */
const records = computed(() => (props.row.kind === "item" && report.value ? inventoryRecordsFor(report.value, props.row.id) : []));

/** 参考集合命中（未命中不推断“非参考条目”）。 */
const refSets = computed(() =>
  props.row.kind === "item" ? referenceSetsFor(store.metadata, props.row.id) : [],
);

/** 组对应的区域计数读数（“当前计数”行；无映射为 null）。 */
const groupCounter = computed(() =>
  props.row.kind === "group" && report.value
    ? counterReadingForGroup(store.metadata, report.value, props.row.location.guide_group_id)
    : null,
);

/** 攻略动作文案：旧快照/门禁关闭时一律降级为一般来源阅读。 */
const sourceActionLabel = computed(() => {
  if (actionClosed.value) {
    return "阅读相关来源";
  }
  if (props.row.kind === "group") {
    return gate.value?.action ?? "查看攻略来源";
  }
  return `在浏览器查看 ${primarySource.value?.label ?? "攻略"}`;
});

function copyLead(): void {
  const lines = [
    `${nameInfo.value.text}（${item.value ? categoryLabel(store.metadata.items[props.row.id]?.category_id ?? null, null) : "攻略组线索"}）`,
    `状态：${status.value?.label ?? "推定遗漏（组级）"}；依据：${evidenceLevel.value ? evidenceLabel(evidenceLevel.value) : "社区关联"}；定位：${precisionLabel(precision.value)}`,
    `区域：${regionText.value}`,
    `线索：${gate.value?.note ?? "暂无攻略组或精确条目对应"}`,
    isStale.value ? "（旧快照：未反映本次读取，仅作一般阅读）" : "",
  ];
  const text = lines.filter((line) => line.length > 0).join("\n");
  leadText.value = text;
  void copyText(text);
}

/** 当前行所属定位线索（组行用自身；单项用其线索）。 */
const locationId = computed(() => props.row.kind === "group" ? props.row.location.location_id : (props.row.location?.location_id ?? null));

/** 链接动作状态（解析/打开/复制；含错误与已核验 URL）。 */
const linkAction = computed(() => {
  if (!locationId.value || !primarySource.value) {
    return null;
  }
  return store.linkActionFor(locationId.value, primarySource.value.id);
});

const linkBusy = computed(
  () => linkAction.value?.status === "opening" || linkAction.value?.status === "resolving",
);

function openSource(): void {
  const source = primarySource.value;
  if (!source || !locationId.value) {
    return;
  }
  void store.actions.openGuideLink(locationId.value, source.id, source.label);
}

function copyVerifiedLink(): void {
  const source = primarySource.value;
  if (!source || !locationId.value) {
    return;
  }
  void store.actions.copyGuideLink(locationId.value, source.id);
}

function copySourceLink(sourceRef: string): void {
  if (!locationId.value) {
    return;
  }
  void store.actions.copyGuideLink(locationId.value, sourceRef);
}
</script>

<template>
  <article class="space-y-3 pb-4" data-testid="item-detail">
    <header class="space-y-1">
      <p class="text-xs text-muted">
        {{ currentSlot?.name ?? "未选择文件" }}
        <template v-if="currentSlot?.result?.snapshot_time"> · 快照 {{ formatSnapshotTime(currentSlot.result.snapshot_time) }}</template>
      </p>
      <h2 ref="heading" tabindex="-1" class="text-base leading-snug font-semibold break-words" data-testid="detail-heading">
        {{ nameInfo.text }}
        <span v-if="props.row.kind === 'group'" class="text-muted">（本组 {{ props.row.memberCount }} 个成员 ID）</span>
      </h2>
      <p v-if="nameInfo.note" class="text-xs text-muted">{{ nameInfo.note }}</p>
      <p v-if="props.row.kind === 'item'" class="mono text-xs text-muted">{{ props.row.id }}</p>
      <div class="flex flex-wrap items-center gap-1.5">
        <UiBadge v-if="status" :tone="status.tone" kind-label="状态">{{ status.label }}</UiBadge>
        <UiBadge v-else tone="caution" kind-label="状态">推定遗漏</UiBadge>
        <UiBadge tone="neutral" kind-label="依据">{{ evidenceLevel ? evidenceLabel(evidenceLevel) : "社区关联" }}</UiBadge>
        <UiBadge tone="info" kind-label="定位">{{ precisionLabel(precision) }}</UiBadge>
      </div>
    </header>

    <CoverageNotice
      v-if="isStale"
      tone="warning"
      :title="copy.stale"
      :lines="[
        `最近刷新错误：${currentSlot?.result?.last_refresh_error?.code ?? '未知'}`,
        '旧证据可读；外部链接仅作一般来源阅读，不代表当前状态。',
      ]"
    />

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">去哪里找</h3>
      <p class="text-[13px]">{{ regionText }}<span v-if="props.row.kind === 'group'"> · {{ nameInfo.text }}</span></p>
      <p v-if="gate" class="text-[13px] text-muted" data-testid="detail-gate">
        {{ gate.label }}：{{ gate.note }}
      </p>
      <p v-else-if="precision === 'region'" class="text-[13px] text-muted">
        {{ copy.noTrustedLocation }}：仅有区域线索，无攻略组或精确条目对应。
      </p>
      <p v-else class="text-[13px] text-muted">{{ copy.noTrustedLocation }}；可复制候选名称用于手动搜索。</p>

      <div class="flex flex-wrap items-center gap-2 pt-1">
        <UiButton
          size="sm"
          :disabled="!primarySource || linkBusy"
          :title="
            actionClosed
              ? '旧快照/门禁关闭：仅作一般来源阅读'
              : primarySource
                ? `在系统浏览器打开：${primarySource.label}`
                : '没有可用来源'
          "
          data-testid="open-source"
          @click="openSource"
        >
          <ExternalLink :size="16" aria-hidden="true" />
          {{ linkBusy ? "正在处理…" : sourceActionLabel }}
        </UiButton>
        <UiButton
          size="sm"
          variant="ghost"
          :disabled="!primarySource || linkBusy"
          :title="primarySource ? `复制已核验链接：${primarySource.label}` : '没有可用来源'"
          data-testid="copy-verified-link"
          @click="copyVerifiedLink"
        >
          <Copy :size="16" aria-hidden="true" />
          复制已核验链接
        </UiButton>
        <UiButton size="sm" variant="ghost" data-testid="copy-lead" @click="copyLead">
          <Copy :size="16" aria-hidden="true" />
          复制可用线索
        </UiButton>
      </div>

      <p v-if="linkAction && linkAction.status === 'opened'" class="text-[13px] text-success" data-testid="link-opened">
        已在系统浏览器打开（WebView 不加载网页）。
        <span class="mono break-all">{{ linkAction.url }}</span>
      </p>
      <p v-else-if="linkAction && linkAction.status === 'copied'" class="text-[13px] text-success" data-testid="link-copied">
        已复制已核验链接：
        <span class="mono break-all">{{ linkAction.url }}</span>
      </p>
      <CoverageNotice
        v-else-if="linkAction && linkAction.status === 'error' && linkAction.error"
        tone="danger"
        test-id="link-error"
        :title="`链接操作失败：${linkAction.error.code}`"
        :lines="[linkAction.error.message, recoveryHint(linkAction.error) ?? '可重试或复制可用线索。']"
      >
        <UiButton size="sm" :disabled="linkBusy" @click="openSource">重试打开</UiButton>
        <UiButton size="sm" variant="ghost" :disabled="linkBusy" @click="copyVerifiedLink">复制已核验链接</UiButton>
      </CoverageNotice>

      <p class="text-xs text-muted">
        说明：未建立内部 ID 与攻略编号的一对一映射，当前无已验证坐标；链接不附加存档内容或私人路径。
      </p>
      <pre
        v-if="leadText"
        class="mono max-h-40 overflow-auto rounded-[6px] border border-line bg-raised px-2 py-1 text-xs whitespace-pre-wrap"
        data-testid="lead-text"
        >{{ leadText }}</pre
      >
    </section>

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">
        为什么显示为{{ props.row.kind === "group" ? "组级线索" : (status?.label ?? "该状态") }}
      </h3>
      <p class="text-[13px] text-muted">
        <template v-if="props.row.kind === 'group'">
          组成员的参考差集结论与当前快照一致；组级判断来自领域层门禁（不凭计数自行推导）。
        </template>
        <template v-else>{{ status?.hint ?? "证据不足以判断收集状态。" }}</template>
      </p>
      <ul class="space-y-0.5 text-[13px] text-muted">
        <li v-if="gate?.reasonCodes.length">门禁原因码：{{ gate.reasonCodes.map(reasonLabel).join("、") }}</li>
        <li v-if="item?.rule_id">规则：<span class="mono">{{ item.rule_id }}</span></li>
        <li
          v-for="limitation in props.row.kind === 'group' ? props.row.location.limitations : props.row.item.limitations"
          :key="limitation"
        >
          {{ limitation }}
        </li>
        <li v-if="report?.reference_set_version">参考集合版本：{{ report.reference_set_version }}</li>
        <li v-if="evaluatedDigest">评估绑定摘要：<span class="mono">{{ evaluatedDigest.slice(0, 16) }}…</span></li>
      </ul>
    </section>

    <section v-if="props.row.kind === 'group'" class="space-y-1.5">
      <h3 class="text-sm font-medium">组内成员（保留原始身份，不伪造攻略编号）</h3>
      <ul class="space-y-0.5 text-[13px]">
        <li v-for="member in memberItems" :key="member.id" class="flex flex-wrap items-center gap-2">
          <span class="break-words">{{ displayNameOf(store.metadata, member.id, shortId).text }}</span>
          <span class="mono text-xs text-muted">{{ member.id }}</span>
          <UiBadge class="ml-auto" :tone="businessStatus(member.item).tone" kind-label="状态">
            {{ businessStatus(member.item).label }}
          </UiBadge>
        </li>
      </ul>
    </section>

    <UiEvidenceDetails summary="原始证据与来源" hint="全部候选名/记录/偏移/规则/参考集/来源版本">
      <p>ID：<span class="mono">{{ props.row.kind === "group" ? props.row.location.guide_group_id : props.row.id }}</span></p>
      <p v-if="props.row.kind === 'item'">
        候选内部名：<span class="mono">{{ store.metadata.items[props.row.id]?.aliases.join("；") || "（无）" }}</span>
      </p>
      <p v-if="props.row.kind === 'item'">
        类别：{{ categoryLabel(store.metadata.items[props.row.id]?.category_id ?? null, null) }}；区域：{{ regionText }}
      </p>
      <p v-if="props.row.kind === 'group'">组成员 ID：<span class="mono">{{ props.row.location.member_ids.join("、") }}</span></p>
      <p v-if="props.row.kind === 'group' && props.row.location.target_count !== null">
        组目标数：{{ props.row.location.target_count }}
      </p>
      <p v-if="groupCounter" data-testid="detail-group-counter">
        当前区域计数：{{ groupCounter.label }} {{ groupCounter.value ?? "未知" }}/{{ groupCounter.target ?? "未知" }}
        <span class="mono">（{{ groupCounter.counterKey }}）</span>
        <span v-if="!groupCounter.withinSourceTarget" class="text-caution">
          · 计数待核对（原始读数 {{ groupCounter.rawValue }}）
        </span>
      </p>

      <!-- 表位置：保留全部原始记录（重复 ID 不去重） -->
      <p v-if="props.row.kind === 'item'" data-testid="detail-table-position">
        <template v-if="records.length > 0">
          表位置：
          <span v-for="(record, index) in records" :key="`${record.table}-${record.index}`"
            ><span v-if="index > 0">；</span>表 <span class="mono">{{ record.table }}</span> #{{ record.index }} ·
            偏移 <span class="mono">{{ record.id_offset_hex }}</span> · 伴随字
            <span class="mono">{{ record.raw_companion_hex }}</span></span
          >
        </template>
        <template v-else>表位置：当前物品表中无该 ID 的记录（未观察到）</template>
      </p>

      <!-- 参考集：命中集合与类别；未命中不推断“非参考条目” -->
      <p v-if="props.row.kind === 'item'" data-testid="detail-reference-sets">
        <template v-if="refSets.length > 0">
          参考集合：
          <span v-for="(set, index) in refSets" :key="set.id"
            ><span v-if="index > 0">；</span><span class="mono">{{ set.id }}</span>（版本 {{ set.version }}；类别
            {{ set.categories.join("、") }}）</span
          >
        </template>
        <template v-else>参考集合：未命中任何内部参考类别（仅保留原始观察）</template>
      </p>

      <ul class="space-y-0.5">
        <li v-for="entry in evidenceEntries" :key="entry.evidence_id">
          <span class="mono">{{ entry.evidence_id }}</span>：{{ entry.method }}
          <span v-if="entry.offset_hex">；偏移 {{ entry.offset_hex }}</span>
          <span v-if="entry.reference_set_id">；参考集合 {{ entry.reference_set_id }}</span>
          <span v-if="entry.cross_checks.length">；交叉检查 {{ entry.cross_checks.join("；") }}</span>
        </li>
      </ul>
      <p v-if="sources.length">
        来源：
        <span v-for="(source, index) in sources" :key="source.id"
          ><span v-if="index > 0">；</span>{{ source.label }}
          <span class="mono">{{ source.url ?? "（无链接，仅书面记录）" }}</span>
          <button
            v-if="source.url"
            type="button"
            class="ml-1 cursor-pointer text-accent-strong hover:underline"
            :aria-label="`复制已核验链接：${source.label}`"
            @click="copySourceLink(source.id)"
          >
            复制链接
          </button></span
        >
      </p>
      <p v-if="report">
        输入摘要（压缩 SHA-256）：<span class="mono">{{ report.input_digest.compressed_sha256.slice(0, 24) }}…</span>
      </p>
      <p v-if="report">
        元数据版本：{{ report.metadata_version ?? "未知" }}；解析器 profile：<span class="mono">{{ report.parser_profile_id }}</span>
      </p>
    </UiEvidenceDetails>
  </article>
</template>
