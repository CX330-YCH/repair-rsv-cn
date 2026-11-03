import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { defaultLanguage, text, type Language, type MessageKey } from "./i18n";
import "./styles.css";

type RepairState = "Ready" | "Running" | "Cancelled" | "Failed" | "Complete";
type Slot = "broken" | "donor" | "output";
type RepairEvent = {
  kind: "started" | "progress" | "status" | "complete" | "failed" | "cancelled";
  percent?: number; status?: string; outputPath?: string;
};
type MediaInfo = {
  sizeBytes: number; container: string; unindexed: boolean; readable: boolean;
  video: { codec: string; width: number; height: number; fps: number | null;
    durationSeconds: number | null; frameCount: number | null; pixelFormat: string | null; bitDepth: number } | null;
  audio: { codec: string; sampleRate: number; channels: number; bitDepth: number; durationSeconds: number | null }[];
};
type Metadata = { state: "idle" | "loading" | "ready" | "failed"; info?: MediaInfo };

const videoExtensions = ["mp4", "mov", "mxf", "m4v"];
let language = defaultLanguage(navigator.languages);
try {
  const saved = localStorage.getItem("repair-rsv-language");
  if (saved === "en" || saved === "zh-CN") language = saved;
} catch { /* Keep the system language if storage is unavailable. */ }
let brokenFile: string | null = null;
let donorFile: string | null = null;
let outputFile: string | null = null;
let state: RepairState = "Ready";
let status: MessageKey | null = null;
let cancelling = false;
let engineState: "preparing" | "ready" | "failed" = "preparing";
let logLines: string[] = [];
const metadata: Record<Slot, Metadata> = { broken: { state: "idle" }, donor: { state: "idle" }, output: { state: "idle" } };
const requests: Record<Slot, number> = { broken: 0, donor: 0, output: 0 };
const t = (key: MessageKey) => text(language, key);
const element = <T extends HTMLElement>(id: string): T => {
  const value = document.getElementById(id);
  if (!value) throw new Error(`Missing element: ${id}`);
  return value as T;
};
const brokenDrop = element<HTMLButtonElement>("brokenDrop");
const donorDrop = element<HTMLButtonElement>("donorDrop");
const languageSelect = element<HTMLSelectElement>("languageSelect");
const repairButton = element<HTMLButtonElement>("repairButton");
const cancelButton = element<HTMLButtonElement>("cancelButton");
const revealButton = element<HTMLButtonElement>("revealButton");
const engineRetry = element<HTMLButtonElement>("engineRetry");

brokenDrop.addEventListener("click", () => chooseFile("broken"));
donorDrop.addEventListener("click", () => chooseFile("donor"));
repairButton.addEventListener("click", startRepair);
cancelButton.addEventListener("click", cancelRepair);
revealButton.addEventListener("click", revealOutput);
engineRetry.addEventListener("click", prepareEngine);
languageSelect.addEventListener("change", () => {
  language = languageSelect.value === "zh-CN" ? "zh-CN" : "en";
  try { localStorage.setItem("repair-rsv-language", language); } catch { /* Optional preference. */ }
  render();
  syncMenuLanguage();
});
listen<RepairEvent>("repair-event", ({ payload }) => handleRepairEvent(payload));
getCurrentWebview().onDragDropEvent((event) => {
  if (event.payload.type !== "drop" || state === "Running" || engineState !== "ready") return;
  for (const path of event.payload.paths) {
    if (path.toLowerCase().endsWith(".rsv")) setFile("broken", path);
    else if (videoExtensions.some((extension) => path.toLowerCase().endsWith(`.${extension}`))) setFile("donor", path);
  }
});
render();
syncMenuLanguage();
// Give the preparing view a paint before requesting background engine setup.
requestAnimationFrame(() => requestAnimationFrame(() => { void prepareEngine(); }));

async function prepareEngine() {
  engineState = "preparing";
  render();
  try {
    await invoke("prepare_engine");
    engineState = "ready";
  } catch (error) {
    engineState = "failed";
    appendLog(String(error));
  }
  render();
}

