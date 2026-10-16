import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  os: string;
  arch: string;
}

// All backend calls live here; components never call invoke directly.
export const getAppInfo = () => invoke<AppInfo>("get_app_info");
