/** 修复 chrono 缺少小数点的时间字符串（RFC 3339 要求毫秒前有小数点） */
function fixChronoDate(isoString: string): string {
  // 匹配 "HH:MM:999999999Z"（没有小数点的纳秒格式）→ 修正为 "HH:MM:SS.fffZ"
  return isoString.replace(
    /(\d{2}:\d{2}):(\d{9})Z$/,
    (_, time, nanos) => {
      const secs = parseInt(nanos, 10) / 1_000_000_000;
      const whole = Math.floor(secs);
      const ms = Math.round((secs - whole) * 1000);
      return `${time}:${String(whole).padStart(2, "0")}.${String(ms).padStart(3, "0")}Z`;
    }
  );
}

/** 格式化时间戳为相对时间 */
export function formatRelativeTime(isoString: string): string {
  const fixed = fixChronoDate(isoString);
  const date = new Date(fixed);
  // 如果日期仍然无效，显示原始时间戳
  if (isNaN(date.getTime())) return isoString || "未知";
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSecs < 60) return "刚刚";
  if (diffMins < 60) return `${diffMins} 分钟前`;
  if (diffHours < 24) return `${diffHours} 小时前`;
  if (diffDays < 7) return `${diffDays} 天前`;

  return date.toLocaleDateString("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** 截断文本到指定长度 */
export function truncateText(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + "...";
}

/** 格式化字节大小为可读字符串 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** 获取内容类型的显示图标 */
export function getContentTypeIcon(type: string): string {
  switch (type) {
    case "text": return "📝";
    case "image": return "🖼️";
    case "files": return "📁";
    case "html": return "🌐";
    case "rtf": return "📄";
    default: return "📋";
  }
}

/** 获取内容类型的显示名称 */
export function getContentTypeLabel(type: string): string {
  switch (type) {
    case "text": return "文本";
    case "image": return "图片";
    case "files": return "文件";
    case "html": return "HTML";
    case "rtf": return "富文本";
    default: return "其他";
  }
}
