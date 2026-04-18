import { useEffect, useState } from "react";
import { api } from "./api";

interface UpdateInfo {
  latestVersion: string;
  releaseUrl: string;
}

const CACHE_KEY = "super-s3-update-check";
const CACHE_TTL = 4 * 60 * 60 * 1000; // 4 hours

function parseVersion(v: string): number[] {
  return v.replace(/^v/i, "").split(".").map(Number);
}

function isNewer(remote: string, local: string): boolean {
  const r = parseVersion(remote);
  const l = parseVersion(local);
  for (let i = 0; i < Math.max(r.length, l.length); i++) {
    const rv = r[i] ?? 0;
    const lv = l[i] ?? 0;
    if (rv > lv) return true;
    if (rv < lv) return false;
  }
  return false;
}

export function useUpdateCheck(currentVersion: string) {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);

  useEffect(() => {
    const cached = sessionStorage.getItem(CACHE_KEY);
    if (cached) {
      try {
        const { info, ts } = JSON.parse(cached);
        if (Date.now() - ts < CACHE_TTL) {
          if (info && isNewer(info.latestVersion, currentVersion)) {
            setUpdate(info);
          }
          return;
        }
      } catch { /* ignore */ }
    }

    let cancelled = false;

    api
      .checkUpdate()
      .then((info) => {
        if (cancelled) return;
        sessionStorage.setItem(
          CACHE_KEY,
          JSON.stringify({ info, ts: Date.now() })
        );
        if (isNewer(info.latestVersion, currentVersion)) {
          setUpdate(info);
        }
      })
      .catch(() => {
        // silently ignore
      });

    return () => {
      cancelled = true;
    };
  }, [currentVersion]);

  return update;
}
