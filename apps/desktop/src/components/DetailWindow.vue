<script setup lang="ts">
// 详情弹窗内容（阶段三）：读取只读 payload、同子关卡清单、详情与外部链接。
// 支持两种宿主：`window`（Tauri 独立窗口 / 浏览器弹窗）与 `inline`（主窗口兜底浮层）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { X } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { t, tFormat } from "../app/i18n";
import { isTauriRuntime } from "../data/tauri";
import { copyText } from "../app/clipboard";
import { toCommandError } from "../data/errors";
import { categoryIcon } from "../app/category-icon";
import { categoryClass } from "../app/category-style";
import {
  DetailPayloadError,
  parseDetailWindowPayload,
  readDetailWindowRaw,
  subscribeDetailWindowUpdates,
  type DetailWindowItem,
  type DetailWindowPayload,
} from "../app/detail-window";
import type { PlanItem } from "../data/atlas";
import ItemDetailView, {
  type DetailExternalLink,
  type DetailLinkActionView,
} from "./ItemDetailView.vue";
import UiBadge from "./ui/UiBadge.vue";
import UiIconButton from "./ui/UiIconButton.vue";

const props = withDefaults(
  defineProps<{
    /** 应用内兜底时由主窗口直接传入；窗口模式从 Rust 内存/浏览器存储读取。 */
    payload?: DetailWindowPayload | null;
    mode?: "window" | "inline";
  }>(),
  { payload: null, mode: "window" },
);

const emit = defineEmits<{ close: [] }>();

const current = ref<DetailWindowPayload | null>(props.mode === "inline" ? props.payload : null);
const errorKind = ref<"missing" | "invalid" | "version" | "read" | null>(
  props.mode === "inline" && !props.payload ? "missing" : null,
);
const selectedId = ref<string | null>(props.payload?.selectedId ?? null);
const linkAction = ref<DetailLinkActionView | null>(null);

let unsubscribe: (() => void) | null = null;

function apply(payload: DetailWindowPayload): void {
  current.value = payload;
  selectedId.value = payload.selectedId;
  errorKind.value = null;
}

watch(
  () => props.payload,
  (next) => {
    if (props.mode !== "inline" || !next) {
      return;
    }
    apply(next);
  },
);

onMounted(async () => {
  if (props.mode !== "window") {
    return;
  }
  // 先订阅再读取：避免读取与推送之间的竞态（事件丢失时读取仍拿到最新内存值）。
  try {
    unsubscribe = await subscribeDetailWindowUpdates((next) => apply(next));
  } catch {
    // 监听不可用不阻断初始读取；错误在读取失败时统一显示。
  }
  try {
    if (!current.value) {
      apply(parseDetailWindowPayload(await readDetailWindowRaw()));
    }
  } catch (error) {
    if (current.value) {
      return;
    }
    errorKind.value =
      error instanceof DetailPayloadError
        ? error.kind
        : error instanceof Error && error.message === "missing"
          ? "missing"
          : "read";
  }
});

onBeforeUnmount(() => {
  unsubscribe?.();
  unsubscribe = null;
});

const items = computed<DetailWindowItem[]>(() => current.value?.items ?? []);

const selectedItem = computed<DetailWindowItem | null>(
  () => items.value.find((item) => item.id === selectedId.value) ?? items.value[0] ?? null,
);

/** 把 payload 条目适配成详情的 PlanItem（只含展示所需字段）。 */
const selectedPlanItem = computed<PlanItem | null>(() => {
  const item = selectedItem.value;
  if (!item) {
    return null;
  }
  return {
    id: item.id,
    display_name: item.displayName,
    name_en: item.nameEn,
    name_zh: item.nameZh,
    internal_name: item.internalName,
    status: item.status,
    is_dlc: item.isDlc,
    required_for_100: item.requiredFor100,
    collection: item.collection,
    position: item.position ? { ...item.position, space: "unit_normalized_unverified" } : undefined,
    acquisition_text: item.acquisitionText,
    acquisition_source: item.acquisitionSource,
    location_hint: "",
    same_unit_remaining: item.sameUnitRemaining,
    shared_names: item.sharedNames,
  };
});

const sameUnitItems = computed<PlanItem[]>(() =>
  items.value.map((item) => ({
    id: item.id,
    display_name: item.displayName,
    status: item.status,
    is_dlc: item.isDlc,
    required_for_100: item.requiredFor100,
    position: item.position ? { ...item.position, space: "unit_normalized_unverified" } : undefined,
    acquisition_text: item.acquisitionText,
    acquisition_source: item.acquisitionSource,
    location_hint: "",
  })),
);

const links = computed<DetailExternalLink[]>(() => {
  const item = selectedItem.value;
  if (!item) {
    return [];
  }
  const result: DetailExternalLink[] = item.directLinks.map((link, index) => ({
    key: `url:${link.url}`,
    actionId: `guide-${index}`,
    label: link.label,
    url: link.url,
    direct: true,
    guideItem: link.guideItem,
  }));
  result.push({
    key: `url:${item.searchTextUrl}`,
    actionId: "search-text",
    label: t("collectionDetail.search-text-guides"),
    url: item.searchTextUrl,
    direct: false,
  });
  result.push({
    key: `url:${item.searchVideoUrl}`,
    actionId: "search-video",
    label: t("collectionDetail.search-videos"),
    url: item.searchVideoUrl,
    direct: false,
  });
  return result;
});

