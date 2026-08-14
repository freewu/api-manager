import { invoke } from "@tauri-apps/api/core";
import type {
  ApiFile,
  AppSettings,
  EnvStore,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
  VersionInfo,
} from "./types";

export function getWorkspace(): Promise<string | null> {
  return invoke<string | null>("get_workspace");
}

export function pickWorkspace(): Promise<string | null> {
  return invoke<string | null>("pick_workspace");
}

export function readTree(): Promise<TreeNode> {
  return invoke<TreeNode>("read_tree");
}

export function readApi(path: string): Promise<ApiFile> {
  return invoke<ApiFile>("read_api", { path });
}

export function saveApi(path: string, data: ApiFile): Promise<string> {
  return invoke<string>("save_api", { path, data });
}

export function saveApiVersion(data: ApiFile): Promise<string> {
  return invoke<string>("save_api_version", { data });
}

export function listVersions(uuid: string): Promise<VersionInfo[]> {
  return invoke<VersionInfo[]>("list_versions", { uuid });
}

export function readApiVersion(path: string): Promise<string> {
  return invoke<string>("read_api_version", { path });
}

export function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("load_settings");
}

export function saveSettings(settings: AppSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

export function createApi(dir: string, name: string): Promise<string> {
  return invoke<string>("create_api", { dir, name });
}

export function createFolder(parent: string, name: string): Promise<string> {
  return invoke<string>("create_folder", { parent, name });
}

export function renameEntry(path: string, newName: string): Promise<void> {
  return invoke<void>("rename_entry", { path, newName });
}

export function deleteEntry(path: string): Promise<void> {
  return invoke<void>("delete_entry", { path });
}

export function readInfo(path: string): Promise<InfoJson> {
  return invoke<InfoJson>("read_info", { path });
}

export function saveInfo(path: string, data: InfoJson): Promise<void> {
  return invoke<void>("save_info", { path, data });
}

export function readEnv(): Promise<EnvStore> {
  return invoke<EnvStore>("read_envs");
}

export function saveEnv(data: EnvStore): Promise<void> {
  return invoke<void>("save_envs", { data });
}

export function updateTrayEnv(name: string): Promise<void> {
  return invoke<void>("update_tray_env", { name });
}

export function sendRequest(req: HttpRequestData): Promise<HttpResult> {
  return invoke<HttpResult>("send_request", { req });
}

export function mockStart(port: number): Promise<MockStatus> {
  return invoke<MockStatus>("mock_start", { port });
}

export function mockStop(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_stop");
}

export function mockStatus(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_status");
}

export function mockReload(): Promise<MockStatus> {
  return invoke<MockStatus>("mock_reload");
}

export function getAppVersion(): Promise<string> {
  return invoke<string>("get_app_version");
}
