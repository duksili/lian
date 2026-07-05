import { invoke } from "@tauri-apps/api/core";

/** Single typed gateway to the Rust core. */
export async function api<T = any>(method: string, payload: Record<string, unknown> = {}): Promise<T> {
  return (await invoke("api", { method, payload })) as T;
}

export async function dataLocation(): Promise<string> {
  return (await invoke("data_location")) as string;
}

export async function restoreBackup(backupPath: string): Promise<any> {
  return await invoke("restore_backup", { backupPath });
}

export async function purgeAllData(): Promise<any> {
  return await invoke("purge_all_data");
}
