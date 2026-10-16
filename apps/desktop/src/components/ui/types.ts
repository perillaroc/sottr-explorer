// 外观组件共享类型（SX-015）。

export interface MenuItem {
  id: string;
  label: string;
  disabled?: boolean;
  reason?: string;
}

export interface ChipOption {
  value: string;
  label: string;
  count?: number;
  disabled?: boolean;
  title?: string;
}
