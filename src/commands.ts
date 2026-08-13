import { invoke } from "@tauri-apps/api/core";
import type {
  ApiFile,
  HttpRequestData,
  HttpResult,
  InfoJson,
  MockStatus,
  TreeNode,
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
