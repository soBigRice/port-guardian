import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// 全局图标缓存，避免重复请求
const iconCache = new Map<string, string | null>();

// 已发出请求但尚未返回的 source（防止并发重复请求）
const pendingFetches = new Map<string, Promise<string | null>>();

function fetchIcon(source: string): Promise<string | null> {
  // 已缓存
  if (iconCache.has(source)) {
    return Promise.resolve(iconCache.get(source) ?? null);
  }
  // 已有 pending 请求
  if (pendingFetches.has(source)) {
    return pendingFetches.get(source)!;
  }

  const promise = invoke<string | null>("get_source_icon", { source })
    .then((dataUrl) => {
      iconCache.set(source, dataUrl);
      return dataUrl;
    })
    .catch(() => {
      iconCache.set(source, null);
      return null;
    })
    .finally(() => {
      pendingFetches.delete(source);
    });

  pendingFetches.set(source, promise);
  return promise;
}

interface Props {
  source: string;
  size?: number;
}

export default function SourceIcon({ source, size = 14 }: Props) {
  const [src, setSrc] = useState<string | null>(iconCache.get(source) ?? null);

  useEffect(() => {
    let cancelled = false;
    fetchIcon(source).then((dataUrl) => {
      if (!cancelled && dataUrl) {
        setSrc(dataUrl);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [source]);

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
