<script setup lang="ts">
// 收集品详情（2026-10-17 重设计；阶段三抽出纯展示层 ItemDetailView）：
// 本组件只作为 store 连接层——计算区域/子关卡/类别文案、直链与搜索地址，
// 调用外链动作与「在新窗口中打开」，再把数据交给 ItemDetailView 渲染。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { localizedName, t } from "../app/i18n";
import { categoryNameOf, groupDisplayName, regionName } from "../app/collection";
import { buildSearchUrl, textSearchQuery, videoSearchQuery } from "../app/external-links";
import { externalLinkKey, guideLinkKey } from "../app/store";
import { basemapForUnit } from "../data/basemaps";
import type { PlanGroup, PlanItem, PlanRegion } from "../data/atlas";
import ItemDetailView, {
  type DetailExternalLink,
  type DetailLinkActionView,
} from "./ItemDetailView.vue";

const props = withDefaults(
  defineProps<{
    region: PlanRegion;
    group: PlanGroup;
    item: PlanItem;
    /** pane：宽屏常驻；drawer：模态抽屉；page：窄窗页面替换。 */
    mode?: "pane" | "drawer" | "page";
    /** 是否显示「同一子关卡内的相对位置」段落（地图页已展示位置，默认 true）。 */
    showPosition?: boolean;
    /** 是否显示「在新窗口中打开」（默认 true）。 */
    showPopout?: boolean;
  }>(),
  { mode: "pane", showPosition: true, showPopout: true },
);

const store = useStore();

const regionLabel = computed(() => regionName(props.region));

/** 类别显示名（徽标按类别着色，颜色与主清单分组头同一套）。 */
const categoryLabel = computed(() => categoryNameOf(store.atlas, props.group.category));

/** 分组自己的名字（故事类别按子类别分组）；与类别名相同时不再单独成徽标。 */
const subgroupLabel = computed(() => {
  const name = groupDisplayName(props.group);
  return name === categoryLabel.value ? null : name;
});

const unitText = computed(() => {
  const unit = props.item.unit;
  if (!unit) {
    return null;
  }
  return localizedName(unit.display_zh, unit.display_en, unit.zone ?? t(`子关卡 ${unit.unit}`, `Sub-level ${unit.unit}`));
});

/** 同一子关卡内的条目（含跨区域共享 unit 的条目）。 */
const sameUnitItems = computed<PlanItem[]>(() => {
  const unit = props.item.unit?.unit;
  if (!unit) {
    return [];
  }
  return store.plan.value.regions
    .flatMap((region) => region.groups)
    .flatMap((group) => group.items)
    .filter((entry) => entry.unit?.unit === unit);
});

/** 当前条目所属子关卡的基底层记录（没有则该段落回退散点）。 */
const unitBasemap = computed(() => basemapForUnit(props.item.unit?.unit));

const searchTextUrl = computed(() =>
  buildSearchUrl(store.state.searchEngine, textSearchQuery(props.item, regionLabel.value)),
);
const searchVideoUrl = computed(() =>
  buildSearchUrl(store.state.searchEngine, videoSearchQuery(props.item, regionLabel.value)),
);

/** 直链候选：只保留每条来源的首个地址，界面每个载体最多一个按钮。 */
const directLinks = computed(() => store.directLinksForItem(props.item));
const directText = computed(() => directLinks.value.find((link) => link.medium === "text") ?? null);
const directVideo = computed(() => directLinks.value.find((link) => link.medium === "video") ?? null);

/** 四个按钮：直链（有候选时）+ 搜索（始终有）。 */
const links = computed<DetailExternalLink[]>(() => {
  const result: DetailExternalLink[] = [];
  const text = directText.value;
  if (text) {
    result.push({
      key: guideLinkKey(text.locationId, text.sourceRef),
      actionId: "guide-text",
      label: t("攻略页", "Guide page"),
      url: text.url,
      direct: true,
      testId: "link-guide",
    });
  }
  result.push({
    key: externalLinkKey(searchTextUrl.value),
    actionId: "search-text",
    label: t("搜索图文攻略", "Search text guides"),
    url: searchTextUrl.value,
    direct: false,
    testId: "link-search-text",
  });
  const video = directVideo.value;
  if (video) {
    result.push({
      key: guideLinkKey(video.locationId, video.sourceRef),
      actionId: "guide-video",
      label: t("视频页", "Video page"),
      url: video.url,
      direct: true,
      testId: "link-video",
    });
  }
  result.push({
    key: externalLinkKey(searchVideoUrl.value),
    actionId: "search-video",
    label: t("搜索视频", "Search videos"),
    url: searchVideoUrl.value,
    direct: false,
    testId: "link-search-video",
  });
  return result;
});

const linkActionView = computed<DetailLinkActionView | null>(() => {
  const action = store.state.linkAction;
  if (!action || !links.value.some((link) => link.key === action.key)) {
    return null;
  }
  return { key: action.key, status: action.status, message: action.error?.message ?? null };
});

/** 直链按「定位 + 来源」交给后端从快照元数据解析（保持原有安全边界）。 */
function openLink(actionId: string): void {
  const link = links.value.find((entry) => entry.actionId === actionId);
  if (!link) {
    return;
  }
  const text = directText.value;
  if (actionId === "guide-text" && text) {
    void store.actions.openGuideLink(text.locationId, text.sourceRef, link.label);
    return;
  }
  const video = directVideo.value;
  if (actionId === "guide-video" && video) {
    void store.actions.openGuideLink(video.locationId, video.sourceRef, link.label);
    return;
  }
  void store.actions.openExternalLink(link.url, link.label);
}

const slotName = computed(() => store.currentSlot.value?.name ?? "");
const snapshotTime = computed(() => store.currentSlot.value?.result?.snapshot_time ?? null);
</script>

<template>
  <ItemDetailView
    :item="item"
    :region-name="regionLabel"
    :unit-label="unitText"
    :category-id="group.category"
    :category-label="categoryLabel"
    :subgroup-label="subgroupLabel"
    :slot-name="slotName"
    :snapshot-time="snapshotTime"
    :stale="store.isStale.value"
    :same-unit-items="sameUnitItems"
    :basemap="unitBasemap"
    :links="links"
    :link-action="linkActionView"
    :show-position="showPosition"
    :show-popout="showPopout"
    :mode="mode"
    @open-link="openLink"
    @popout="store.actions.openDetailWindow()"
  />
</template>
