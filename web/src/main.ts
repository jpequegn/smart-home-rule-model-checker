import {
  createIcons,
  Play,
  ShieldCheck,
  Scissors,
  Upload,
  Download,
  Square,
  FileCode,
  ChevronRight,
  Home,
} from "lucide";
import yamlExample from "../../examples/alarm.yaml?raw";
import scenarioExample from "../../examples/alarm.json?raw";
import "./style.css";

type Row = {
  index: number;
  at: number;
  kind: string;
  automation: string | null;
  source: { line: number; column: number } | null;
  entity: string | null;
  before: string | null;
  after: string | null;
  detail: string;
  state: Record<string, string>;
};
type Simulation = {
  trace: Row[];
  final_state: Record<string, string>;
  unknown: string[];
  incomplete: string[];
  horizon: number;
  processed_steps: number;
};
type Check = {
  status: string;
  simulation: Simulation;
  violations: {
    invariant: string;
    at: number;
    trace_index: number;
    message: string;
  }[];
};
type Result = {
  status?: string;
  replay?: Check;
  simulation?: Simulation;
  violations?: Check["violations"];
  check?: Check;
  lint?: {
    findings: {
      code: string;
      certainty: string;
      message: string;
      sources: { line: number; column: number }[];
    }[];
  };
  verification?: { status: string; cases: number; reasons: string[] };
  witness?: unknown;
  scenario?: unknown;
  cases?: number;
  reasons?: string[];
};
const el = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T;
document.querySelector("#app")!.innerHTML = `
<header><div class="brand"><i data-lucide="home"></i><div><h1>Homecheck</h1><span>Smart home rule model checker</span></div></div><span class="offline">Offline model · No device access</span></header>
<div class="toolbar"><label>Scenario <select id="fixture"><option value="alarm">Delayed light-off + alarm</option><option value="safe">Guarded light-off</option><option value="cycle">Light toggle loop</option><option value="unknown">Unsupported service</option></select></label><div class="tools"><button id="import" title="Import local project" aria-label="Import local project"><i data-lucide="upload"></i></button><button id="export" title="Export project" aria-label="Export project"><i data-lucide="download"></i></button><input hidden type="file" id="file" accept=".json"><button id="run"><i data-lucide="play"></i>Run</button><button id="verify"><i data-lucide="shield-check"></i>Verify</button><button id="minimize"><i data-lucide="scissors"></i>Minimize</button><button id="cancel" disabled title="Cancel analysis" aria-label="Cancel analysis"><i data-lucide="square"></i></button></div></div>
<main><section class="inputs"><div class="tabs" role="tablist" aria-label="Input editor"><button role="tab" aria-selected="true" id="yaml-tab">Automations</button><button role="tab" aria-selected="false" id="scenario-tab">Scenario JSON</button></div><div class="editor-head"><span id="editor-name">automations.yaml</span><span id="source-location"></span></div><textarea id="yaml" aria-label="Automation YAML" spellcheck="false"></textarea><textarea id="scenario" aria-label="Scenario JSON" spellcheck="false" hidden></textarea><div class="limits"><label>Horizon (s)<input id="horizon" type="number" min="0" max="86400"></label><label>Search depth<input id="depth" type="number" min="0" max="5"></label></div></section>
<section class="results"><div class="result-head"><div><span class="eyebrow">Analysis</span><h2 id="status" role="status">Ready</h2></div><button id="evidence" disabled title="Export analysis evidence" aria-label="Export analysis evidence"><i data-lucide="download"></i></button></div><p id="summary"></p><div id="error" role="alert" hidden></div><div id="bounds"></div><div id="findings"></div><div id="violations"></div><div id="replay" hidden><div class="section-head"><h3>State timeline</h3><span id="position"></span></div><canvas id="timeline" aria-label="Entity state timeline"></canvas><label class="scrubber">Replay<input type="range" id="step" min="0" value="0" max="0"></label><div id="states"></div><div class="section-head"><h3>Event trace</h3><button id="use" hidden>Load counterexample</button></div><div class="table-wrap"><table><thead><tr><th>Time</th><th>Event</th><th>Automation / entity</th><th>Effect</th><th>Source</th></tr></thead><tbody id="trace"></tbody></table></div></div><details id="raw" hidden><summary>Structured evidence</summary><pre id="json"></pre></details></section></main><footer>Bounded simulation, not a household safety certificate. Unsupported behavior remains unknown.</footer>`;
createIcons({
  icons: {
    Play,
    ShieldCheck,
    Scissors,
    Upload,
    Download,
    Square,
    FileCode,
    ChevronRight,
    Home,
  },
});
const yaml = el<HTMLTextAreaElement>("yaml"),
  scenario = el<HTMLTextAreaElement>("scenario");
