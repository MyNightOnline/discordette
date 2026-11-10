import { invoke } from "@tauri-apps/api/core";

export interface Tag { id: string; name: string; emoji?: string | null }
export interface Room { id: string; name: string; kind: "forum" | "text"; channel_id?: string | null; guild_id?: string | null; tags: Tag[]; require_tag: boolean }
export interface AiModel { id: string; ctx?: string | null; vision: boolean; enabled: boolean }
export interface Provider { id: string; name: string; preset?: string | null; base_url: string; format: "anthropic" | "chat"; api_key: string; enabled: boolean; models: AiModel[] }
export interface Config { rooms: Room[]; max_file_mb: number; max_video_mb: number; theme: "system" | "light" | "dark"; prompt_template: string; providers: Provider[] }
export interface WebhookInfo { name: string; channel_id: string; guild_id?: string | null; avatar_url?: string | null }
export interface Staged { id: number; size: number; w: number; h: number; shrunk: boolean; original_size: number; name: string; video: boolean; mime: string }
export interface FolderFile { path: string; name: string; video: boolean }
export interface Entry { at: string; room_id: string; room_name: string; title: string; content: string; source_url: string; tags: string[]; link?: string | null }

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveSettings: (maxFileMb: number, maxVideoMb: number, theme: string, promptTemplate: string) => invoke<Config>("save_settings", { maxFileMb, maxVideoMb, theme, promptTemplate }),
  saveProviders: (providers: Provider[]) => invoke<Config>("save_providers", { providers }),
  aiSummarize: (providerId: string, model: string, text: string, extra: string, tags: string[]) => invoke<string>("ai_summarize", { providerId, model, text, extra, tags }),
  createTag: (roomId: string, name: string) => invoke<Config>("create_tag", { roomId, name }),
  aiTest: (provider: Provider, model: string) => invoke<void>("ai_test", { provider, model }),
  saveRoom: (room: Room, webhookUrl?: string) => invoke<Config>("save_room", { room, webhookUrl }),
  deleteRoom: (id: string) => invoke<Config>("delete_room", { id }),
  reorderRooms: (ids: string[]) => invoke<Config>("reorder_rooms", { ids }),
  getWebhookUrl: (roomId: string) => invoke<string | null>("get_webhook_url", { roomId }),
  webhookInfo: (url: string) => invoke<WebhookInfo>("webhook_info", { url }),
  testRoom: (roomId: string) => invoke<WebhookInfo>("test_room", { roomId }),
  setBotToken: (token: string) => invoke<void>("set_bot_token", { token }),
  hasBotToken: () => invoke<boolean>("has_bot_token"),
  getBotToken: () => invoke<string | null>("get_bot_token"),
  fetchTags: (roomId: string) => invoke<Config>("fetch_tags", { roomId }),
  stageImage: (file: File) =>
    file.arrayBuffer().then((buf) =>
      invoke<Staged>("stage_image", new Uint8Array(buf), { headers: { name: encodeURIComponent(file.name || "image.png"), mime: file.type } })),
  listFolder: (path: string, image: boolean, video: boolean) => invoke<{ files: FolderFile[]; skipped: number }>("list_folder", { path, image, video }),
  stagePath: (path: string) => invoke<Staged>("stage_path", { path }),
  getStaged: (id: number) => invoke<ArrayBuffer>("get_staged", { id }),
  removeImage: (id: number) => invoke<void>("remove_image", { id }),
  clearImages: () => invoke<void>("clear_images"),
  sendPost: (a: { roomId: string; title: string; content: string; tagIds: string[]; imageIds: number[]; sourceUrl: string }) =>
    invoke<{ link?: string | null; sent: number; cancelled: boolean }>("send_post", a),
  cancelSend: () => invoke<void>("cancel_send"),
  findDuplicate: (roomId: string, title: string, sourceUrl: string) => invoke<Entry | null>("find_duplicate", { roomId, title, sourceUrl }),
  exportConfig: (path: string) => invoke<void>("export_config", { path }),
  importConflicts: (path: string) => invoke<string[]>("import_conflicts", { path }),
  importConfig: (path: string, replace: boolean) => invoke<{ added: number; replaced: number; skipped: number }>("import_config", { path, replace }),
};

export const errMsg = (e: unknown) => (typeof e === "string" ? e : (e as Error)?.message ?? String(e));
