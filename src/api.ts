import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AccountConfig,
  ObjectMeta,
  ListResult,
  SearchResult,
  DeleteResult,
} from "./types";

export const api = {
  accounts(): Promise<Account[]> {
    return invoke("list_accounts");
  },

  getConfig(): Promise<AccountConfig[]> {
    return invoke("get_config");
  },

  putConfig(accounts: AccountConfig[]): Promise<{ ok: boolean }> {
    return invoke("put_config", { accounts });
  },

  buckets(accountId: number): Promise<{ buckets: string[] }> {
    return invoke("list_buckets", { accountIdx: accountId });
  },

  listObjects(
    accountId: number,
    bucket: string,
    opts: {
      prefix?: string;
      delimiter?: string;
      continuation_token?: string;
      limit?: number;
    } = {}
  ): Promise<ListResult> {
    return invoke("list_objects", {
      accountIdx: accountId,
      bucket,
      prefix: opts.prefix ?? "",
      delimiter: opts.delimiter ?? "/",
      continuationToken: opts.continuation_token ?? null,
      limit: opts.limit ?? 200,
    });
  },

  search(
    accountId: number,
    bucket: string,
    q: string,
    prefix = "",
    limit = 200,
    continuationToken?: string
  ): Promise<SearchResult> {
    return invoke("search_objects", {
      accountIdx: accountId,
      bucket,
      q,
      prefix,
      limit,
      continuationToken: continuationToken ?? null,
    });
  },

  deleteObjects(
    accountId: number,
    bucket: string,
    keys: string[]
  ): Promise<DeleteResult> {
    return invoke("delete_objects", {
      accountIdx: accountId,
      bucket,
      keys,
    });
  },

  /** Download S3 object directly to a local file path. */
  download(
    accountId: number,
    bucket: string,
    key: string,
    savePath: string
  ): Promise<{ success: boolean }> {
    return invoke("download_object", {
      accountIdx: accountId,
      bucket,
      key,
      savePath,
    });
  },

  presign(
    accountId: number,
    bucket: string,
    key: string,
    expires = 3600
  ): Promise<{ url: string }> {
    return invoke("presign_object", {
      accountIdx: accountId,
      bucket,
      key,
      expires,
    });
  },

  meta(accountId: number, bucket: string, key: string): Promise<ObjectMeta> {
    return invoke("object_meta", {
      accountIdx: accountId,
      bucket,
      key,
    });
  },

  preview(
    accountId: number,
    bucket: string,
    key: string
  ): Promise<{ text: string }> {
    return invoke("preview_object", {
      accountIdx: accountId,
      bucket,
      key,
    });
  },

  updateText(
    accountId: number,
    bucket: string,
    key: string,
    text: string,
    contentType = "text/plain; charset=utf-8"
  ): Promise<{ ok: boolean }> {
    return invoke("update_text", {
      accountIdx: accountId,
      bucket,
      key,
      text,
      contentType,
    });
  },

  /** Upload from a local file path (for file dialog picks). */
  uploadObject(
    accountId: number,
    bucket: string,
    key: string,
    filePath: string,
    contentType?: string,
    taskId?: string
  ): Promise<{ success: boolean; key: string; size: number }> {
    return invoke("upload_object", {
      accountIdx: accountId,
      bucket,
      key,
      filePath,
      contentType: contentType ?? null,
      taskId: taskId ?? null,
    });
  },

  /** Upload from raw bytes (for drag-drop). */
  uploadObjectBytes(
    accountId: number,
    bucket: string,
    key: string,
    data: number[],
    contentType?: string
  ): Promise<{ success: boolean; key: string; size: number }> {
    return invoke("upload_object_bytes", {
      accountIdx: accountId,
      bucket,
      key,
      data,
      contentType: contentType ?? null,
    });
  },

  createFolder(
    accountId: number,
    bucket: string,
    prefix: string
  ): Promise<{ success: boolean; key: string }> {
    return invoke("create_folder", {
      accountIdx: accountId,
      bucket,
      prefix,
    });
  },

  rename(
    accountId: number,
    bucket: string,
    srcKey: string,
    dstKey: string
  ): Promise<{ success: boolean; src: string; dst: string }> {
    return invoke("rename_object", {
      accountIdx: accountId,
      bucket,
      srcKey,
      dstKey,
    });
  },

  batchDownload(
    accountId: number,
    bucket: string,
    keys: string[],
    saveDir: string,
    stripPrefix?: string
  ): Promise<{ success: boolean; downloaded: number; errors: string[] }> {
    return invoke("batch_download", {
      accountIdx: accountId,
      bucket,
      keys,
      saveDir,
      stripPrefix: stripPrefix ?? "",
    });
  },
};