let worker: Worker | undefined,
  result: Result | undefined,
  rows: Row[] = [],
  generation = 0,
  busy = false;
function text(id: string, value: string) {
  el(id).textContent = value;
}
function save(name: string, value: unknown) {
  const a = document.createElement("a");
  const u = URL.createObjectURL(
    new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }),
  );
  a.href = u;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(u), 1000);
}
function tab(which: "yaml" | "scenario") {
  yaml.hidden = which !== "yaml";
  scenario.hidden = which !== "scenario";
  el("yaml-tab").setAttribute("aria-selected", String(which === "yaml"));
  el("scenario-tab").setAttribute(
    "aria-selected",
    String(which === "scenario"),
  );
  text("editor-name", which === "yaml" ? "automations.yaml" : "scenario.json");
}
function clear() {
  result = undefined;
  rows = [];
  el("replay").hidden = true;
  el("raw").hidden = true;
  el<HTMLButtonElement>("evidence").disabled = true;
  for (const id of ["findings", "violations", "bounds", "summary"])
    text(id, "");
  text("status", "Inputs changed");
  el("status").className = "";
}
function controls(value: boolean) {
  busy = value;
  for (const id of ["run", "verify", "minimize"])
    el<HTMLButtonElement>(id).disabled = value;
  el<HTMLButtonElement>("cancel").disabled = !value;
}
function cancel() {
  generation++;
  worker?.terminate();
  worker = undefined;
  controls(false);
}
function changed() {
  cancel();
  clear();
}
function limits() {
  try {
    const s = JSON.parse(scenario.value);
    el<HTMLInputElement>("horizon").value = s.limits.horizon;
    el<HTMLInputElement>("depth").value = s.limits.search_depth;
  } catch {
    /* Editor can contain incomplete JSON. */
  }
}
function load(kind: string) {
  cancel();
  clear();
  let y = yamlExample;
  const s = JSON.parse(scenarioExample);
  if (kind === "safe")
    y = y.replace(
      "    - delay: 3",
      '    - delay: 3\n    - condition: state\n      entity_id: input_boolean.alarm\n      state: "off"',
    );
  if (kind === "unknown") y = y.replace("light.turn_on", "lock.unlock");
  if (kind === "cycle") {
    y =
      "- id: toggle-loop\n  mode: parallel\n  triggers:\n    - trigger: state\n      entity_id: light.hall\n  actions:\n    - action: light.toggle\n      target:\n        entity_id: light.hall\n";
    s.events = [
      { at: 0, input: { kind: "set", entity: "light.hall", value: "on" } },
    ];
    s.invariants = [];
    s.title = "Repeated toggling";
  }
  if (kind === "cycle") {
    s.invariants = [
      {
        kind: "transition",
        id: "no-toggle-off",
        entity: "light.hall",
        from: "on",
        to: "off",
      },
    ];
    s.alphabet = [{ kind: "set", entity: "light.hall", value: "on" }];
    s.gaps = [0];
  }
  yaml.value = y;
  scenario.value = JSON.stringify(s, null, 2);
  limits();
  text("status", "Ready");
  tab("yaml");
}
function jump(line: number) {
  tab("yaml");
  const start = yaml.value
    .split("\n")
    .slice(0, line - 1)
    .reduce((n, s) => n + s.length + 1, 0);
  yaml.focus();
  yaml.setSelectionRange(
    start,
    yaml.value.indexOf("\n", start) < 0
      ? yaml.value.length
      : yaml.value.indexOf("\n", start),
  );
  yaml.scrollTop = Math.max(0, (line - 5) * 20);
  text("source-location", "Line " + line);
}
function box(parent: HTMLElement, title: string, body: string) {
  const d = document.createElement("div");
  d.className = "finding";
  const strong = document.createElement("strong");
  strong.textContent = title;
  const p = document.createElement("p");
  p.textContent = body;
  d.append(strong, p);
  parent.append(d);
  return d;
}
function render(r: Result) {
  result = r;
  el("error").hidden = true;
  el<HTMLButtonElement>("evidence").disabled = false;
  const c =
    r.replay ?? r.check ?? (r.simulation ? (r as unknown as Check) : undefined);
  const sim = c?.simulation;
  const status =
    r.status ??
    (c?.status && c.status !== "no_violation_in_trace"
      ? c.status
      : undefined) ??
    r.verification?.status ??
    c?.status ??
    "Complete";
  text("status", status.replaceAll("_", " "));
  el("status").className =
    /counterexample|violation/.test(status) && !status.startsWith("no_")
      ? "bad"
      : /unknown|inconclusive|incomplete/.test(status)
        ? "warn"
        : "";
  text(
    "summary",
    sim
      ? `${sim.processed_steps} steps · ${sim.trace.length} trace entries · ${sim.horizon}s observation`
      : "Bounded event-sequence search",
  );
  const s = JSON.parse(scenario.value);
  const cases = r.cases ?? r.verification?.cases;
  text(
    "bounds",
    `Depth ≤ ${s.limits.search_depth} · Cases ≤ ${s.limits.max_cases}${cases !== undefined ? " · Explored " + cases : ""} · Gaps: ${s.gaps.join(", ")}s · Alphabet: ${s.alphabet.length} events. Source-order internal scheduling.`,
  );
  el("findings").replaceChildren();
  for (const f of r.lint?.findings ?? []) {
    const d = box(
      el("findings"),
      f.code.replaceAll("_", " ") + " · " + f.certainty,
      f.message,
    );
    for (const src of f.sources) {
      const b = document.createElement("button");
      b.textContent = "Line " + src.line;
      b.onclick = () => jump(src.line);
      d.append(b);
    }
  }
  for (const reason of [
    ...(r.reasons ?? r.verification?.reasons ?? []),
    ...(sim?.unknown ?? []),
    ...(sim?.incomplete ?? []),
  ])
    box(el("findings"), "Unresolved", reason);
  el("violations").replaceChildren();
  for (const v of c?.violations ?? []) {
    box(el("violations"), v.invariant, `${v.message} · detected at ${v.at}s`);
  }
  rows = sim?.trace ?? [];
  el("replay").hidden = rows.length === 0;
  el("trace").replaceChildren();
  for (const row of rows) {
    const tr = document.createElement("tr");
    tr.tabIndex = 0;
    tr.onclick = () => select(row.index);
    tr.onkeydown = (e) => {
      if (e.key === "Enter") select(row.index);
    };
    for (const value of [
      row.at + "s",
      row.kind,
      row.automation ?? row.entity ?? "scenario",
      row.after !== null ? `${row.before} → ${row.after}` : row.detail,
    ]) {
      const td = document.createElement("td");
      td.textContent = value;
      tr.append(td);
    }
    const td = document.createElement("td");
    if (row.source) {
      const b = document.createElement("button");
      b.textContent = "L" + row.source.line;
      b.onclick = (e) => {
        e.stopPropagation();
        jump(row.source!.line);
      };
      td.append(b);
    }
    tr.append(td);
    el("trace").append(tr);
  }
  el<HTMLInputElement>("step").max = String(Math.max(0, rows.length - 1));
  el<HTMLInputElement>("step").value = "0";
  el("use").hidden = !(r.witness || r.scenario);
  el("raw").hidden = false;
  text("json", JSON.stringify(r, null, 2));
  if (rows.length) select(0);
}
function select(index: number) {
  const row = rows[index];
  if (!row) return;
  el<HTMLInputElement>("step").value = String(index);
  text("position", `${row.at}s · ${index + 1} / ${rows.length}`);
  el("states").replaceChildren();
  for (const [entity, value] of Object.entries(row.state)) {
    const d = document.createElement("div");
    const name = document.createElement("span");
    name.textContent = entity;
    const state = document.createElement("strong");
    state.textContent = value;
    state.className =
      value === "on"
        ? "on"
        : value === "unknown" || value === "unavailable"
          ? "warn"
          : "";
    d.append(name, state);
    el("states").append(d);
  }
  Array.from(el("trace").children).forEach((tr, i) =>
    tr.classList.toggle("selected", i === index),
  );
  draw(index);
}
function draw(index: number) {
  const canvas = el<HTMLCanvasElement>("timeline");
  const ctx = canvas.getContext("2d")!;
  const names = Object.keys(rows[0].state);
  const width = canvas.clientWidth;
  const height = names.length * 32 + 26;
  const scale = devicePixelRatio || 1;
  canvas.width = width * scale;
  canvas.height = height * scale;
  canvas.style.height = height + "px";
  ctx.scale(scale, scale);
  ctx.fillStyle = "#f4f6f4";
  ctx.fillRect(0, 0, width, height);
  const max = Math.max(1, rows.at(-1)!.at);
  names.forEach((name, n) => {
    for (let i = 0; i < rows.length; i++) {
      const start = (rows[i].at / max) * width;
      const end = ((rows[i + 1]?.at ?? max) / max) * width;
      const value = rows[i].state[name];
      ctx.fillStyle =
        value === "on"
          ? "#27845b"
          : value === "unknown" || value === "unavailable"
            ? "#b17c16"
            : "#d9dfda";
      ctx.fillRect(start, n * 32 + 2, Math.max(1, end - start), 26);
    }
    ctx.fillStyle = "#14251a";
    ctx.font = "11px system-ui";
    ctx.fillText(name, 6, n * 32 + 19);
  });
  ctx.fillStyle = "#a92536";
  ctx.fillRect((rows[index].at / max) * (width - 2), 0, 2, height - 18);
  ctx.fillStyle = "#444";
  ctx.fillText("0s", 4, height - 3);
  ctx.fillText(max + "s", width - 32, height - 3);
}
function run(operation: string) {
  cancel();
  clear();
  el("error").hidden = true;
  controls(true);
  text("status", "Running");
  const token = ++generation;
  worker = new Worker(new URL("./worker.ts", import.meta.url), {
    type: "module",
  });
  worker.onmessage = (e) => {
    if (token !== generation) return;
    controls(false);
    worker?.terminate();
    worker = undefined;
    if (e.data.error) {
      text("error", e.data.error);
      el("error").hidden = false;
      text("status", "Invalid input");
      return;
    }
    try {
      render(e.data.result);
    } catch (err) {
      text("error", String(err));
      el("error").hidden = false;
      text("status", "Rendering error");
    }
  };
  worker.onerror = (e) => {
    controls(false);
    text("error", e.message);
    el("error").hidden = false;
    text("status", "Worker failed");
    worker?.terminate();
  };
  worker.postMessage({ operation, yaml: yaml.value, scenario: scenario.value });
}
el("yaml-tab").onclick = () => tab("yaml");
el("scenario-tab").onclick = () => tab("scenario");
yaml.oninput = changed;
scenario.oninput = () => {
  changed();
  limits();
};
el("run").onclick = () => run("report");
el("verify").onclick = () => run("verify");
el("minimize").onclick = () => run("minimize");
el("cancel").onclick = () => {
  cancel();
  text("status", "Cancelled");
};
el<HTMLInputElement>("step").oninput = (e) =>
  select(Number((e.target as HTMLInputElement).value));
