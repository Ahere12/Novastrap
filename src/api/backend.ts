// Typed wrappers over Tauri commands. The UI never calls Roblox or other sites directly.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ErrorInfo {
  kind:
    | "network" | "http_status" | "upstream_changed" | "io" | "unsupported"
    | "invalid_input" | "verification" | "launch" | "busy";
  message: string;
  retryable: boolean;
}

/** Commands reject with ErrorInfo; this turns anything thrown into readable text. */
export function errorMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as ErrorInfo).message);
  return String(e);
}

// ---- Roblox install / launch ----
export interface Installation { versionGuid: string; directory: string; executable: string }
export type HandlerOwner =
  | { kind: "none" }
  | { kind: "official" | "bloxstrap" | "fishstrap" | "novastrap" | "other"; executable: string };
export interface ClientVersion { version: string; clientVersionUpload: string }
export type UpdateState =
  | { state: "notInstalled" } | { state: "upToDate" } | { state: "updateAvailable" }
  | { state: "unknown"; error: ErrorInfo };
export interface RobloxStatus {
  installations: Installation[]; handler: HandlerOwner; latest: ClientVersion | null; update: UpdateState;
}
export type InstallProgress =
  | { stage: "downloading"; downloaded: number; total: number | null }
  | { stage: "verifying" } | { stage: "installing" } | { stage: "done" };
export type LaunchTarget = { kind: "app" } | { kind: "place"; placeId: number; jobId: string | null };
export interface Launched { pid: number; target: LaunchTarget; warnings: string[] }

// ---- accounts ----
export interface SavedAccount {
  userId: number; username: string; displayName: string; avatarUrl: string | null;
  linked: boolean; addedAt: number; lastUsedAt: number | null;
}
export type SessionStatus = "profileOnly" | "signedIn" | "expired";
export interface AccountView extends SavedAccount { status: SessionStatus; active: boolean }
export interface AccountsView { accounts: AccountView[]; oauthConfigured: boolean; redirectUri: string }

// ---- instances ----
export interface InstanceProfile { id: string; name: string; target: string; performanceProfile: string | null }
export interface RunningInstance { pid: number; profileId: string | null; name: string; startedAt: number }
export interface ClientProcess { pid: number; memoryBytes: number; runTimeSecs: number; instance: RunningInstance | null }

// ---- discovery / servers ----
export interface GameCard {
  universeId: number; rootPlaceId: number; name: string; playerCount: number | null;
  totalUpVotes: number | null; totalDownVotes: number | null; genre: string | null; iconUrl: string | null;
}
export interface Sort { id: string; name: string; description: string | null; games: GameCard[] }
export interface DiscoverHome { sorts: Sort[]; recommended: GameCard[]; recommendedHint: string | null }
export interface GameDetail {
  id: number; rootPlaceId: number; name: string; description: string;
  creator: { id: number; name: string; kind: string; hasVerifiedBadge: boolean };
  playing: number; visits: number; maxPlayers: number; favoritedCount: number; created: string; updated: string;
  genre: string | null; subgenre: string | null; thumbnails: string[]; iconUrl: string | null;
  upVotes: number | null; downVotes: number | null;
}
export interface Server { id: string; maxPlayers: number; playing: number; fps: number | null; ping: number | null }
export interface ServerPage { servers: Server[]; nextCursor: string | null }
export type ServerOrder = "desc" | "asc";

// ---- favorites / history ----
export interface FavGame { universeId: number; placeId: number; name: string; iconUrl: string | null; genre: string | null; addedAt: number }
export interface FavServer { placeId: number; jobId: string; label: string; addedAt: number }
export interface FavItem { id: number; itemType: string; name: string; thumbnailUrl: string | null; addedAt: number }
export interface Favorites { games: FavGame[]; servers: FavServer[]; items: FavItem[] }
export interface Session {
  placeId: number; universeId: number | null; jobId: string; userId: number | null;
  startedAt: number; endedAt: number | null; gameName: string | null;
}
export interface GameStat {
  placeId: number; universeId: number | null; gameName: string | null; sessions: number; seconds: number; lastPlayed: number;
}
export interface PlayStats { totalSeconds: number; last7DaysSeconds: number; sessions: number; recent: GameStat[]; top: GameStat[] }
export interface Live { session: Session | null; iconUrl: string | null; discordStatus: string | null }
export interface HomeData { stats: PlayStats; icons: Record<string, string>; live: Live; tracking: boolean }

