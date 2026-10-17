<script setup lang="ts">
// 全收集（100%）页面（SX-021 阶段 C2b）：
// - 数据来自内嵌目录（data/catalog）与当前存档的观察集合；
// - 规则与 sottr-core::missing / CLI missing 一致（C2a 黄金测试保证）：
//   必收类别按必收子集给逐项缺失清单；可选类别只给计数；未闭合类别不显示缺失结论；
//   共用同一 ID 的条目按 ID 计数并列出全部名称；目录外观察 ID 单列。
import { computed } from "vue";
import { useStore } from "../app/inject";
import {
  computeMissing,
  productionCatalog,
  type CategoryMissing,
  type MissingItem,
} from "../data/catalog";

const store = useStore();

const missing = computed(() => {
  const report = store.report.value;
  if (!report) {
    return null;
  }
  const ids = report.observations.inventory.records.map((record) => record.internal_id);
  return computeMissing(productionCatalog, ids);
});

const required = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === true) ?? [],
);
const optional = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === false) ?? [],
);
const unresolved = computed<CategoryMissing[]>(
  () => missing.value?.categories.filter((category) => category.required_for_100 === null) ?? [],
);

const requiredTotals = computed(() => {
  const totals = { total: 0, observed: 0, missing: 0 };
  for (const category of required.value) {
    totals.total += category.total;
    totals.observed += category.observed;
    totals.missing += category.missing ?? 0;
  }
  return totals;
});

function missingLabel(item: MissingItem): string {
  return item.name ?? item.id;
}
</script>

<template>
  <section class="mx-auto max-w-[980px] px-4 py-5" data-testid="full-collection">
    <header class="mb-4">
      <h1 class="text-[20px] font-semibold text-ink">全收集（100%）</h1>
      <p class="mt-1 text-[12px] leading-5 text-muted">
        按成就/100% 完成度口径统计。必收类别给出逐项缺失清单（缺失 = 未在当前存档的观察集合中出现）；
        可选类别只给计数；未闭合类别不显示缺失结论。必收计数与清单按必收子集，DLC 与非必收条目不混入。
      </p>
    </header>

    <p v-if="!missing" class="rounded-[6px] border border-line bg-surface px-3 py-4 text-[13px] text-muted">
      先分析一个存档文件，再查看全收集缺失清单。
    </p>

    <template v-else>
      <div class="mb-4 flex flex-wrap items-center gap-3 rounded-[6px] border border-line bg-surface px-3 py-2 text-[13px]">
        <span class="text-ink">必收合计：{{ requiredTotals.observed }}/{{ requiredTotals.total }}</span>
        <span class="rounded-[6px] bg-accent-bg px-2 py-0.5 text-accent-strong">缺失 {{ requiredTotals.missing }}</span>
        <span v-if="missing.observed_not_in_catalog.length > 0" class="text-muted">
          目录外未知 ID：{{ missing.observed_not_in_catalog.length }}（单列，不混入缺失）
        </span>
      </div>

      <h2 class="mb-2 text-[16px] font-medium text-ink">必收类别</h2>
      <ul class="mb-5 space-y-1.5">
        <li
          v-for="category in required"
          :key="category.id"
          class="rounded-[6px] border border-line bg-surface px-3 py-2"
          :data-testid="`category-${category.id}`"
        >
          <div class="flex flex-wrap items-center gap-2 text-[13px]">
            <span class="text-ink">{{ category.name }}</span>
            <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
            <span v-if="(category.missing ?? 0) > 0" class="rounded-[6px] bg-accent-bg px-2 py-0.5 text-accent-strong">
              缺失 {{ category.missing }}
            </span>
            <span v-else class="text-muted">已全部观察到</span>
          </div>
          <details
            v-if="(category.items?.length ?? 0) > 0"
            class="mt-1.5"
            :data-testid="`missing-toggle-${category.id}`"
          >
            <summary class="cursor-pointer text-[12px] text-accent-strong">展开缺失清单（{{ category.items?.length }}）</summary>
            <ul class="mt-1 max-h-72 space-y-0.5 overflow-auto pl-3 text-[12px] leading-5">
              <li
                v-for="item in category.items"
                :key="item.id"
                class="text-muted"
                data-testid="missing-item"
              >
                <span class="text-ink">{{ missingLabel(item) }}</span>
                <span class="ml-2 font-mono text-[11px]">{{ item.id }}</span>
                <span v-if="item.collection" class="ml-2">合集：{{ item.collection }}</span>
                <span v-if="item.shared_names" class="ml-2">
                  共用 ID：{{ item.shared_names.join(" / ") }}（按 ID 计数，不拆分判定）
                </span>
              </li>
            </ul>
          </details>
        </li>
      </ul>

      <h2 class="mb-2 text-[16px] font-medium text-ink">可选类别</h2>
      <ul class="mb-5 space-y-1.5">
        <li
          v-for="category in optional"
          :key="category.id"
          class="flex flex-wrap items-center gap-2 rounded-[6px] border border-line bg-surface px-3 py-2 text-[13px]"
          :data-testid="`category-${category.id}`"
        >
          <span class="text-ink">{{ category.name }}</span>
          <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
          <span class="text-muted">可选，不进入缺失清单</span>
        </li>
      </ul>

      <h2 class="mb-2 text-[16px] font-medium text-ink">未闭合类别（不显示缺失结论）</h2>
      <ul class="space-y-1.5">
        <li
          v-for="category in unresolved"
          :key="category.id"
          class="flex flex-wrap items-center gap-2 rounded-[6px] border border-line bg-surface px-3 py-2 text-[13px]"
          :data-testid="`category-${category.id}`"
        >
          <span class="text-ink">{{ category.name }}</span>
          <span class="text-muted">{{ category.observed }}/{{ category.total }}</span>
          <span class="text-muted">数据未闭合：不显示缺失数或清单</span>
        </li>
      </ul>
    </template>
  </section>
</template>
