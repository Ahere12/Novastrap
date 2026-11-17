// Typed wrappers over Tauri commands. The UI never calls Roblox directly.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ErrorInfo {
  kind:
    | "network"
    | "http_status"
    | "upstream_changed"
    | "io"
    | "unsupported"
    | "invalid_input"
    | "verification"
    | "launch"
    | "busy";
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

export type InstallProgress =
  | { stage: "downloading"; downloaded: number; total: number | null }
  | { stage: "verifying" }
  | { stage: "installing" }
  | { stage: "done" };

export type LaunchTarget =
  | { kind: "app" }
  | { kind: "place"; placeId: number; jobId: string | null };

export interface Launched {
  pid: number;
  target: LaunchTarget;
}

/** Commands reject with ErrorInfo; this turns anything thrown into readable text. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as ErrorInfo).message);
  return String(e);
}

export const backend = {
  robloxStatus: () => invoke<RobloxStatus>("roblox_status"),
  installRoblox: () => invoke<void>("install_roblox"),
  launchRoblox: (input: string) => invoke<Launched>("launch_roblox", { input }),
  onInstallProgress: (cb: (p: InstallProgress) => void): Promise<UnlistenFn> =>
    listen<InstallProgress>("install-progress", (e) => cb(e.payload)),
  appInfo: () => invoke<AppInfo>("app_info"),
};
