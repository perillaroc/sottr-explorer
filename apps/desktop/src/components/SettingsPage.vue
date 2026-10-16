<script setup lang="ts">
// 设置与关于（SX-015）：只保留必要展示项（主题、布局恢复、版本与范围、许可证）。
import { computed } from "vue";
import { useStore } from "../app/inject";
import { copy } from "../app/labels";
import { supportedCategories } from "../app/catalog";
import type { ThemePreference } from "../app/theme";
import UiButton from "./ui/UiButton.vue";

const store = useStore();

const themeOptions: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
];

const report = computed(() => store.report.value);
const categories = computed(() => (report.value ? supportedCategories(store.metadata, report.value) : []));

const shortcuts = [
  { keys: "Ctrl+O", action: "打开文件（选择存档）" },
  { keys: "Ctrl+F", action: "聚焦当前清单搜索" },
  { keys: "F5", action: "刷新当前槽位（手动）" },
  { keys: "Esc", action: "关闭菜单或详情抽屉" },
  { keys: "Tab / Shift+Tab", action: "顶栏 → 导航 → 工具栏 → 清单 → 详情" },
];
</script>

<template>
  <section class="scroll-area min-h-0 flex-1 px-6 py-5" aria-labelledby="settings-title">
    <div class="mx-auto max-w-3xl space-y-4">
      <h1 id="settings-title" class="text-xl font-semibold">设置与关于</h1>

      <section class="space-y-2">
        <h2 class="text-base font-medium">主题</h2>
        <div class="flex flex-wrap items-center gap-2" role="radiogroup" aria-label="主题">
          <label
            v-for="option in themeOptions"
            :key="option.value"
            class="inline-flex cursor-pointer items-center gap-2 rounded-[6px] border px-3 py-1.5 text-[13px]"
            :class="store.state.theme === option.value ? 'border-accent bg-accent-bg text-accent' : 'border-line bg-surface text-ink'"
          >
            <input
              type="radio"
              name="theme"
              :value="option.value"
              :checked="store.state.theme === option.value"
              :data-testid="`theme-${option.value}`"
              @change="store.actions.setTheme(option.value)"
            />
            {{ option.label }}
          </label>
        </div>
        <p class="text-xs text-muted">令牌与对比度目标见开发方案；Windows 高对比度/DPI 实测由 SX-017 记录。</p>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">布局</h2>
        <div class="flex flex-wrap items-center gap-2">
          <UiButton size="sm" @click="store.actions.resetLayout()">恢复默认布局</UiButton>
          <span class="text-xs text-muted">导航展开、详情常驻；首版不提供窗格拖拽调宽。</span>
        </div>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">数据与范围</h2>
        <ul class="space-y-1 text-[13px]">
          <li>应用版本：0.1.0（模拟数据界面，SX-015）</li>
          <li>元数据版本：{{ store.metadata.metadata_version }}（{{ store.metadata.namespace }}）</li>
          <li>
            已支持范围：普通/稀有贮藏点、石碑、地下墓穴宝藏的参考遗漏推定；已验证坐标 0 项；区域百分比暂不支持。
          </li>
          <li v-if="categories.length">当前报告支持类别：{{ categories.map((category) => category.label).join("、") }}</li>
          <li>许可证：产品许可证尚未选择；第三方来源与许可见仓库 THIRD_PARTY_LICENSES.md。</li>
          <li>本地只读：无账号、无遥测、无后台上传；导出默认分享脱敏（接线在 SX-016）。</li>
        </ul>
        <p class="rounded-[6px] border border-caution/40 bg-caution-bg px-3 py-2 text-xs text-caution">
          {{ copy.mockBanner }}
        </p>
      </section>

      <section class="space-y-2">
        <h2 class="text-base font-medium">快捷键</h2>
        <ul class="space-y-1 text-[13px]">
          <li v-for="shortcut in shortcuts" :key="shortcut.keys">
            <span class="mono">{{ shortcut.keys }}</span>：{{ shortcut.action }}
          </li>
        </ul>
      </section>
    </div>
  </section>
</template>
