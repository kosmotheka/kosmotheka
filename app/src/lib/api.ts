import { invoke } from "@tauri-apps/api/core";

// Mirrors `db::AppInfo` in src-tauri/src/db.rs.
export interface AppInfo {
  launchCount: number;
  firstLaunch: string | null;
  sqliteVersion: string;
  databasePath: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}
