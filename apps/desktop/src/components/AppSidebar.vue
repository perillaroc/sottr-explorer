<script setup lang="ts">
// 侧栏（SX-015）：“概览 / 区域与收集 / 区域段 / 诊断 / 设置与关于”在同一侧栏，
// 不做四列；折叠时只显示图标（aria-label 保留），区域选择移到正文或浮层。
import { computed } from "vue";
import { Activity, FolderOpen, LayoutDashboard, ListTree, Settings } from "@lucide/vue";
import { useStore } from "../app/inject";
import { regionsOf } from "../data/metadata";
import { regionLabel } from "../app/labels";

const props = withDefaults(defineProps<{ expanded: boolean }>(), { expanded: true });

const store = useStore();

const hasResult = computed(() => store.currentSlot.value?.result !== null);
const regions = computed(() =>
  regionsOf(store.metadata).map((region) => ({
    id: region.region_id,
    label: regionLabel(region.region_id, region.display_name),
  })),
);

const baseItem =
  "flex w-full items-center gap-2 rounded-[6px] px-2 py-[7px] text-left text-[13px] disabled:cursor-not-allowed disabled:opacity-60";

function itemClass(active: boolean): string {
  return `${baseItem} ${active ? "bg-accent-bg text-accent-strong" : "text-ink hover:bg-raised"}`;
}

function go(view: "import" | "overview" | "regions" | "diagnostics" | "settings"): void {
  store.actions.setView(view);
}

function goRegion(region: string): void {
  store.actions.setRegion(region);
  store.actions.setView("regions");
}
</script>

<template>
  <nav
    class="flex h-full flex-col border-r border-line bg-surface"
    :class="props.expanded ? 'w-[216px]' : 'w-14'"
    aria-label="主导航"
    data-testid="app-sidebar"
  >
    <ul class="flex-1 space-y-0.5 p-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'import')"
          :aria-current="store.state.view === 'import' ? 'page' : undefined"
          title="打开存档"
          @click="go('import')"
        >
          <FolderOpen :size="16" aria-hidden="true" />
          <span v-if="props.expanded">打开存档</span>
          <span v-else class="sr-only">打开存档</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'overview')"
          :disabled="!hasResult"
          :title="hasResult ? '概览' : '概览：先分析一个存档文件'"
          :aria-current="store.state.view === 'overview' ? 'page' : undefined"
          @click="go('overview')"
        >
          <LayoutDashboard :size="16" aria-hidden="true" />
          <span v-if="props.expanded">概览</span>
          <span v-else class="sr-only">概览</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'regions')"
          :disabled="!hasResult"
          :title="hasResult ? '区域与收集' : '区域与收集：先分析一个存档文件'"
          :aria-current="store.state.view === 'regions' ? 'page' : undefined"
          @click="go('regions')"
        >
          <ListTree :size="16" aria-hidden="true" />
          <span v-if="props.expanded">区域与收集</span>
          <span v-else class="sr-only">区域与收集</span>
        </button>
      </li>

      <li v-if="props.expanded" class="mt-2 px-2 text-xs text-muted">区域</li>
      <li v-if="props.expanded">
        <ul class="space-y-0.5">
          <li>
            <button
              type="button"
              :class="itemClass(store.state.view === 'regions' && store.state.region === 'all')"
              :disabled="!hasResult"
              :title="hasResult ? '全部区域' : '区域列表：先分析一个存档文件'"
              @click="goRegion('all')"
            >
              全部区域
            </button>
          </li>
          <li v-for="region in regions" :key="region.id">
            <button
              type="button"
              :class="itemClass(store.state.view === 'regions' && store.state.region === region.id)"
              :disabled="!hasResult"
              :title="hasResult ? region.label : '区域列表：先分析一个存档文件'"
              @click="goRegion(region.id)"
            >
              {{ region.label }}
            </button>
          </li>
          <li>
            <button
              type="button"
              :class="itemClass(store.state.view === 'regions' && store.state.region === 'unknown')"
              :disabled="!hasResult"
              :title="hasResult ? '区域未知' : '区域列表：先分析一个存档文件'"
              @click="goRegion('unknown')"
            >
              区域未知
            </button>
          </li>
        </ul>
      </li>
    </ul>

    <ul class="space-y-0.5 border-t border-line p-1.5">
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'diagnostics')"
          :disabled="!hasResult"
          :title="hasResult ? '诊断' : '诊断：先分析一个存档文件'"
          :aria-current="store.state.view === 'diagnostics' ? 'page' : undefined"
          @click="go('diagnostics')"
        >
          <Activity :size="16" aria-hidden="true" />
          <span v-if="props.expanded">诊断</span>
          <span v-else class="sr-only">诊断</span>
        </button>
      </li>
      <li>
        <button
          type="button"
          :class="itemClass(store.state.view === 'settings')"
          :aria-current="store.state.view === 'settings' ? 'page' : undefined"
          title="设置与关于"
          @click="go('settings')"
        >
          <Settings :size="16" aria-hidden="true" />
          <span v-if="props.expanded">设置与关于</span>
          <span v-else class="sr-only">设置与关于</span>
        </button>
      </li>
    </ul>
  </nav>
</template>
