<script setup lang="ts">
// 收集品详情（2026-10-17 重设计）：头部 → 如何获取 → 外部参考 → 同一子关卡相对位置（默认折叠）。
// 位置是关卡内归一化坐标，未校准；界面不把它写成导航点位，也不画真实地图底图。
// 状态判定说明与原始证据移到「诊断」页的「当前选中条目的原始证据」一节。
import { computed, nextTick, onMounted, ref } from "vue";
import { Copy, ExternalLink, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { localizedName, t, tFormat } from "../app/i18n";
import { copyText } from "../app/clipboard";
import { formatSnapshotTime } from "../app/format";
import { regionName, categoryNameOf, groupDisplayName } from "../app/collection";
import { categoryClass } from "../app/category-style";
import { buildSearchUrl, textSearchQuery, videoSearchQuery } from "../app/external-links";
import { externalLinkKey, guideLinkKey, type DirectLink, type LinkActionState } from "../app/store";
import type { PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import { basemapForUnit } from "../data/basemaps";
import MapPanel from "./MapPanel.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";

const props = withDefaults(
  defineProps<{
    region: PlanRegion;
    group: PlanGroup;
    item: PlanItem;
    /** pane：宽屏常驻；drawer：模态抽屉；page：窄窗页面替换。 */
    mode?: "pane" | "drawer" | "page";
    /** 是否显示「同一子关卡内的相对位置」段落（地图页已展示位置，默认 true）。 */
    showPosition?: boolean;
  }>(),
  { mode: "pane", showPosition: true },
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

/** 类别显示名（徽标按类别着色，颜色与主清单分组头同一套）。 */
const categoryLabel = computed(() => categoryNameOf(store.atlas, props.group.category));

/** 分组自己的名字（故事类别按子类别分组）；与类别名相同时不再单独成徽标。 */
const subgroupLabel = computed(() => {
  const name = groupDisplayName(props.group);
  return name === categoryLabel.value ? null : name;
});

/** 同一子关卡内的条目（用于相对位置散点）。 */
const sameUnitItems = computed<PlanItem[]>(() => {
  const unit = props.item.unit?.unit;
  if (!unit) {
    return [];
  }
  return props.region.groups
    .flatMap((group) => group.items)
    .filter((entry) => entry.unit?.unit === unit);
});

/** 当前条目所属子关卡的基底层记录（没有则该段落回退散点）。 */
const unitBasemap = computed(() => basemapForUnit(props.item.unit?.unit));

/** 「相对位置」段落是否展开：底图只在展开后加载（P3-05）。 */
const positionOpen = ref(false);

function onPositionToggle(event: Event) {
  positionOpen.value = (event.currentTarget as HTMLDetailsElement).open;
}

const unitText = computed(() => {
  const unit = props.item.unit;
  if (!unit) {
    return null;
  }
  return localizedName(unit.display_zh, unit.display_en, unit.zone ?? t(`子关卡 ${unit.unit}`, `Sub-level ${unit.unit}`));
});

/** 「如何获取」主体：区域 · 子关卡 → 类别规则原文 → 关卡内相对位置 → 同子关卡剩余。 */
const acquisitionBody = computed(() => {
  const parts = [
    `${regionName(props.region)}${unitText.value ? ` · ${unitText.value}` : ""}`,
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
      value: `${regionName(props.region)}${unitText.value ? ` · ${unitText.value}` : ""}`,
    }),
    tFormat("获取：{value}", "Acquisition: {value}", { value: props.item.acquisition_text }),
  ].join("\n"),
);

async function copyHint(): Promise<void> {
  leadText.value = copyLines.value;
  copiedHint.value = await copyText(copyLines.value);
}

// ---------------------------------------------------------------------------
// 外部参考：直链（元数据来源）+ 搜索（按设置页的搜索引擎合成）
// ---------------------------------------------------------------------------

const searchTextUrl = computed(() =>
  buildSearchUrl(store.state.searchEngine, textSearchQuery(props.item, regionName(props.region))),
);
const searchVideoUrl = computed(() =>
  buildSearchUrl(store.state.searchEngine, videoSearchQuery(props.item, regionName(props.region))),
);

/** 直链候选：只保留每条来源的首个地址，界面每个载体最多一个按钮。 */
const directLinks = computed(() => store.directLinksForItem(props.item));
const directText = computed(() => directLinks.value.find((link) => link.medium === "text") ?? null);
const directVideo = computed(() => directLinks.value.find((link) => link.medium === "video") ?? null);

interface LinkButton {
  key: string;
  label: string;
  retry: () => Promise<void>;
}

function openDirectLink(link: DirectLink, label: string): Promise<void> {
  return store.actions.openGuideLink(link.locationId, link.sourceRef, label);
}

function openSearch(medium: "text" | "video"): Promise<void> {
  return medium === "text"
    ? store.actions.openExternalLink(searchTextUrl.value, t("搜索图文攻略", "Search text guides"))
    : store.actions.openExternalLink(searchVideoUrl.value, t("搜索视频", "Search videos"));
}

/** 四个按钮的目标与重试动作（统一显示动作状态；直链无候选时不出现）。 */
const linkButtons = computed<LinkButton[]>(() => {
  const buttons: LinkButton[] = [];
  const text = directText.value;
  if (text) {
    buttons.push({
      key: guideLinkKey(text.locationId, text.sourceRef),
      label: t("攻略页", "Guide page"),
      retry: () => openDirectLink(text, t("攻略页", "Guide page")),
    });
  }
  buttons.push({
    key: externalLinkKey(searchTextUrl.value),
    label: t("搜索图文攻略", "Search text guides"),
    retry: () => openSearch("text"),
  });
  const video = directVideo.value;
  if (video) {
    buttons.push({
      key: guideLinkKey(video.locationId, video.sourceRef),
      label: t("视频页", "Video page"),
      retry: () => openDirectLink(video, t("视频页", "Video page")),
    });
  }
  buttons.push({
    key: externalLinkKey(searchVideoUrl.value),
    label: t("搜索视频", "Search videos"),
    retry: () => openSearch("video"),
  });
  return buttons;
});

function statusText(action: LinkActionState): string {
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
      return tFormat("失败：{message}", "Failed: {message}", { message: action.error?.message ?? t("未知错误", "unknown error") });
  }
}

const activeLink = computed(() => {
  const action = store.state.linkAction;
  if (!action) {
    return null;
  }
  const button = linkButtons.value.find((entry) => entry.key === action.key);
  return button ? { button, action } : null;
});

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
      <p class="text-xs text-muted">
        {{ store.currentSlot.value?.name ?? t("未选择文件", "No file selected") }}
        <template v-if="store.currentSlot.value?.result?.snapshot_time">
          · {{ t("快照", "snapshot") }} {{ formatSnapshotTime(store.currentSlot.value.result.snapshot_time) }}
        </template>
      </p>
      <h2 ref="heading" tabindex="-1" class="text-base leading-snug font-semibold break-words" data-testid="detail-heading">
        {{ item.display_name }}
      </h2>
      <p class="mono text-xs text-muted">{{ item.id }}</p>
      <div class="flex flex-wrap items-center gap-1.5">
        <UiBadge :tone="collected ? 'success' : 'caution'" :kind-label="t('状态', 'Status')">
          {{ collected ? t("已收集", "Collected") : t("未收集", "Uncollected") }}
        </UiBadge>
        <UiBadge tone="neutral" :kind-label="t('类别', 'Category')" :class="categoryClass(group.category)">
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
          v-if="directText"
          size="sm"
          variant="ghost"
          data-testid="link-guide"
          @click="openDirectLink(directText, t('攻略页', 'Guide page'))"
        >
          <ExternalLink :size="16" aria-hidden="true" />
          {{ t("攻略页", "Guide page") }}
        </UiButton>
        <UiButton size="sm" variant="ghost" data-testid="link-search-text" @click="openSearch('text')">
          <Search :size="16" aria-hidden="true" />
          {{ t("搜索图文攻略", "Search text guides") }}
        </UiButton>
        <UiButton
          v-if="directVideo"
          size="sm"
          variant="ghost"
          data-testid="link-video"
          @click="openDirectLink(directVideo, t('视频页', 'Video page'))"
        >
          <ExternalLink :size="16" aria-hidden="true" />
          {{ t("视频页", "Video page") }}
        </UiButton>
        <UiButton size="sm" variant="ghost" data-testid="link-search-video" @click="openSearch('video')">
          <Search :size="16" aria-hidden="true" />
          {{ t("搜索视频", "Search videos") }}
        </UiButton>
      </div>
      <p v-if="activeLink" class="flex flex-wrap items-center gap-2 text-xs" :class="linkStatusTone" data-testid="link-status">
        {{ activeLink.button.label }}：{{ statusText(activeLink.action) }}
        <UiButton
          v-if="activeLink.action.status === 'error'"
          size="sm"
          variant="ghost"
          data-testid="link-retry"
          @click="activeLink.button.retry()"
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
          :basemap="unitBasemap"
          :active="positionOpen"
        />
      </div>
    </details>
  </article>
</template>
