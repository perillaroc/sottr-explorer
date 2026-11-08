<script lang="ts">
/** 外部参考按钮（主窗口详情与弹窗共用）。 */
export interface DetailExternalLink {
  key: string;
  /** 稳定的动作标识（URL/key 会随搜索引擎变化，动作标识不变）。 */
  actionId: string;
  label: string;
  url: string;
  /** 直链用外链图标，搜索用放大镜图标。 */
  direct: boolean;
  /** 既有详情测试沿用的定位 ID（弹窗可不传）。 */
  testId?: string;
}

/** 外链动作的展示状态（宿主导出 store 或弹窗本地状态）。 */
export interface DetailLinkActionView {
  key: string;
  status: "resolving" | "opening" | "opened" | "copied" | "error";
  message?: string | null;
}
</script>

<script setup lang="ts">
// 收集品详情的纯展示层（阶段三抽出）：头部 → 如何获取 → 外部参考 → 相对位置（默认折叠）。
// 主窗口（CollectionDetail 包装 store）与详情弹窗（DetailWindow）共用同一套区块与样式；
// 位置是关卡内归一化坐标，未校准；界面不把它写成导航点位。
import { computed, nextTick, onMounted, ref } from "vue";
import { ChevronDown, Copy, ExternalLink, PanelRightOpen, Search } from "@lucide/vue";
import { t, tFormat } from "../app/i18n";
import { collectionDisplayName } from "../app/guidance";
import { copyText } from "../app/clipboard";
import { formatSnapshotTime } from "../app/format";
import { categoryClass } from "../app/category-style";
import type { Basemap } from "../data/basemaps";
import type { PlanItem } from "../data/atlas";
import MapPanel from "./MapPanel.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";
import UiIconButton from "./ui/UiIconButton.vue";

const props = withDefaults(
  defineProps<{
    item: PlanItem;
    /** 区域显示名（当前语言）。 */
    regionName: string;
    /** 子关卡显示名（无子关卡时为 null）。 */
    unitLabel: string | null;
    categoryId: string;
    categoryLabel: string;
    subgroupLabel: string | null;
    slotName: string;
    snapshotTime: string | null;
    /** 报告是否为过期快照（弹窗会显式标注）。 */
    stale?: boolean;
    /** 同一子关卡内的条目（用于相对位置底图/散点）。 */
    sameUnitItems: PlanItem[];
    basemap: Basemap | null;
    links: DetailExternalLink[];
    linkAction?: DetailLinkActionView | null;
    showPosition?: boolean;
    /** 是否显示「在新窗口中打开」（弹窗自身不显示）。 */
    showPopout?: boolean;
    /** pane：宽屏常驻；drawer：模态抽屉；page：窄窗页面替换；window：独立弹窗。 */
    mode?: "pane" | "drawer" | "page" | "window";
  }>(),
  {
    stale: false,
    linkAction: null,
    showPosition: true,
    showPopout: true,
    mode: "pane",
  },
);

const emit = defineEmits<{
  "open-link": [actionId: string];
  popout: [];
}>();

const heading = ref<HTMLElement | null>(null);
const copiedHint = ref(false);
const leadText = ref<string | null>(null);
const positionOpen = ref(false);

onMounted(() => {
  if (props.mode === "page") {
    void nextTick(() => heading.value?.focus());
  }
});

const collected = computed(() => props.item.status === "collected");
const isWindow = computed(() => props.mode === "window");
const showTopRow = computed(() => isWindow.value || (props.showPopout && !isWindow.value));

/** 同一子关卡剩余未收集数。 */
const sameUnitRemaining = computed(() => props.item.same_unit_remaining ?? 0);

/** 「如何获取」行标签：子关卡指引不是获取方式，避免写成「获取」。 */
const acquisitionRowLabel = computed(() =>
  props.item.acquisition_source === "unit_note" ? t("指引", "Guidance") : t("获取", "How"),
);

/** 注记来源标签。 */
const acquisitionSourceLabel = computed(() => {
  switch (props.item.acquisition_source) {
    case "item_note":
      return t("逐项注记", "Per-item note");
    case "collection_note":
      return t("合集指引", "Collection guidance");
    case "unit_note":
      return t("子关卡指引", "Sub-level guidance");
    default:
      return t("类别规则", "Category rule");
  }
});

