// Typed wrappers over Tauri commands. The UI never calls Roblox directly.
import { invoke } from "@tauri-apps/api/core";

export interface ErrorInfo {
  kind: "network" | "http_status" | "upstream_changed" | "io" | "unsupported";
  message: string;
  retryable: boolean;
}

export interface Installation {
  versionGuid: string;
  directory: string;
  executable: string;
}

export type HandlerOwner =
  | { kind: "none" }
  | { kind: "official" | "bloxstrap" | "fishstrap" | "novastrap" | "other"; executable: string };

export interface ClientVersion {
  version: string;
  clientVersionUpload: string;
}

export type UpdateState =
  | { state: "notInstalled" }
  | { state: "upToDate" }
  | { state: "updateAvailable" }
  | { state: "unknown"; error: ErrorInfo };

export interface RobloxStatus {
  installations: Installation[];
  handler: HandlerOwner;
  latest: ClientVersion | null;
  update: UpdateState;
}

export interface AppInfo {
  name: string;
  version: string;
}

export const backend = {
  robloxStatus: () => invoke<RobloxStatus>("roblox_status"),
  appInfo: () => invoke<AppInfo>("app_info"),
};
