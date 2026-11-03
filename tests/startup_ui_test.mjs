import { readFileSync } from "node:fs";
import vm from "node:vm";
import { test } from "node:test";
import assert from "node:assert/strict";
import ts from "../app/node_modules/typescript/lib/typescript.js";

const source = readFileSync(new URL("../app/src/main.ts", import.meta.url), "utf8");
const html = readFileSync(new URL("../app/index.html", import.meta.url), "utf8");
const parsed = ts.createSourceFile("main.ts", source, ts.ScriptTarget.Latest, true);
const body = parsed.statements.filter(statement => !ts.isImportDeclaration(statement)).map(statement => statement.getFullText(parsed)).join("\n");
const compiled = ts.transpileModule(body, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText;

function element() {
  return { hidden: false, disabled: false, dataset: {}, style: {}, handlers: {},
    textContent: "", setAttribute() {}, replaceChildren() {}, append() {},
    addEventListener(type, handler) { this.handlers[type] = handler; } };
}

function startApp() {
  const elements = new Map([...html.matchAll(/id="([^"]+)"/g)].map(match => [match[1], element()]));
  const frames = [], calls = [], preparations = [];
  let drop;
  const context = vm.createContext({
    exports: {}, console, navigator: { languages: ["zh-CN"] },
    document: { documentElement: {}, getElementById: id => elements.get(id), querySelectorAll: () => [], createElement: element },
    localStorage: { getItem: () => null, setItem() {} },
    defaultLanguage: () => "zh-CN", text: (_, key) => key,
    requestAnimationFrame: callback => frames.push(callback),
    listen: async () => {}, getCurrentWebview: () => ({ onDragDropEvent(callback) { drop = callback; } }),
    open: async () => { calls.push("file_picker"); return null; },
    invoke(command) {
      calls.push(command);
      if (command === "prepare_engine") return new Promise((resolve, reject) => preparations.push({ resolve, reject }));
      return Promise.resolve();
    },
  });
  vm.runInContext(compiled, context);
  return { elements, calls, preparations, paint() { while (frames.length) frames.shift()(); }, drop: payload => drop({ payload }) };
}

const settle = () => new Promise(resolve => setImmediate(resolve));

test("preparing view is rendered before initialization and blocks engine-dependent actions", async () => {
  const app = startApp(), ui = id => app.elements.get(id);
  assert.equal(ui("engineMessage").textContent, "enginePreparing");
  assert.equal(ui("enginePanel").hidden, false);
  assert.equal(ui("mainPage").hidden, true);
  assert.equal(ui("brokenDrop").disabled, true);
  assert.equal(app.calls.includes("prepare_engine"), false);
  app.paint();
  assert.equal(app.preparations.length, 1);
  await ui("brokenDrop").handlers.click();
  app.drop({ type: "drop", paths: ["broken.RSV", "donor.MP4"] });
  await ui("repairButton").handlers.click();
  assert.equal(app.calls.includes("file_picker"), false);
  assert.equal(app.calls.includes("probe_media"), false);
  assert.equal(app.calls.includes("start_repair"), false);
  app.preparations[0].resolve();
  await settle();
  assert.equal(ui("enginePanel").hidden, true);
  assert.equal(ui("mainPage").hidden, false);
  assert.equal(ui("statePill").textContent, "ready");
  assert.equal(ui("brokenDrop").disabled, false);
  assert.equal(ui("donorDrop").disabled, false);
  assert.equal(ui("repairButton").disabled, true); // Still needs the two input files.
});

test("initialization errors keep inputs blocked and retry can recover", async () => {
  const app = startApp(), ui = id => app.elements.get(id);
  app.paint();
  app.preparations[0].reject(new Error("Cache is temporarily unavailable"));
  await settle();
  assert.equal(ui("engineMessage").textContent, "engineFailed");
  assert.equal(ui("engineRetry").hidden, false);
  assert.equal(ui("mainPage").hidden, true);
  assert.equal(ui("brokenDrop").disabled, true);
  const retry = ui("engineRetry").handlers.click();
  assert.equal(ui("engineSpinner").hidden, false);
  assert.equal(app.preparations.length, 2);
  app.preparations[1].resolve();
  await retry;
  assert.equal(ui("enginePanel").hidden, true);
  assert.equal(ui("brokenDrop").disabled, false);
});
