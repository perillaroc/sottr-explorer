<script setup lang="ts">
// 地图右栏（阶段二）：「详情 / 清单」两个标签页，各自保留滚动位置。
// - 点击地图图标由父级切到「详情」；点击清单项只选中并高亮地图、停留在「清单」；
// - 详情复用 CollectionDetail，并隐藏「相对位置」段落（地图本身已展示位置）；
// - 清单底部说明本图无坐标条目与本区域无底图条目，并提供回到区域清单的入口。
import { computed, nextTick, ref, watch } from "vue";
import { t, tFormat, localizedName, tFormatPair } from "../../app/i18n";
import { statusMarkOf } from "../../app/collection";
import { categoryClass } from "../../app/category-style";
import { categoryIcon } from "../../app/category-icon";
import type { MapEntry, MapGroupView } from "../../app/map-view";
import CollectionDetail from "../CollectionDetail.vue";
import UiButton from "../ui/UiButton.vue";
import UiStatusMark from "../ui/UiStatusMark.vue";

const props = withDefaults(
  defineProps<{
    entries: MapEntry[];
    groups: MapGroupView[];
    selectedId: string | null;
    tab: "detail" | "list";
    /** 本图 unit 内没有坐标的条目数。 */
    unplaced: number;
    /** 本区域没有底图的条目数。 */
    unmapped: number;
    /** 窄窗清单行尾显示「详情」按钮（打开全宽详情浮层）。 */
    showRowDetail?: boolean;
    /** 只渲染清单（窄窗折叠区）；不显示标签栏与详情面板。 */
    listOnly?: boolean;
  }>(),
  { showRowDetail: false, listOnly: false },
);

const emit = defineEmits<{
  "update:tab": [tab: "detail" | "list"];
  select: [id: string];
  "open-detail": [id: string];
  "open-region": [];
  "clear-filters": [];
}>();

const detailTab = ref<HTMLButtonElement | null>(null);
const listTab = ref<HTMLButtonElement | null>(null);
const detailScroll = ref<HTMLElement | null>(null);
const listScroll = ref<HTMLElement | null>(null);

const visibleIds = computed(
  () => new Set(props.groups.flatMap((group) => group.items.map((entry) => entry.item.id))),
);

const selectedEntry = computed(
  () => props.entries.find((entry) => entry.item.id === props.selectedId) ?? null,
);

const selectedVisible = computed(
  () => props.selectedId !== null && visibleIds.value.has(props.selectedId),
);

function selectTab(tab: "detail" | "list"): void {
  if (tab === props.tab) {
    return;
  }
  emit("update:tab", tab);
}

function onTabKeydown(event: KeyboardEvent): void {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
    return;
  }
  event.preventDefault();
  const next = props.tab === "detail" ? "list" : "detail";
  selectTab(next);
  void nextTick(() => (next === "detail" ? detailTab : listTab).value?.focus());
}

// 切换条目标时：详情面板回到顶部；清单把该项滚动到可视位置（不切换标签）。
watch(
  () => props.selectedId,
  async (id) => {
    if (id === null) {
      return;
    }
    await nextTick();
    if (detailScroll.value) {
      detailScroll.value.scrollTop = 0;
    }
    const row = listScroll.value?.querySelector<HTMLElement>(`[data-item-id="${id}"]`);
    row?.scrollIntoView?.({ block: "nearest" });
  },
);
</script>

