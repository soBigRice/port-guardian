/**
 * 将 Tauri updater 或网络层返回的错误转换成用户能判断下一步的中文文案。
 * 这里保留原始错误中的关键信息，是为了排查 GitHub Release、latest.json 和签名问题时不再只看到“检查失败”。
 */
export function formatUpdateError(error: unknown): string {
  const raw =
    error instanceof Error
      ? error.message
      : typeof error === "string"
        ? error
        : JSON.stringify(error);
  const message = raw || "未知错误";
  const lower = message.toLowerCase();

  if (message.includes("404") || lower.includes("not found")) {
    return "更新源未找到 latest.json，请确认 GitHub Release 已公开并包含更新元数据。";
  }

  if (lower.includes("signature") || lower.includes("pubkey") || lower.includes("public key")) {
    return "更新签名校验失败，请确认发布私钥和应用内公钥匹配。";
  }

  if (lower.includes("network") || lower.includes("timeout") || lower.includes("failed to fetch")) {
    return "更新源连接失败，请检查网络或稍后重试。";
  }

  return message;
}