// ---- performance ----
export type FlagKind = { type: "bool" } | { type: "int"; min: number; max: number } | { type: "choice"; options: number[] };
export interface FlagSpec { name: string; kind: FlagKind; group: string; description: string }
export interface PerformanceProfile {
  name: string; flags: Record<string, boolean | number>; framerateCap: number | null; graphicsQuality: number | null;
}
export interface RobloxSettings { framerateCap: number | null; graphicsQuality: number | null; path: string | null }
export interface PerformanceState {
  allowlist: FlagSpec[]; presets: PerformanceProfile[]; profiles: PerformanceProfile[]; active: string | null;
  roblox: RobloxSettings; framerateCaps: number[];
}
export interface Diagnostics {
  status: RobloxStatus; clients: ClientProcess[]; flagsFile: string | null; flagsWritten: number;
  robloxSettings: RobloxSettings; recentErrors: string[]; mods: { slotId: string; ok: boolean; reason: string | null }[];
  /** Signature check of RobloxPlayerBeta.exe; null when Roblox isn't installed. */
  executableSignature: { Ok: null } | { Err: string } | null;
}

// ---- mods / catalog / trading ----
export interface ModSlot {
  slot: { id: string; name: string; group: string; relativePath: string; format: "png" | "ogg"; description: string };
  enabled: boolean; compatible: boolean; reason: string | null;
}
export interface CatalogItem {
  id: number; itemType: string; assetType: number | null; name: string; description: string; creatorName: string;
  creatorHasVerifiedBadge: boolean; price: number | null; lowestPrice: number | null; lowestResalePrice: number | null;
  favoriteCount: number; itemRestrictions: string[]; totalQuantity: number | null;
  unitsAvailableForConsumption: number | null; isOffSale: boolean; itemCreatedUtc: string | null; thumbnailUrl: string | null;
}
export interface CatalogPage { data: CatalogItem[]; nextPageCursor: string | null }
export type CatalogCategory = "all" | "accessories" | "clothing" | "bodyParts" | "animations" | "collectibles";
export type CatalogSort = "relevance" | "mostFavorited" | "bestselling" | "recentlyUpdated" | "priceLowToHigh" | "priceHighToLow";
export interface ItemValue {
  id: number; name: string; acronym: string | null; rap: number | null; value: number | null;
  demand: "terrible" | "low" | "normal" | "high" | "amazing" | null;
  trend: "lowering" | "unstable" | "stable" | "raising" | "fluctuating" | null;
  projected: boolean; hyped: boolean; rare: boolean;
}
export interface ValueTable { sourceName: string; sourceUrl: string; fetchedAt: number; items: ItemValue[] }

// ---- settings / system ----
export interface Settings {
  general: { minimizeOnLaunch: boolean; trackActivity: boolean; closeToTray: boolean };
  appearance: {
    theme: string; accent: string; background: string | null; backgroundDim: number; font: string;
    reduceMotion: boolean; uiSounds: boolean; compact: boolean; textScale: number;
    sidebarOrder: string[]; sidebarHidden: string[];
  };
  accounts: { oauthClientId: string; activeUserId: number | null };
  instances: { defaultProfile: string | null };
  integrations: {
    discordEnabled: boolean; discordAppId: string; discordShowGame: boolean; discordShowJoin: boolean;
    discordShowAccount: boolean; discordCustomState: string;
  };
  notifications: { gameJoined: boolean; updateAvailable: boolean };
  shortcuts: { enabled: boolean; showWindow: string; toggleHud: string; play: string };
  updates: { channel: "stable" | "beta" };
}
export type NoticeKind = "update" | "game" | "launch" | "error" | "info";
export interface Notice { id: number; at: number; kind: NoticeKind; title: string; body: string; read: boolean }
export interface Release {
  tagName: string; name: string | null; body: string | null; htmlUrl: string; prerelease: boolean; draft: boolean; publishedAt: string | null;
}
export interface ReleasesView { releases: Release[]; history: { version: string; firstRun: number }[] }
export interface StorageItem { id: string; label: string; bytes: number; clearable: boolean }
export interface UpdateInfo { current: string; channel: string; latest: string | null; url: string | null; updateAvailable: boolean }
export interface AppInfo { name: string; version: string }

