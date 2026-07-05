import { invoke } from "@tauri-apps/api/core";

/** Single typed gateway to the Rust core. */
export async function api<T = any>(method: string, payload: Record<string, unknown> = {}): Promise<T> {
  return (await invoke("api", { method, payload })) as T;
}

export async function dataLocation(): Promise<string> {
  return (await invoke("data_location")) as string;
}

export async function restoreBackup(backupPath: string, allowUntrusted = false): Promise<any> {
  return await invoke("restore_backup", { backupPath, allowUntrusted });
}

/** The confirmation phrase is re-verified at the Rust command boundary. */
export async function purgeAllData(confirm: string): Promise<any> {
  return await invoke("purge_all_data", { confirm });
}
