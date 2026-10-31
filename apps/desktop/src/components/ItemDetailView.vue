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
import { Copy, ExternalLink, PanelRightOpen, Search } from "@lucide/vue";
import { t, tFormat } from "../app/i18n";
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

/** 「如何获取」主体：区域 · 子关卡 → 类别规则原文 → 关卡内相对位置 → 同子关卡剩余。 */
const acquisitionBody = computed(() => {
  const parts = [
    `${props.regionName}${props.unitLabel ? ` · ${props.unitLabel}` : ""}`,
    props.item.acquisition_text,
  ];
  const position = props.item.position;
  if (position) {
    parts.push(
      t(
        `关卡内相对位置约 (${position.x.toFixed(2)}, ${position.y.toFixed(2)})`,
        `Approx. in-level position (${position.x.toFixed(2)}, ${position.y.toFixed(2)})`,
      ),
    );
  }
  const remaining = props.item.same_unit_remaining ?? 0;
  if (remaining > 0) {
    parts.push(
      t(`同一子关卡还有 ${remaining} 项未收集`, `${remaining} more uncollected in the same sub-level`),
    );
  }
  return parts.join(" · ");
});

const copyLines = computed(() =>
  [
    `${props.item.display_name}`,
    t(`状态：${collected.value ? "已收集" : "未收集"}`, `Status: ${collected.value ? "collected" : "uncollected"}`),
    tFormat("区域：{value}", "Region: {value}", {
      value: `${props.regionName}${props.unitLabel ? ` · ${props.unitLabel}` : ""}`,
    }),
    tFormat("获取：{value}", "Acquisition: {value}", { value: props.item.acquisition_text }),
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
  <article class="space-y-4 pb-4" data-testid="item-detail">
    <header class="space-y-1">
      <div class="flex items-start justify-between gap-2">
        <p class="min-w-0 text-xs text-muted">
          {{ slotName || t("未选择文件", "No file selected") }}
          <template v-if="snapshotTime">
            · {{ t("快照", "snapshot") }} {{ formatSnapshotTime(snapshotTime) }}
          </template>
        </p>
        <UiIconButton
          v-if="showPopout && mode !== 'window'"
          :label="t('在新窗口中打开', 'Open in a new window')"
          data-testid="detail-popout"
          @click="emit('popout')"
        >
          <PanelRightOpen :size="16" aria-hidden="true" />
        </UiIconButton>
      </div>
      <h2 ref="heading" tabindex="-1" class="text-base leading-snug font-semibold break-words" data-testid="detail-heading">
        {{ item.display_name }}
      </h2>
      <p class="mono text-xs text-muted">{{ item.id }}</p>
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
        <UiBadge v-if="item.required_for_100 === false" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("可选（不计入 100%）", "Optional (not counted toward 100%)") }}</UiBadge>
        <UiBadge v-else-if="item.required_for_100 === null" tone="neutral" :kind-label="t('口径', 'Scope')">{{ t("未闭合（不计入缺失结论）", "Unresolved (excluded from missing conclusions)") }}</UiBadge>
      </div>
      <p v-if="item.shared_names && item.shared_names.length > 1" class="text-xs text-caution">
        {{ tFormat("该 ID 对应 {count} 个名称：{names}（按 ID 计数，不拆分判定）", "This ID maps to {count} names: {names} (counted by ID; not split for assessment)", { count: item.shared_names.length, names: item.shared_names.join(" / ") }) }}
      </p>
    </header>

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">{{ t("如何获取", "How to obtain") }}</h3>
      <p class="text-[13px]" data-testid="detail-acquisition">{{ acquisitionBody }}</p>
      <p class="text-xs text-muted">
        <template v-if="item.acquisition_source === 'item_note'">{{ t("逐项注记（人工整理，未逐条与游戏内核实）。", "Per-item note (curated; not verified item by item in-game).") }}</template>
        <template v-else>{{ t("类别通用说明（人工整理，未逐条与游戏内核实）。", "Category-level note (curated; not verified item by item in-game).") }}</template>
      </p>
      <div class="flex flex-wrap items-center gap-2 pt-1">
        <UiButton size="sm" variant="ghost" data-testid="copy-lead" @click="copyHint">
          <Copy :size="16" aria-hidden="true" />
          {{ t("复制这条线索", "Copy this hint") }}
        </UiButton>
        <span v-if="copiedHint" class="text-[13px] text-success">{{ t("已复制到剪贴板。", "Copied to clipboard.") }}</span>
        <span v-else-if="leadText" class="text-[13px] text-caution">{{ t("复制失败：可手动选择下方文本。", "Copy failed: select the text below manually.") }}</span>
      </div>
      <pre
        v-if="leadText"
        class="mono max-h-40 overflow-auto rounded-[6px] border border-line bg-raised px-2 py-1 text-xs whitespace-pre-wrap"
        data-testid="lead-text"
        >{{ leadText }}</pre
      >
    </section>

    <section class="space-y-1.5" data-testid="external-links">
      <h3 class="text-sm font-medium">{{ t("外部参考", "External references") }}</h3>
      <div class="flex flex-wrap items-center gap-2">
        <UiButton
          v-for="link in links"
          :key="link.key"
          size="sm"
          variant="ghost"
          :data-testid="link.testId"
          @click="emit('open-link', link.actionId)"
        >
          <ExternalLink v-if="link.direct" :size="16" aria-hidden="true" />
          <Search v-else :size="16" aria-hidden="true" />
          {{ link.label }}
        </UiButton>
      </div>
      <p v-if="activeLink" class="flex flex-wrap items-center gap-2 text-xs" :class="linkStatusTone" data-testid="link-status">
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
      <p class="text-xs text-muted">
        {{ t("在系统浏览器中打开；设置页选「仅复制链接」时只复制地址。应用内始终不加载网页。", "Opens in the system browser; with “Copy link only” selected in Settings, only the address is copied. The app never loads web pages in-app.") }}
      </p>
    </section>

    <details
      v-if="showPosition && item.position"
      class="rounded-[8px] border border-line bg-raised/60 px-3 py-2"
      data-testid="position-section"
      :open="positionOpen"
      @toggle="onPositionToggle"
    >
      <summary class="cursor-pointer text-[13px] font-medium">{{ t("同一子关卡内的相对位置", "Relative positions within the same sub-level") }}</summary>
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
