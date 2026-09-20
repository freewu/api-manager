import { AppSettings } from "../../types";

/** 设置 Tab 通用 props：settings 为当前设置，patch 合并保存（立即生效 + 持久化） */
export interface SettingsTabProps {
  settings: AppSettings;
  patch: (p: Partial<AppSettings>) => void;
}
