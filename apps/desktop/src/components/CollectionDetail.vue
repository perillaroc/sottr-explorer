<script setup lang="ts">
// 收集品详情（SX-022）：名称与状态 → 去哪里找 → 怎么拿 → 原始证据（默认折叠）。
// 位置是关卡内归一化坐标，未校准；界面不把它写成导航点位，也不画真实地图底图。
import { computed, nextTick, onMounted, ref } from "vue";
import { Copy } from "@lucide/vue";
import { useStore } from "../app/inject";
import { copyText } from "../app/clipboard";
import { copy } from "../app/labels";
import { formatSnapshotTime } from "../app/format";
import { regionName } from "../app/collection";
import type { PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import PositionScatter from "./PositionScatter.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiEvidenceDetails from "./ui/UiEvidenceDetails.vue";

const props = withDefaults(
  defineProps<{
    region: PlanRegion;
    group: PlanGroup;
    item: PlanItem;
    /** pane：宽屏常驻；drawer：模态抽屉；page：窄窗页面替换。 */
    mode?: "pane" | "drawer" | "page";
  }>(),
  { mode: "pane" },
);

const store = useStore();
const heading = ref<HTMLElement | null>(null);
const copiedHint = ref(false);
const leadText = ref<string | null>(null);

onMounted(() => {
  if (props.mode === "page") {
    void nextTick(() => heading.value?.focus());
  }
});

const collected = computed(() => props.item.status === "collected");

/** 同一子关卡内的条目（用于相对位置散点与“还有 N 项”）。 */
const sameUnitItems = computed<PlanItem[]>(() => {
  const unit = props.item.unit?.unit;
  if (!unit) {
    return [];
  }
  return props.region.groups
    .flatMap((group) => group.items)
    .filter((entry) => entry.unit?.unit === unit);
});

const groupFact = computed(() => {
  const sub = store.atlas.categories.find((entry) => entry.id === props.group.category);
  return sub?.name ?? props.group.category;
});

const regionRecord = computed(() =>
  store.atlas.regions.find((entry) => entry.region_hash === props.region.region_hash) ?? null,
);

const unitText = computed(() => {
  const unit = props.item.unit;
  if (!unit) {
    return null;
  }
  return unit.display_zh ?? unit.display_en ?? unit.zone ?? `子关卡 ${unit.unit}`;
});

const copyLines = computed(() => {
  const lines = [
    `${props.item.display_name}${props.item.name_en ? `（${props.item.name_en}）` : ""}`,
    `状态：${collected.value ? "已收集" : "未收集"} · ${regionName(props.region)}${unitText.value ? ` · ${unitText.value}` : ""}`,
    `怎么拿：${props.item.acquisition_text}`,
    `位置：${props.item.location_hint}`,
  ];
  return lines.join("\n");
});

async function copyHint(): Promise<void> {
  leadText.value = copyLines.value;
  copiedHint.value = await copyText(copyLines.value);
}
</script>

<template>
  <article class="space-y-4 pb-4" data-testid="item-detail">
    <header class="space-y-1">
      <p class="text-xs text-muted">
        {{ store.currentSlot.value?.name ?? "未选择文件" }}
        <template v-if="store.currentSlot.value?.result?.snapshot_time">
          · 快照 {{ formatSnapshotTime(store.currentSlot.value.result.snapshot_time) }}
        </template>
      </p>
      <h2 ref="heading" tabindex="-1" class="text-base leading-snug font-semibold break-words" data-testid="detail-heading">
        {{ item.display_name }}
      </h2>
      <p v-if="item.name_en && item.name_zh" class="text-[13px] text-muted">{{ item.name_en }}</p>
      <p class="mono text-xs text-muted">{{ item.id }}</p>
      <div class="flex flex-wrap items-center gap-1.5">
        <UiBadge :tone="collected ? 'success' : 'caution'" kind-label="状态">
          {{ collected ? "已收集" : "未收集" }}
        </UiBadge>
        <UiBadge tone="neutral" kind-label="类别">{{ group.name }}</UiBadge>
        <UiBadge v-if="item.is_dlc" tone="neutral" kind-label="版本">DLC</UiBadge>
        <UiBadge v-if="item.required_for_100 === false" tone="neutral" kind-label="口径">可选（不计入 100%）</UiBadge>
        <UiBadge v-else-if="item.required_for_100 === null" tone="neutral" kind-label="口径">未闭合（不计入缺失结论）</UiBadge>
      </div>
      <p v-if="item.shared_names && item.shared_names.length > 1" class="text-xs text-caution">
        该 ID 对应 {{ item.shared_names.length }} 个名称：{{ item.shared_names.join(" / ") }}（按 ID 计数，不拆分判定）
      </p>
    </header>

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">去哪里找</h3>
      <p class="text-[13px]" data-testid="detail-location">{{ item.location_hint }}</p>
      <p v-if="!item.position" class="text-xs text-muted">
        该条目没有可用的关卡内相对位置（游戏数据未给出或坐标越界），请按区域与子关卡线索查找。
      </p>
      <PositionScatter
        v-else
        :items="sameUnitItems"
        :selected-id="item.id"
      />
    </section>

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">怎么拿</h3>
      <p class="text-[13px]" data-testid="detail-acquisition">{{ item.acquisition_text }}</p>
      <p class="text-xs text-muted">
        <template v-if="item.acquisition_source === 'item_note'">逐项注记（人工整理，未逐条与游戏内核实）。</template>
        <template v-else>类别通用说明（人工整理，未逐条与游戏内核实）。</template>
      </p>
      <div class="flex flex-wrap items-center gap-2 pt-1">
        <UiButton size="sm" variant="ghost" data-testid="copy-lead" @click="copyHint">
          <Copy :size="16" aria-hidden="true" />
          复制这条线索
        </UiButton>
        <span v-if="copiedHint" class="text-[13px] text-success">已复制到剪贴板。</span>
        <span v-else-if="leadText" class="text-[13px] text-caution">复制失败：可手动选择下方文本。</span>
      </div>
      <pre
        v-if="leadText"
        class="mono max-h-40 overflow-auto rounded-[6px] border border-line bg-raised px-2 py-1 text-xs whitespace-pre-wrap"
        data-testid="lead-text"
        >{{ leadText }}</pre
      >
    </section>

    <section class="space-y-1">
      <h3 class="text-sm font-medium">为什么是这个状态</h3>
      <p class="text-[13px] text-muted" data-testid="detail-status-reason">
        <template v-if="collected">
          该条目 ID 出现在当前存档的物品表中（观察集合），因此判为已收集。
        </template>
        <template v-else>
          该条目 ID 未出现在当前存档的物品表中，因此判为未收集。
          若你在游戏中刚取得，请刷新（F5）后再看。
        </template>
      </p>
      <p class="text-xs text-muted">
        区域与子类别已用存档自带的区域计数器逐行核对（区域 × 类别 43 行全部吻合）。
      </p>
    </section>

    <UiEvidenceDetails summary="原始证据与数据集信息" hint="图鉴版本 / 区域绑定 / 子关卡 / 坐标 / 内部名">
      <p>图鉴版本：{{ store.atlas.catalog_version }}（schema {{ store.atlas.catalog_schema_version }}）</p>
      <p>区域：{{ regionName(region) }}（<span class="mono">{{ region.region_hash }}</span>）<span v-if="regionRecord?.binding">
        · 绑定签名距离 {{ regionRecord.binding.signature_distance }}，次优 {{ regionRecord.binding.runner_up_distance ?? "无" }}
      </span></p>
      <p v-if="regionRecord?.binding">
        计数核对：<span class="mono">{{ JSON.stringify(regionRecord.binding.signature) }}</span>
        vs 存档目标 <span class="mono">{{ JSON.stringify(regionRecord.binding.source_targets) }}</span>
      </p>
      <p v-if="item.unit">
        子关卡：<span class="mono">{{ item.unit.unit }}</span> · zone <span class="mono">{{ item.unit.zone ?? "未知" }}</span>
        （名称来源 {{ item.unit.name_source }}）
      </p>
      <p v-if="item.position">
        原始坐标：x={{ item.position.x }}，y={{ item.position.y }}，空间
        <span class="mono">{{ item.position.space }}</span>（未校准）
      </p>
      <p>内部候选名：<span class="mono">{{ item.internal_name ?? "（无）" }}</span></p>
      <p v-if="item.collection">故事合集：<span class="mono">{{ item.collection }}</span></p>
      <p>类别：{{ groupFact }}<template v-if="item.name_source"> · 名称来源 {{ item.name_source }}</template></p>
      <p>获取方式规则：<span class="mono">{{ group.acquisition_rule_key }}</span></p>
      <p v-if="store.isStale.value">{{ copy.stale }}：本次结果仅反映上一次读取。</p>
    </UiEvidenceDetails>
  </article>
</template>
