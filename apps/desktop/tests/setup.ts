// Vitest 全局设置（SX-015）：每个用例后卸载组件，避免 Teleport/焦点状态跨用例泄漏。
import { afterEach } from "vitest";
import { enableAutoUnmount } from "@vue/test-utils";

enableAutoUnmount(afterEach);