for (const [id, key] of [
  ["horizon", "horizon"],
  ["depth", "search_depth"],
])
  el<HTMLInputElement>(id).onchange = () => {
    try {
      const s = JSON.parse(scenario.value);
      s.limits[key] = Number(el<HTMLInputElement>(id).value);
      scenario.value = JSON.stringify(s, null, 2);
      changed();
    } catch {
      text("error", "Correct the scenario JSON before changing limits.");
      el("error").hidden = false;
    }
  };
el<HTMLSelectElement>("fixture").onchange = (e) => {
  load((e.target as HTMLSelectElement).value);
  run("report");
};
el("import").onclick = () => el<HTMLInputElement>("file").click();
el<HTMLInputElement>("file").onchange = async (e) => {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  try {
    if (f.size > 524288) throw Error("Project exceeds 512 KiB");
    const v = JSON.parse(await f.text());
    if (typeof v.yaml !== "string" || !v.scenario)
      throw Error("Expected project with yaml and scenario");
    changed();
    yaml.value = v.yaml;
    scenario.value = JSON.stringify(v.scenario, null, 2);
    limits();
    run("report");
  } catch (err) {
    text("error", String(err));
    el("error").hidden = false;
  }
  (e.target as HTMLInputElement).value = "";
};
el("export").onclick = () => {
  try {
    save("project.homecheck.json", {
      yaml: yaml.value,
      scenario: JSON.parse(scenario.value),
    });
  } catch {
    text("error", "Scenario JSON is invalid.");
    el("error").hidden = false;
  }
};
el("evidence").onclick = () => save("homecheck-evidence.json", result);
el("use").onclick = () => {
  const s = result?.witness ?? result?.scenario;
  if (s) {
    scenario.value = JSON.stringify(s, null, 2);
    changed();
    limits();
    run("report");
  }
};
window.addEventListener("resize", () => {
  if (rows.length) draw(Number(el<HTMLInputElement>("step").value));
});
load("alarm");
run("report");
