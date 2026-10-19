<script setup lang="ts">
// 收集品详情（2026-10-17 重设计）：头部 → 如何获取 → 外部参考 → 同一子关卡相对位置（默认折叠）。
// 位置是关卡内归一化坐标，未校准；界面不把它写成导航点位，也不画真实地图底图。
// 状态判定说明与原始证据移到「诊断」页的「当前选中条目的原始证据」一节。
import { computed, nextTick, onMounted, ref } from "vue";
import { Copy, ExternalLink, Search } from "@lucide/vue";
import { useStore } from "../app/inject";
import { copyText } from "../app/clipboard";
import { formatSnapshotTime } from "../app/format";
import { regionName, categoryNameOf } from "../app/collection";
import { categoryClass } from "../app/category-style";
import { buildSearchUrl, textSearchQuery, videoSearchQuery } from "../app/external-links";
import { externalLinkKey, guideLinkKey, type DirectLink, type LinkActionState } from "../app/store";
import type { PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import PositionScatter from "./PositionScatter.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiButton from "./ui/UiButton.vue";

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

/** 类别显示名（徽标按类别着色，颜色与主清单分组头同一套）。 */
const categoryLabel = computed(() => categoryNameOf(store.atlas, props.group.category));

/** 分组自己的名字（故事类别按子类别分组）；与类别名相同时不再单独成徽标。 */
const subgroupLabel = computed(() =>
  props.group.name === categoryLabel.value ? null : props.group.name,
);

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

const unitText = computed(() => {
  const unit = props.item.unit;
  if (!unit) {
    return null;
  }
  return unit.display_zh ?? unit.display_en ?? unit.zone ?? `子关卡 ${unit.unit}`;
});

/** 「如何获取」主体：区域 · 子关卡 → 类别规则原文 → 关卡内相对位置 → 同子关卡剩余。 */
const acquisitionBody = computed(() => {
  const parts = [
    `${regionName(props.region)}${unitText.value ? ` · ${unitText.value}` : ""}`,
    props.item.acquisition_text,
  ];
  const position = props.item.position;
  if (position) {
    parts.push(`关卡内相对位置约 (${position.x.toFixed(2)}, ${position.y.toFixed(2)})`);
  }
  const remaining = props.item.same_unit_remaining ?? 0;
  if (remaining > 0) {
    parts.push(`同一子关卡还有 ${remaining} 项未收集`);
  }
  return parts.join(" · ");
});

const copyLines = computed(() =>
  [
    `${props.item.display_name}${props.item.name_en ? `（${props.item.name_en}）` : ""}`,
    `状态：${collected.value ? "已收集" : "未收集"}`,
    `区域：${regionName(props.region)}${unitText.value ? ` · ${unitText.value}` : ""}`,
    `获取：${props.item.acquisition_text}`,
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
    ? store.actions.openExternalLink(searchTextUrl.value, "搜索图文攻略")
    : store.actions.openExternalLink(searchVideoUrl.value, "搜索视频");
}

/** 四个按钮的目标与重试动作（统一显示动作状态；直链无候选时不出现）。 */
const linkButtons = computed<LinkButton[]>(() => {
  const buttons: LinkButton[] = [];
  const text = directText.value;
  if (text) {
    buttons.push({
      key: guideLinkKey(text.locationId, text.sourceRef),
      label: "攻略页",
      retry: () => openDirectLink(text, "攻略页"),
    });
  }
  buttons.push({
    key: externalLinkKey(searchTextUrl.value),
    label: "搜索图文攻略",
    retry: () => openSearch("text"),
  });
  const video = directVideo.value;
  if (video) {
    buttons.push({
      key: guideLinkKey(video.locationId, video.sourceRef),
      label: "视频页",
      retry: () => openDirectLink(video, "视频页"),
    });
  }
  buttons.push({
    key: externalLinkKey(searchVideoUrl.value),
    label: "搜索视频",
    retry: () => openSearch("video"),
  });
  return buttons;
});

function statusText(action: LinkActionState): string {
  switch (action.status) {
    case "resolving":
      return "正在解析链接…";
    case "opening":
      return "正在打开…";
    case "opened":
      return "已在浏览器打开";
    case "copied":
      return "已复制链接";
    case "error":
      return `失败：${action.error?.message ?? "未知错误"}`;
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
        <UiBadge tone="neutral" kind-label="类别" :class="categoryClass(group.category)">
          {{ categoryLabel }}
        </UiBadge>
        <UiBadge v-if="subgroupLabel" tone="neutral" kind-label="子类别">{{ subgroupLabel }}</UiBadge>
        <UiBadge v-if="item.is_dlc" tone="neutral" kind-label="版本">DLC</UiBadge>
        <UiBadge v-if="item.required_for_100 === false" tone="neutral" kind-label="口径">可选（不计入 100%）</UiBadge>
        <UiBadge v-else-if="item.required_for_100 === null" tone="neutral" kind-label="口径">未闭合（不计入缺失结论）</UiBadge>
      </div>
      <p v-if="item.shared_names && item.shared_names.length > 1" class="text-xs text-caution">
        该 ID 对应 {{ item.shared_names.length }} 个名称：{{ item.shared_names.join(" / ") }}（按 ID 计数，不拆分判定）
      </p>
    </header>

    <section class="space-y-1.5">
      <h3 class="text-sm font-medium">如何获取</h3>
      <p class="text-[13px]" data-testid="detail-acquisition">{{ acquisitionBody }}</p>
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

    <section class="space-y-1.5" data-testid="external-links">
      <h3 class="text-sm font-medium">外部参考</h3>
      <div class="flex flex-wrap items-center gap-2">
        <UiButton
          v-if="directText"
          size="sm"
          variant="ghost"
          data-testid="link-guide"
          @click="openDirectLink(directText, '攻略页')"
        >
          <ExternalLink :size="16" aria-hidden="true" />
          攻略页
        </UiButton>
        <UiButton size="sm" variant="ghost" data-testid="link-search-text" @click="openSearch('text')">
          <Search :size="16" aria-hidden="true" />
          搜索图文攻略
        </UiButton>
        <UiButton
          v-if="directVideo"
          size="sm"
          variant="ghost"
          data-testid="link-video"
          @click="openDirectLink(directVideo, '视频页')"
        >
          <ExternalLink :size="16" aria-hidden="true" />
          视频页
        </UiButton>
        <UiButton size="sm" variant="ghost" data-testid="link-search-video" @click="openSearch('video')">
          <Search :size="16" aria-hidden="true" />
          搜索视频
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
          重试
        </UiButton>
      </p>
      <p class="text-xs text-muted">
        在系统浏览器中打开；设置页选「仅复制链接」时只复制地址。应用内始终不加载网页。
      </p>
    </section>

    <details
      v-if="item.position"
      class="rounded-[8px] border border-line bg-raised/60 px-3 py-2"
      data-testid="position-section"
    >
      <summary class="cursor-pointer text-[13px] font-medium">同一子关卡内的相对位置</summary>
      <div class="mt-2 space-y-1.5">
        <PositionScatter :items="sameUnitItems" :selected-id="item.id" />
        <p class="text-xs text-caution">
          未校准：坐标为关卡内归一化值，不是导航点位；仅用于同一子关卡内的相对参照。
        </p>
      </div>
    </details>
  </article>
</template>