export const backend = {
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  // launcher
  robloxStatus: () => invoke<RobloxStatus>("roblox_status"),
  installRoblox: () => invoke<void>("install_roblox"),
  launchRoblox: (input: string, jobId?: string, safe?: boolean) => invoke<Launched>("launch_roblox", { input, jobId: jobId ?? null, safe: safe ?? false }),
  onInstallProgress: (cb: (p: InstallProgress) => void): Promise<UnlistenFn> =>
    listen<InstallProgress>("install-progress", (e) => cb(e.payload)),
  // instances
  instanceProfiles: () => invoke<InstanceProfile[]>("instance_profiles"),
  saveInstanceProfile: (profile: InstanceProfile) => invoke<InstanceProfile[]>("save_instance_profile", { profile }),
  deleteInstanceProfile: (id: string) => invoke<InstanceProfile[]>("delete_instance_profile", { id }),
  launchInstance: (id: string) => invoke<Launched>("launch_instance", { id }),
  runningClients: () => invoke<ClientProcess[]>("running_clients"),
  closeClient: (pid: number) => invoke<void>("close_client", { pid }),
  // performance
  performanceState: () => invoke<PerformanceState>("performance_state"),
  savePerformanceProfile: (profile: PerformanceProfile, originalName: string | null) =>
    invoke<{ profile: PerformanceProfile; dropped: string[] }>("save_performance_profile", { profile, originalName }),
  deletePerformanceProfile: (name: string) => invoke<void>("delete_performance_profile", { name }),
  setActivePerformanceProfile: (name: string | null) => invoke<string[]>("set_active_performance_profile", { name }),
  applyRobloxSettings: (framerateCap: number | null, graphicsQuality: number | null) =>
    invoke<RobloxSettings>("apply_roblox_settings", { framerateCap, graphicsQuality }),
  diagnostics: () => invoke<Diagnostics>("diagnostics"),
  // accounts
  accountsList: () => invoke<AccountsView>("accounts_list"),
  accountAddProfile: (username: string) => invoke<SavedAccount>("account_add_profile", { username }),
  accountSignIn: () => invoke<SavedAccount>("account_sign_in"),
  accountSwitch: (userId: number) => invoke<void>("account_switch", { userId }),
  accountRemove: (userId: number) => invoke<void>("account_remove", { userId }),
  avatarRender: (userId: number) => invoke<string | null>("avatar_render", { userId }),
  // discovery / servers / favorites
  discoverHome: () => invoke<DiscoverHome>("discover_home"),
  discoverSearch: (query: string) => invoke<GameCard[]>("discover_search", { query }),
  gameDetails: (universeId: number | null, placeId: number | null) => invoke<GameDetail>("game_details", { universeId, placeId }),
  serverList: (placeId: number, order: ServerOrder, cursor: string | null, fresh: boolean) =>
    invoke<ServerPage>("server_list", { placeId, order, cursor, fresh }),
  favorites: () => invoke<Favorites>("favorites"),
  toggleFavoriteGame: (game: FavGame) => invoke<boolean>("toggle_favorite_game", { game }),
  toggleFavoriteServer: (server: FavServer) => invoke<boolean>("toggle_favorite_server", { server }),
  toggleFavoriteItem: (item: FavItem) => invoke<boolean>("toggle_favorite_item", { item }),
  homeData: () => invoke<HomeData>("home_data"),
  clearHistory: () => invoke<void>("clear_history"),
  serverHistory: () => invoke<Session[]>("server_history"),
  createGameShortcut: (placeId: number, name: string) => invoke<string>("create_game_shortcut", { placeId, name }),
  // mods / appearance / catalog / trading
  modSlots: () => invoke<ModSlot[]>("mod_slots"),
  modChooseFile: (slotId: string) => invoke<boolean>("mod_choose_file", { slotId }),
  modRevert: (slotId: string) => invoke<void>("mod_revert", { slotId }),
  backgroundChoose: () => invoke<string | null>("background_choose"),
  backgroundClear: () => invoke<void>("background_clear"),
  backgroundData: () => invoke<string | null>("background_data"),
  catalogSearch: (keyword: string, category: CatalogCategory, sort: CatalogSort, cursor: string | null) =>
    invoke<CatalogPage>("catalog_search", { keyword, category, sort, cursor }),
  tradeValues: () => invoke<ValueTable>("trade_values"),
  // system
  appInfo: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  liveActivity: () => invoke<Live>("live_activity"),
  onActivity: (cb: (l: Live) => void): Promise<UnlistenFn> => listen<Live>("activity", (e) => cb(e.payload)),
  hudToggle: () => invoke<boolean>("hud_toggle"),
  storageUsage: () => invoke<StorageItem[]>("storage_usage"),
  storageClear: (id: string) => invoke<number>("storage_clear", { id }),
  appLogTail: () => invoke<string[]>("app_log_tail"),
  openFolder: (which: "logs" | "data" | "robloxLogs") => invoke<void>("open_folder", { which }),
  backupExport: () => invoke<string | null>("backup_export"),
  backupImport: () => invoke<boolean>("backup_import"),
  checkAppUpdate: () => invoke<UpdateInfo>("check_app_update"),
  appReleases: () => invoke<ReleasesView>("app_releases"),
  noticesList: () => invoke<{ items: Notice[] }>("notices_list"),
  noticesMarkRead: () => invoke<void>("notices_mark_read"),
  noticesClear: () => invoke<void>("notices_clear"),
  onNotice: (cb: (n: Notice) => void): Promise<UnlistenFn> => listen<Notice>("notice", (e) => cb(e.payload)),
  shortcutProblems: () => invoke<string[]>("shortcut_problems"),
};