<template>
  <div class="flex h-full min-h-0 flex-col bg-surface" data-testid="map-rail">
    <div
      v-if="!listOnly"
      role="tablist"
      :aria-label="t('mapPage.map-details-and-list')"
      class="flex h-11 shrink-0 items-stretch border-b border-line"
    >
      <button
        id="map-tab-detail"
        ref="detailTab"
        type="button"
        role="tab"
        :aria-selected="tab === 'detail'"
        aria-controls="map-panel-detail"
        :tabindex="tab === 'detail' ? 0 : -1"
        class="flex-1 cursor-pointer border-b-2 px-3 text-[13px]"
        :class="tab === 'detail' ? 'border-accent text-accent-strong' : 'border-transparent text-muted hover:bg-raised'"
        data-testid="map-tab-detail"
        @click="selectTab('detail')"
        @keydown="onTabKeydown"
      >
        {{ t("common.details") }}
      </button>
      <button
        id="map-tab-list"
        ref="listTab"
        type="button"
        role="tab"
        :aria-selected="tab === 'list'"
        aria-controls="map-panel-list"
        :tabindex="tab === 'list' ? 0 : -1"
        class="flex-1 cursor-pointer border-b-2 px-3 text-[13px]"
        :class="tab === 'list' ? 'border-accent text-accent-strong' : 'border-transparent text-muted hover:bg-raised'"
        data-testid="map-tab-list"
        @click="selectTab('list')"
        @keydown="onTabKeydown"
      >
        {{ tFormatPair(`mapPage.list`, { v0: entries.length }) }}
      </button>
    </div>

    <div
      v-show="!listOnly && tab === 'detail'"
      id="map-panel-detail"
      ref="detailScroll"
      role="tabpanel"
      aria-labelledby="map-tab-detail"
      class="scroll-area min-h-0 flex-1 px-4 py-3"
      data-testid="map-detail-panel"
    >
      <CollectionDetail
        v-if="selectedEntry && selectedVisible"
        :region="selectedEntry.region"
        :group="selectedEntry.group"
        :item="selectedEntry.item"
        mode="pane"
        :show-position="false"
      />
      <div v-else-if="selectedEntry" class="space-y-2" data-testid="map-detail-filtered">
        <p class="text-[13px] text-muted">
          {{ t("mapRail.this-item-is-currently-hidden") }}
        </p>
        <UiButton size="sm" @click="emit('clear-filters')">{{ t("common.clear-filters") }}</UiButton>
      </div>
      <p v-else class="text-[13px] text-muted" data-testid="map-detail-empty">
        {{ t("mapRail.select-a-marker-on-the-map") }}
      </p>
    </div>

    <div
      v-show="listOnly || tab === 'list'"
      id="map-panel-list"
      ref="listScroll"
      role="tabpanel"
      aria-labelledby="map-tab-list"
      class="scroll-area min-h-0 flex-1 px-2 py-2"
      data-testid="map-list-panel"
    >
      <div v-if="entries.length === 0" class="space-y-2 px-2 py-3" data-testid="map-list-empty">
        <p class="text-[13px]">{{ t("mapPage.this-sub-level-has-no-collectibles") }}</p>
      </div>
      <div v-else-if="groups.length === 0" class="space-y-2 px-2 py-3" data-testid="map-list-filtered-empty">
        <p class="text-[13px]">{{ t("mapRail.no-items-match-the-current-filters") }}</p>
        <UiButton size="sm" @click="emit('clear-filters')">{{ t("common.clear-filters") }}</UiButton>
      </div>

      <section v-for="group in groups" :key="`${group.category}-${group.subcategory ?? ''}`" class="mb-3">
        <header class="mb-1 flex flex-wrap items-baseline gap-1.5 border-b border-line/60 px-2 pb-1">
          <component
            :is="categoryIcon(group.category)"
            :size="14"
            class="shrink-0 self-center"
            :class="categoryClass(group.category)"
            aria-hidden="true"
          />
          <h3 class="text-[13px] font-semibold" :class="categoryClass(group.category)">{{ group.label }}</h3>
          <span class="text-xs tabular-nums text-muted">{{ group.collected }}/{{ group.total }}</span>
        </header>
        <ul>
          <li
            v-for="entry in group.items"
            :key="entry.item.id"
            class="flex items-stretch border-b border-line/50 last:border-b-0"
            :class="selectedId === entry.item.id ? 'ui-row-selected' : 'hover:bg-raised'"
          >
            <button
              type="button"
              class="flex min-h-10 min-w-0 flex-1 cursor-pointer items-start gap-2 px-2 py-1.5 text-left"
              :data-testid="`map-row-${entry.item.id}`"
              :data-item-id="entry.item.id"
              @click="emit('select', entry.item.id)"
            >
              <component
                :is="categoryIcon(entry.group.category)"
                :size="15"
                class="mt-0.5 shrink-0"
                :class="categoryClass(entry.group.category)"
                aria-hidden="true"
              />
              <span class="min-w-0 flex-1">
                <span
                  class="line-clamp-2 block break-words text-[13px] leading-5"
                  :class="selectedId === entry.item.id ? 'font-medium text-ink' : ''"
                >{{ entry.item.display_name }}</span>
                <span class="mt-0.5 block text-xs text-muted">
                  {{ entry.item.unit ? localizedName(entry.item.unit.display_zh, entry.item.unit.display_en, entry.item.unit.zone ?? "") : "" }}
                </span>
              </span>
              <span
                class="flex shrink-0 items-center gap-1 self-center text-xs whitespace-nowrap"
                :class="entry.item.status === 'collected' ? 'text-success' : 'text-caution'"
              >
                <UiStatusMark :status="statusMarkOf(entry.item)" decorative />
                {{ entry.item.status === "collected" ? t("common.collected") : t("common.uncollected") }}
              </span>
            </button>
            <button
              v-if="showRowDetail"
              type="button"
              class="ui-interactive my-1.5 mr-1 shrink-0 cursor-pointer self-center rounded-control px-2 py-0.5 text-xs text-accent-strong hover:bg-surface/70"
              :aria-label="tFormat('mapRail.view-details',  { name: entry.item.display_name })"
              :data-testid="`map-row-detail-${entry.item.id}`"
              @click="emit('open-detail', entry.item.id)"
            >
              {{ t("common.details") }}
            </button>
          </li>
        </ul>
      </section>

      <footer class="mt-2 space-y-1.5 border-t border-line px-2 pt-2 text-xs text-muted">
        <p class="tabular-nums" data-testid="map-list-unplaced">
          {{ tFormatPair(`mapRail.more-items-in-this-map-have-no`, { v0: unplaced }) }}
        </p>
        <p class="tabular-nums" data-testid="map-list-unmapped">
          {{ tFormatPair(`mapRail.more-items-in-this-region-have-no`, { v0: unmapped }) }}
        </p>
        <UiButton size="sm" variant="ghost" data-testid="map-open-region" @click="emit('open-region')">
          {{ t("mapRail.view-this-region-in-the-regional-list") }}
        </UiButton>
      </footer>
    </div>
  </div>
</template>