function syncMenuLanguage() {
  invoke("set_language", { language }).catch((error) => appendLog(String(error)));
}
function appendLog(line: string) {
  logLines = [...logLines, ...line.split(/\r?\n/).filter(Boolean)].slice(-200);
  element("logText").textContent = logLines.join("\n");
  element("logPanel").hidden = logLines.length === 0;
}
async function chooseFile(slot: "broken" | "donor") {
  if (state === "Running" || engineState !== "ready") return;
  try {
    const selected = await open({ title: t(slot === "broken" ? "broken" : "donor"), multiple: false, directory: false,
      filters: slot === "broken" ? [{ name: "Sony RSV", extensions: ["rsv", "RSV"] }] : [{ name: t("videoFilter"), extensions: videoExtensions }] });
    if (typeof selected === "string") setFile(slot, selected);
  } catch (error) { appendLog(String(error)); status = "chooseError"; render(); }
}
function setFile(slot: "broken" | "donor", path: string) {
  if (engineState !== "ready") return;
  if (slot === "broken") brokenFile = path; else donorFile = path;
  outputFile = null;
  metadata.output = { state: "idle" };
  ++requests.output;
  state = "Ready";
  status = null;
  setProgress(0);
  render();
  void readMetadata(slot, path);
}
async function readMetadata(slot: Slot, path: string) {
  const request = ++requests[slot];
  metadata[slot] = { state: "loading" };
  renderMetadata(slot);
  try {
    const info = await invoke<MediaInfo>("probe_media", { filePath: path });
    if (request !== requests[slot]) return;
    metadata[slot] = { state: "ready", info };
  } catch (error) {
    if (request !== requests[slot]) return;
    metadata[slot] = { state: "failed" };
    appendLog(String(error));
  }
  renderMetadata(slot);
}
async function startRepair() {
  if (!brokenFile || !donorFile || state === "Running" || engineState !== "ready") return;
  state = "Running";
  status = "starting";
  cancelling = false;
  outputFile = null;
  metadata.output = { state: "idle" };
  ++requests.output;
  logLines = [];
  element("logText").textContent = "";
  element("logPanel").hidden = true;
  setProgress(0);
  render();
  try { await invoke("start_repair", { donorPath: donorFile, brokenPath: brokenFile }); }
  catch (error) { appendLog(String(error)); state = "Failed"; status = "startError"; render(); }
}
async function cancelRepair() {
  if (state !== "Running" || cancelling) return;
  cancelling = true;
  status = "cancelling";
  render();
  try { await invoke("cancel_repair"); }
  catch (error) { appendLog(String(error)); cancelling = false; status = "cancelError"; render(); }
}
async function revealOutput() {
  if (!outputFile) return;
  try { await invoke("reveal_output", { outputPath: outputFile }); }
  catch (error) { appendLog(String(error)); status = "revealError"; render(); }
}
function handleRepairEvent(event: RepairEvent) {
  if (typeof event.percent === "number") setProgress(event.percent);
  if (event.status) appendLog(event.status);
  if (event.kind === "started") { state = "Running"; status = "repairing"; }
  if (event.kind === "complete") { state = "Complete"; status = "completeStatus"; }
  if (event.kind === "failed") { state = "Failed"; status = "failedStatus"; }
  if (event.kind === "cancelled") { state = "Cancelled"; status = "cancelledStatus"; }
  if (event.outputPath) {
    outputFile = event.outputPath;
    void readMetadata("output", event.outputPath);
  }
  render();
}
function setProgress(percent: number) {
  const clamped = Math.max(0, Math.min(100, percent));
  element("progressFill").style.width = `${clamped}%`;
  element("progressText").textContent = `${Math.round(clamped)}%`;
  element("progressTrack").setAttribute("aria-valuenow", String(Math.round(clamped)));
}
function number(value: number, decimals = 2) {
  return new Intl.NumberFormat(language, { maximumFractionDigits: decimals }).format(value);
}
function seconds(value: number | null | undefined) {
  return value == null ? "—" : `${number(value)} ${t("seconds")}`;
}
function renderMetadata(slot: Slot) {
  const host = element(`${slot}Info`);
  const current = metadata[slot];
  host.replaceChildren();
  const note = (key: MessageKey) => {
    const p = document.createElement("p"); p.className = "info-note"; p.textContent = t(key); host.append(p);
  };
  if (current.state !== "ready" || !current.info) {
    if (current.state === "loading") note("loading");
    else if (current.state === "failed") note("infoFailed");
    return;
  }
  const info = current.info;
  const v = info.video;
  const values: [MessageKey, string][] = info.unindexed ? [
    ["format", "RSV"], ["size", `${number(info.sizeBytes / 1_000_000, 1)} MB`],
  ] : [
    ["resolution", v && v.width > 0 && v.height > 0 ? `${v.width} × ${v.height}` : "—"],
    ["frameRate", v?.fps ? `${number(v.fps, 3)} fps` : "—"],
    ["duration", seconds(v?.durationSeconds)],
    ["size", `${number(info.sizeBytes / 1_000_000, 1)} MB`],
  ];
  if (!info.unindexed && v) {
    const codec = v.codec === "h264" ? "H.264" : v.codec === "hevc" ? "HEVC" : v.codec.toUpperCase();
    const chroma = v.pixelFormat?.includes("422") ? "4:2:2" : v.pixelFormat?.includes("420") ? "4:2:0" : v.pixelFormat?.includes("444") ? "4:4:4" : null;
    values.push(["video", [codec, v.bitDepth > 0 ? `${v.bitDepth} bit` : null, chroma].filter(Boolean).join(" · ")]);
  } else if (!info.unindexed) values.push(["format", info.container === "Unknown" ? "—" : info.container]);
  const audio = info.audio.map((a) => [a.sampleRate > 0 ? `${number(a.sampleRate / 1000)} kHz` : null,
    a.channels === 1 ? t("mono") : a.channels === 2 ? t("stereo") : a.channels > 0 ? `${a.channels} ${t("channels")}` : null,
    a.bitDepth > 0 ? `${a.bitDepth} bit` : a.codec.toUpperCase()].filter(Boolean).join(" · ")).join(" / ");
  if (!info.unindexed) values.push(["audio", audio || (info.readable ? t("noAudio") : "—")]);
  if (slot === "output") {
    values.push(["frames", v?.frameCount != null ? number(v.frameCount, 0) : "—"]);
    values.push(["audioDuration", seconds(info.audio[0]?.durationSeconds)]);
  }
  const list = document.createElement("dl"); list.className = "info-grid";
  for (const [key, value] of values) {
    const cell = document.createElement("div");
    const label = document.createElement("dt"); label.textContent = t(key);
    const content = document.createElement("dd"); content.textContent = value; content.title = value;
    cell.append(label, content); list.append(cell);
  }
  host.append(list);
  if (!info.unindexed && !info.readable) note("unreadable");
}
function render() {
  document.documentElement.lang = language;
  languageSelect.value = language;
  for (const target of document.querySelectorAll<HTMLElement>("[data-i18n]")) {
    target.textContent = t(target.dataset.i18n as MessageKey);
  }
  languageSelect.setAttribute("aria-label", t("language"));
  element("inputGrid").setAttribute("aria-label", t("inputs"));
  element("brokenIcon").dataset.icon = brokenFile ? "document" : "plus";
  element("donorIcon").dataset.icon = donorFile ? "document" : "plus";
  element("brokenPath").textContent = brokenFile ?? t("brokenHint");
  element("donorPath").textContent = donorFile ?? t("donorHint");
  element("outputPath").textContent = outputFile ?? "";
  element("statusText").textContent = status ? t(status) : "";
  element("statusText").hidden = !status;
  element("resultGrid").hidden = !status && !outputFile;
  element("outputSummary").hidden = !outputFile;
  const stateKeys: Record<RepairState, MessageKey> = { Ready: "ready", Running: "running", Cancelled: "cancelled", Failed: "failed", Complete: "complete" };
  element("statePill").textContent = t(engineState === "ready" ? stateKeys[state] : engineState === "preparing" ? "enginePreparingShort" : "engineFailedShort");
  element("statePill").dataset.state = engineState === "ready" ? state.toLowerCase() : engineState;
  const engineReady = engineState === "ready";
  element("mainPage").hidden = !engineReady;
  element("enginePanel").hidden = engineReady;
  element("enginePanel").setAttribute("aria-busy", String(engineState === "preparing"));
  element("engineMessage").textContent = t(engineState === "failed" ? "engineFailed" : "enginePreparing");
  element("engineSpinner").hidden = engineState !== "preparing";
  engineRetry.hidden = engineState !== "failed";
  const running = state === "Running";
  repairButton.disabled = !engineReady || !brokenFile || !donorFile || running;
  cancelButton.disabled = !running || cancelling;
  revealButton.disabled = !outputFile || running;
  brokenDrop.disabled = donorDrop.disabled = running || !engineReady;
  for (const slot of ["broken", "donor", "output"] as const) renderMetadata(slot);
}