async function openLink(actionId: string): Promise<void> {
  const link = links.value.find((entry) => entry.actionId === actionId);
  if (!link) {
    return;
  }
  // 与主窗口一致：设置页选「仅复制链接」时只复制地址，不唤起浏览器。
  if (current.value?.linkOpenMode === "copy") {
    const copied = await copyText(link.url);
    linkAction.value = { key: link.key, status: copied ? "copied" : "error", message: copied ? null : t("detailWindow.copy-failed") };
    return;
  }
  linkAction.value = { key: link.key, status: "opening", message: null };
  try {
    if (isTauriRuntime()) {
      await invoke("open_external_url", { url: link.url });
    } else {
      const opened = window.open(link.url, "_blank");
      if (!opened) {
        throw new Error(t("detailWindow.cannot-open-a-browser-window"));
      }
    }
    linkAction.value = { key: link.key, status: "opened", message: null };
  } catch (error) {
    linkAction.value = {
      key: link.key,
      status: "error",
      message: toCommandError(error).message,
    };
  }
}

async function closeWindow(): Promise<void> {
  if (props.mode === "inline") {
    emit("close");
    return;
  }
  if (isTauriRuntime()) {
    try {
      await getCurrentWindow().close();
      return;
    } catch {
      // 回退到浏览器 close（开发模式）。
    }
  }
  window.close();
}

const errorTitle = computed(() => {
  switch (errorKind.value) {
    case "missing":
      return t("detailWindow.no-details-to-show");
    case "version":
      return t("detailWindow.incompatible-details-data");
    default:
      return t("detailWindow.failed-to-read-details-data");
  }
});

const errorHelp = computed(() => {
  switch (errorKind.value) {
    case "missing":
      return t("detailWindow.select-an-item-in-the-main-window");
    case "version":
      return t("detailWindow.this-window-has-data-from-another-version");
    default:
      return t("detailWindow.reopen-from-the-main-window-closing");
  }
});
</script>

<template>
  <div
    :class="mode === 'window' ? 'flex h-full min-h-0 flex-col bg-surface text-ink' : 'space-y-3'"
    data-testid="detail-window"
  >
    <header
      v-if="mode === 'window'"
      class="flex h-12 shrink-0 items-center justify-between gap-2 border-b border-line px-5"
    >
      <h1 class="ui-section-title truncate">{{ t("app.item-details-2") }}</h1>
      <UiIconButton :label="t('detailWindow.close-window-esc')" data-testid="detail-window-close" @click="closeWindow">
        <X :size="16" aria-hidden="true" />
      </UiIconButton>
    </header>

    <div :class="mode === 'window' ? 'scroll-area min-h-0 flex-1 px-5 py-4' : ''">
      <div v-if="errorKind" class="flex flex-col items-start gap-2 py-6" data-testid="detail-window-error">
        <p class="text-sm font-medium">{{ errorTitle }}</p>
        <p class="text-[13px] text-muted">{{ errorHelp }}</p>
        <UiIconButton v-if="mode === 'window'" :label="t('detailWindow.close-window-esc')" @click="closeWindow">
          <X :size="16" aria-hidden="true" />
        </UiIconButton>
      </div>

      <template v-else-if="current && selectedPlanItem">
        <ItemDetailView
          :item="selectedPlanItem"
          :region-name="current.regionName"
          :unit-label="current.unitLabel === '—' ? null : current.unitLabel"
          :category-id="selectedItem?.category ?? ''"
          :category-label="selectedItem?.categoryLabel ?? ''"
          :subgroup-label="selectedItem?.subgroupLabel ?? null"
          :slot-name="current.slotName"
          :snapshot-time="current.snapshotTime"
          :stale="current.stale"
          :same-unit-items="sameUnitItems"
          :basemap="current.basemap"
          :links="links"
          :link-action="linkAction"
          :show-position="true"
          :show-popout="false"
          mode="window"
          @open-link="openLink"
        />

        <details open class="mt-4 rounded-section border border-line bg-raised/50 px-3 py-2" data-testid="detail-window-list">
          <summary class="cursor-pointer text-[13px] font-medium">
            {{ tFormat("detailWindow.items-in-this-sub-level",  { count: items.length }) }}
          </summary>
          <ul class="mt-2 max-h-64 divide-y divide-line/70 overflow-auto">
            <li v-for="item in items" :key="item.id">
              <button
                type="button"
                class="ui-interactive flex min-h-8 w-full cursor-pointer items-center gap-2 rounded-control px-1.5 py-1.5 text-left text-[13px]"
                :class="item.id === selectedItem?.id ? 'ui-row-selected font-medium text-accent-strong' : 'hover:bg-raised'"
                :data-testid="`detail-window-item-${item.id}`"
                @click="selectedId = item.id"
              >
                <component :is="categoryIcon(item.category)" :size="14" :class="categoryClass(item.category)" aria-hidden="true" />
                <span class="min-w-0 flex-1 truncate">{{ item.displayName }}</span>
                <UiBadge :tone="item.status === 'collected' ? 'success' : 'caution'" :kind-label="t('common.status')">
                  {{ item.status === "collected" ? t("common.collected") : t("common.uncollected") }}
                </UiBadge>
              </button>
            </li>
          </ul>
        </details>
      </template>
    </div>
  </div>
</template>
