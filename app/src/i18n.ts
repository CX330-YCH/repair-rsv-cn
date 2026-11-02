export type Language = "zh-CN" | "en";

const en = {
  eyebrow: "Sony RSV Recovery", language: "Language", inputs: "Repair inputs",
  broken: "Broken RSV", donor: "Donor Clip", brokenHint: "Drop .RSV file or click to browse",
  donorHint: "Drop donor video or click to browse", videoFilter: "Video",
  ready: "Ready", running: "Repairing", cancelled: "Cancelled", failed: "Failed", complete: "Complete",
  starting: "Starting repair…", repairing: "Repairing…",
  cancelling: "Cancelling repair…", cancelledStatus: "Repair cancelled.",
  completeStatus: "Output file",
  failedStatus: "Repair failed. Expand the repair log for details.",
  startError: "Could not start repair. See the repair log for details.",
  cancelError: "Could not cancel repair. See the repair log for details.",
  revealError: "Could not reveal the output. See the repair log for details.",
  chooseError: "Could not open the file picker. See the repair log for details.",
  repair: "Repair", cancel: "Cancel", reveal: "Reveal Output", logs: "Repair log",
  loading: "Reading…",
  infoFailed: "Could not read file information.", unreadable: "Media information unavailable.",
  resultTitle: "Recovered file",
  resolution: "Resolution", frameRate: "Frame rate", duration: "Video duration", size: "File size",
  format: "Format", video: "Video", audio: "Audio", audioDuration: "Audio duration", frames: "Frames",
  seconds: "s", channels: "ch", mono: "Mono", stereo: "Stereo", noAudio: "No audio track",
};
export type MessageKey = keyof typeof en;
const zh: Record<MessageKey, string> = {
  eyebrow: "SONY RSV 视频修复", language: "语言", inputs: "修复文件选择",
  broken: "待修复 RSV", donor: "参照视频", brokenHint: "拖入 .RSV 文件，或点击选择",
  donorHint: "拖入同模式参照视频，或点击选择", videoFilter: "视频",
  ready: "准备就绪", running: "正在修复", cancelled: "已取消", failed: "修复失败", complete: "修复完成",
  starting: "正在启动修复…", repairing: "正在修复…",
  cancelling: "正在取消修复…", cancelledStatus: "修复已取消。",
  completeStatus: "输出文件",
  failedStatus: "修复失败，请展开运行日志查看详情。",
  startError: "无法启动修复，请展开运行日志查看详情。",
  cancelError: "无法取消修复，请展开运行日志查看详情。",
  revealError: "无法显示输出文件，请展开运行日志查看详情。",
  chooseError: "无法打开文件选择窗口，请展开运行日志查看详情。",
  repair: "开始修复", cancel: "取消修复", reveal: "显示输出文件", logs: "运行日志",
  loading: "读取中…",
  infoFailed: "文件信息读取失败。", unreadable: "媒体参数不可用。",
  resultTitle: "修复文件信息",
  resolution: "分辨率", frameRate: "帧率", duration: "视频时长", size: "文件大小",
  format: "格式", video: "视频编码", audio: "音频", audioDuration: "音频时长", frames: "帧数",
  seconds: "秒", channels: "声道", mono: "单声道", stereo: "双声道", noAudio: "无音轨",
};

export function defaultLanguage(languages: readonly string[]): Language {
  return languages.some((value) => /^zh(?:-|$)/i.test(value)) ? "zh-CN" : "en";
}
export function text(language: Language, key: MessageKey): string {
  return (language === "zh-CN" ? zh : en)[key];
}