const copyLines = computed(() =>
  [
    `${props.item.display_name}`,
    t(`状态：${collected.value ? "已收集" : "未收集"}`, `Status: ${collected.value ? "collected" : "uncollected"}`),
    tFormat("区域：{value}", "Region: {value}", {
      value: `${props.regionName}${props.unitLabel ? ` · ${props.unitLabel}` : ""}`,
    }),
    ...(props.item.collection
      ? [
          tFormat("合集：{value}", "Collection: {value}", {
            value: collectionDisplayName(props.item.collection),
          }),
        ]
      : []),
    tFormat("{label}：{value}", "{label}: {value}", {
      label: acquisitionRowLabel.value,
      value: props.item.acquisition_text,
    }),
  ].join("\n"),
);

async function copyHint(): Promise<void> {
  leadText.value = copyLines.value;
  copiedHint.value = await copyText(copyLines.value);
}

function onPositionToggle(event: Event): void {
  positionOpen.value = (event.currentTarget as HTMLDetailsElement).open;
}

const activeLink = computed(() => {
  const action = props.linkAction;
  if (!action) {
    return null;
  }
  const link = props.links.find((entry) => entry.key === action.key);
  return link ? { link, action } : null;
});

function statusText(action: DetailLinkActionView): string {
  switch (action.status) {
    case "resolving":
      return t("正在解析链接…", "Resolving link…");
    case "opening":
      return t("正在打开…", "Opening…");
    case "opened":
      return t("已在浏览器打开", "Opened in browser");
    case "copied":
      return t("已复制链接", "Link copied");
    case "error":
      return tFormat("失败：{message}", "Failed: {message}", {
        message: action.message ?? t("未知错误", "unknown error"),
      });
  }
}

const linkStatusTone = computed(() => {
  const status = activeLink.value?.action.status;
  return status === "error"
    ? "text-danger"
    : status === "opened" || status === "copied"
      ? "text-success"
      : "text-muted";
});
</script>

