import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// 全局图标缓存，避免重复请求
const iconCache = new Map<string, string | null>();

// 已发出请求但尚未返回的缓存键（source + executablePath）
const pendingFetches = new Map<string, Promise<string | null>>();

function getCacheKey(source: string, executablePath?: string): string {
  return `${source}::${executablePath ?? ""}`;
}

function fetchIcon(source: string, executablePath?: string): Promise<string | null> {
  const cacheKey = getCacheKey(source, executablePath);

  // 已缓存
  if (iconCache.has(cacheKey)) {
    return Promise.resolve(iconCache.get(cacheKey) ?? null);
  }
  // 已有 pending 请求
  if (pendingFetches.has(cacheKey)) {
    return pendingFetches.get(cacheKey)!;
  }

  const promise = invoke<string | null>("get_source_icon", {
    source,
    executablePath: executablePath || null,
  })
    .then((dataUrl) => {
      iconCache.set(cacheKey, dataUrl);
      return dataUrl;
    })
    .catch(() => {
      iconCache.set(cacheKey, null);
      return null;
    })
    .finally(() => {
      pendingFetches.delete(cacheKey);
    });

  pendingFetches.set(cacheKey, promise);
  return promise;
}

interface Props {
  source: string;
  executablePath?: string;
  size?: number;
}

export default function SourceIcon({ source, executablePath, size = 14 }: Props) {
  const cacheKey = getCacheKey(source, executablePath);
  const [src, setSrc] = useState<string | null>(iconCache.get(cacheKey) ?? null);

  useEffect(() => {
    setSrc(iconCache.get(cacheKey) ?? null);

    let cancelled = false;
    fetchIcon(source, executablePath).then((dataUrl) => {
      if (!cancelled) {
        setSrc(dataUrl ?? null);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [source, executablePath, cacheKey]);

  if (!src) {
    return null;
  }

  return (
    <img
      src={src}
      alt={source}
      className="source-icon"
      style={{
        width: size,
        height: size,
        marginRight: 4,
        verticalAlign: "middle",
        borderRadius: 2,
      }}
      title={source}
    />
  );
}
