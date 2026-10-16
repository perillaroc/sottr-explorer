<script setup lang="ts">
// 收集品清单（SX-015）：组行与逐项行共享身份；组行只呈现“组级动作”的入口，
// 不复制为多个精确条目；名称回退文案带标识；行是按钮，可用键盘选择。
import { computed, ref } from "vue";
import { ChevronDown, ChevronRight } from "@lucide/vue";
import { useStore } from "../app/inject";
import { businessStatus, categoryLabel, itemEvidenceLevel, evidenceLabel, precisionLabel, regionLabel } from "../app/labels";
import { shortId } from "../app/format";
import type { CollectibleRow, GroupRow, ItemRow } from "../app/catalog";
import { displayNameOf } from "../app/catalog";
import UiBadge from "./ui/UiBadge.vue";

const props = defineProps<{
  rows: CollectibleRow[];
  selectedId: string | null;
}>();

const emit = defineEmits<{ select: [id: string] }>();

const store = useStore();
const expandedGroups = ref<Set<string>>(new Set());

const showRegion = computed(() => store.state.region === "all");

function groupLabel(row: GroupRow): { text: string; note: string | null } {
  const group = row.location.guide_group_id ? store.metadata.guide_groups[row.location.guide_group_id] : null;
  if (group?.display_name) {
    return { text: group.display_name, note: group.name_kind === "curated_short" ? "人工简名" : null };
  }
  return { text: row.location.guide_group_id ?? row.id, note: null };
}

function itemName(row: ItemRow) {
  return displayNameOf(store.metadata, row.id, shortId);
}

function regionText(regionId: string | null): string {
  if (!regionId) {
    return "区域未知";
  }
  return regionLabel(regionId, store.metadata.regions[regionId]?.display_name ?? null);
}

function memberRows(row: GroupRow): ItemRow[] {
  const members: ItemRow[] = [];
  for (const id of row.location.member_ids) {
    const item = store.report.value?.assessments.find((entry) => entry.internal_id === id) ?? null;
    if (item) {
      members.push({ kind: "item", id, item, location: row.location, regionId: row.regionId });
    }
  }
  return members;
}

function toggleGroup(id: string): void {
  const next = new Set(expandedGroups.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  expandedGroups.value = next;
}
</script>

<template>
  <ul class="divide-y divide-line/70" aria-label="收集品清单">
    <li v-for="row in props.rows" :key="row.id" class="py-1">
      <template v-if="row.kind === 'group'">
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 rounded-[6px] px-2 py-1.5 text-left hover:bg-raised"
            :class="props.selectedId === row.id ? 'bg-accent-bg' : ''"
            :data-testid="`row-${row.id}`"
            @click="emit('select', row.id)"
          >
            <span class="min-w-0">
              <span class="block break-words">
                {{ groupLabel(row).text }} · 本组 {{ row.missingCount }} 项参考遗漏推定
              </span>
              <span class="block text-xs text-muted">
                {{ groupLabel(row).note ? `${groupLabel(row).note}；` : "" }}组内共 {{ row.memberCount }} 个成员 ID；攻略组线索，不复制为精确点位
              </span>
            </span>
            <span class="ml-auto flex shrink-0 items-center gap-1">
              <UiBadge tone="caution" kind-label="状态">推定遗漏</UiBadge>
              <UiBadge tone="info" kind-label="定位">攻略组</UiBadge>
            </span>
          </button>
          <button
            type="button"
            class="inline-flex h-8 cursor-pointer items-center gap-1 rounded-[6px] px-2 text-xs text-muted hover:bg-raised hover:text-ink"
            :aria-expanded="expandedGroups.has(row.id)"
            @click="toggleGroup(row.id)"
          >
            <component :is="expandedGroups.has(row.id) ? ChevronDown : ChevronRight" :size="14" aria-hidden="true" />
            展开组内条目
          </button>
        </div>
        <ul v-if="expandedGroups.has(row.id)" class="mt-1 ml-4 space-y-0.5 border-l border-line pl-3">
          <li v-for="member in memberRows(row)" :key="member.id">
            <button
              type="button"
              class="flex w-full flex-wrap items-center gap-2 rounded-[6px] px-2 py-1 text-left text-[13px] hover:bg-raised"
              @click="emit('select', member.id)"
            >
              <span class="break-words">{{ itemName(member).text }}</span>
              <span class="mono text-xs text-muted">{{ member.id }}</span>
              <span class="ml-auto flex items-center gap-1">
                <UiBadge :tone="businessStatus(member.item).tone === 'neutral' ? 'neutral' : 'caution'" kind-label="状态">
                  {{ businessStatus(member.item).label }}
                </UiBadge>
                <UiBadge tone="info" kind-label="定位">攻略组</UiBadge>
              </span>
            </button>
          </li>
        </ul>
      </template>

      <button
        v-else
        type="button"
        class="flex w-full flex-wrap items-start gap-2 rounded-[6px] px-2 py-1.5 text-left hover:bg-raised"
        :class="props.selectedId === row.id ? 'bg-accent-bg' : ''"
        :data-testid="`row-${row.id}`"
        @click="emit('select', row.id)"
      >
        <span class="min-w-0 flex-1">
          <span class="line-clamp-2 block break-words">{{ itemName(row).text }}</span>
          <span class="mt-0.5 block text-xs text-muted">
            {{ categoryLabel(store.metadata.items[row.id]?.category_id ?? null, null) }} · {{ row.id }}
            <template v-if="showRegion"> · {{ regionText(row.regionId) }}</template>
            <template v-if="itemName(row).note"> · {{ itemName(row).note }}</template>
          </span>
        </span>
        <span class="ml-auto flex shrink-0 flex-wrap items-center gap-1">
          <UiBadge :tone="businessStatus(row.item).tone" kind-label="状态">{{ businessStatus(row.item).label }}</UiBadge>
          <UiBadge tone="neutral" kind-label="依据">{{ evidenceLabel(itemEvidenceLevel(row.item)) }}</UiBadge>
          <UiBadge tone="info" kind-label="定位">{{ precisionLabel(row.location?.precision ?? null) }}</UiBadge>
        </span>
      </button>
    </li>
  </ul>
</template>