<template>
  <article class="space-y-5 pb-4" data-testid="item-detail">
    <header class="space-y-2">
      <div
        v-if="showTopRow"
        class="flex items-start gap-2"
        :class="isWindow ? 'justify-between' : 'justify-end'"
      >
        <p v-if="isWindow" class="min-w-0 text-xs text-muted">
          {{ slotName || t("未选择文件", "No file selected") }}
          <template v-if="snapshotTime">
            · {{ t("快照", "snapshot") }} {{ formatSnapshotTime(snapshotTime) }}
          </template>
        </p>
        <UiIconButton
          v-if="showPopout && !isWindow"
          :label="t('在新窗口中打开', 'Open in a new window')"
          data-testid="detail-popout"
          @click="emit('popout')"
        >
          <PanelRightOpen :size="16" aria-hidden="true" />
        </UiIconButton>
      </div>
      <h2 ref="heading" tabindex="-1" class="ui-detail-title break-words" data-testid="detail-heading">
        {{ item.display_name }}
      </h2>
      <p class="mono break-all text-xs text-muted">{{ item.id }}</p>
      <div class="flex flex-wrap items-center gap-1.5">
        <UiBadge :tone="collected ? 'success' : 'caution'" :kind-label="t('状态', 'Status')">
          {{ collected ? t("已收集", "Collected") : t("未收集", "Uncollected") }}
        </UiBadge>
        <UiBadge v-if="stale" tone="caution" :kind-label="t('快照', 'Snapshot')">{{ t("旧快照", "Stale snapshot") }}</UiBadge>
        <UiBadge tone="neutral" :kind-label="t('类别', 'Category')" :class="categoryClass(categoryId)">
          {{ categoryLabel }}
        </UiBadge>
        <UiBadge v-if="subgroupLabel" tone="neutral" :kind-label="t('子类别', 'Subcategory')">{{ subgroupLabel }}</UiBadge>
        <UiBadge v-if="item.is_dlc" tone="neutral" :kind-label="t('版本', 'Edition')">DLC</UiBadge>
        <UiBadge v-if="item.required_for_100 === false" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("可选", "Optional") }}</UiBadge>
        <UiBadge v-else-if="item.required_for_100 === null" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("未闭合", "Unresolved") }}</UiBadge>
      </div>
      <p v-if="item.shared_names && item.shared_names.length > 1" class="text-xs text-caution">
        {{ tFormat("该 ID 对应 {count} 个名称：{names}（按 ID 计数，不拆分判定）", "This ID maps to {count} names: {names} (counted by ID; not split for assessment)", { count: item.shared_names.length, names: item.shared_names.join(" / ") }) }}
      </p>
    </header>

    <section class="rounded-section bg-raised/50 px-3 py-3" data-testid="detail-acquisition-section">
      <h3 class="ui-section-title">{{ t("如何获取", "How to obtain") }}</h3>
      <dl class="mt-2 space-y-1.5 text-[14px] leading-[22px]" data-testid="detail-acquisition">
        <div class="flex gap-2">
          <dt class="w-14 shrink-0 text-[13px] leading-[22px] text-muted">{{ t("区域", "Region") }}</dt>
          <dd class="min-w-0">{{ regionName }}{{ unitLabel ? ` · ${unitLabel}` : "" }}</dd>
        </div>
        <div v-if="item.collection" class="flex gap-2">
          <dt class="w-14 shrink-0 text-[13px] leading-[22px] text-muted">{{ t("合集", "Collection") }}</dt>
          <dd class="min-w-0">{{ collectionDisplayName(item.collection) }}</dd>
        </div>
        <div class="flex gap-2">
          <dt class="w-14 shrink-0 text-[13px] leading-[22px] text-muted">{{ acquisitionRowLabel }}</dt>
          <dd class="min-w-0">{{ item.acquisition_text }}</dd>
        </div>
        <div v-if="item.position" class="flex gap-2">
          <dt class="w-14 shrink-0 text-[13px] leading-[22px] text-muted">{{ t("位置", "Where") }}</dt>
          <dd class="min-w-0">
            {{ t(
              `关卡内相对位置约 (${item.position.x.toFixed(2)}, ${item.position.y.toFixed(2)})`,
              `Approx. in-level position (${item.position.x.toFixed(2)}, ${item.position.y.toFixed(2)})`,
            ) }}
          </dd>
        </div>
        <div v-if="sameUnitRemaining > 0" class="flex gap-2">
          <dt class="w-14 shrink-0 text-[13px] leading-[22px] text-muted">{{ t("同关卡", "Same level") }}</dt>
          <dd class="min-w-0">{{ t(`还有 ${sameUnitRemaining} 项未收集`, `${sameUnitRemaining} more uncollected`) }}</dd>
        </div>
      </dl>
      <p class="mt-1 text-xs text-muted">
        {{ acquisitionSourceLabel }}
      </p>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        <UiButton size="sm" variant="ghost" data-testid="copy-lead" @click="copyHint">
          <Copy :size="16" aria-hidden="true" />
          {{ t("复制这条线索", "Copy this hint") }}
        </UiButton>
        <span v-if="copiedHint" class="text-[13px] text-success" role="status" aria-live="polite">{{ t("已复制到剪贴板。", "Copied to clipboard.") }}</span>
        <span v-else-if="leadText" class="text-[13px] text-caution">{{ t("复制失败：可手动选择下方文本。", "Copy failed: select the text below manually.") }}</span>
      </div>
      <pre
        v-if="leadText"
        class="mono mt-2 max-h-40 overflow-auto rounded-control border border-line bg-surface px-2 py-1 text-xs whitespace-pre-wrap"
        data-testid="lead-text"
        >{{ leadText }}</pre
      >
    </section>

    <section class="space-y-2" data-testid="external-links">
      <h3 class="ui-section-title">{{ t("外部参考", "External references") }}</h3>
      <div class="flex flex-wrap items-center gap-2">
        <UiButton
          v-for="link in links"
          :key="link.key"
          size="sm"
          :variant="link.direct ? 'secondary' : 'ghost'"
          :data-testid="link.testId"
          @click="emit('open-link', link.actionId)"
        >
          <ExternalLink v-if="link.direct" :size="16" aria-hidden="true" />
          <Search v-else :size="16" aria-hidden="true" />
          {{ link.label }}
        </UiButton>
      </div>
      <p v-if="activeLink" class="flex flex-wrap items-center gap-2 text-[12px]" :class="linkStatusTone" data-testid="link-status">
        {{ activeLink.link.label }}：{{ statusText(activeLink.action) }}
        <UiButton
          v-if="activeLink.action.status === 'error'"
          size="sm"
          variant="ghost"
          data-testid="link-retry"
          @click="emit('open-link', activeLink.link.actionId)"
        >
          {{ t("重试", "Retry") }}
        </UiButton>
      </p>
    </section>

    <details
      v-if="showPosition && item.position"
      class="ui-disclosure rounded-section border border-line bg-raised/50 px-3 py-2"
      data-testid="position-section"
      :open="positionOpen"
      @toggle="onPositionToggle"
    >
      <summary class="flex cursor-pointer items-center justify-between gap-2 text-[13px] font-medium">
        {{ t("相对位置", "Relative positions") }}
        <ChevronDown class="ui-chevron" :size="16" aria-hidden="true" />
      </summary>
      <div class="mt-2 space-y-1.5">
        <MapPanel
          :items="sameUnitItems"
          :selected-id="item.id"
          :basemap="basemap"
          :active="positionOpen"
        />
      </div>
    </details>
  </article>
</template>
