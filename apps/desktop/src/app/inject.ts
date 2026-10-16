// Store 注入（SX-015）：组件通过 useStore() 获取同一实例；测试用 provide 注入替身。
import { inject, provide, type InjectionKey } from "vue";
import type { AppStore } from "./store";

export const storeKey: InjectionKey<AppStore> = Symbol("sottr-explorer-store");

export function provideStore(store: AppStore): void {
  provide(storeKey, store);
}

export function useStore(): AppStore {
  const store = inject(storeKey);
  if (!store) {
    throw new Error("app store 未提供：组件必须挂载在 provideStore 之下");
  }
  return store;
}
