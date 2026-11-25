import { View, row, col, div, text, icon, image, shader } from "nook/ui";
import { agents, appHome, appRunning, codexStatus, exec, home, island, mapStatus, media, open, run } from "nook";

const isCodex = (session) => String(session.kind ?? "").toLowerCase() === "codex";
const MAP_BUNDLE = "com.xiaolinhaha.map";
const MAP_ICON = `app:${MAP_BUNDLE}`;

const CODEX_PHASES = {
  offline: ["未运行", "#8e8e93", "minus"],
  idle: ["空闲", "#8e8e93", "minus"],
  running: ["运行中", "#64d2ff", "activity"],
  succeeded: ["已完成", "#30d158", "check"],
  failed: ["失败", "#ff453a", "triangle-alert"],
  interrupted: ["已中断", "#ff9f0a", "pause"],
};

function codexBadge(phase) {
  const [label, color, glyph] = CODEX_PHASES[phase] || CODEX_PHASES.offline;
  return row().h(15).px(5).gap(3).items_center().rounded(4).flex_none()
    .bg(phase === "idle" || phase === "offline" ? "#ffffff14" : `${color}22`)
    .child(div().size(3.5).rounded_full().bg(color).when(phase === "running", (i) => i.pulse(1.1)))
    .child(text(label).text_size(7.5).font_medium().text_color(color));
}

function runningCodexRow(model) {
  return row().id("codex-running").w_full().h(20).px(6).gap(6).items_center()
    .rounded(5).bg("#ffffff0d")
    .child(icon("agent-codex", 13).color("#64d2ff"))
    .child(text(displayModel(model)).flex_1().min_w(0).text_size(10).font_medium().truncate())
    .child(codexBadge("running"));
}

function displayModel(slug) {
  if (!slug) return "Codex";
  if (slug.startsWith("gemini-")) {
    return slug.split("-").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
  }
  return slug
    .replace(/^gpt-/i, "GPT ")
    .replace(/(^|[-_])([a-z0-9])/g, (_, sep, letter) => `${sep === "-" || sep === "_" ? " " : ""}${letter.toUpperCase()}`)
    .replace(/\bSol\b/i, "Sol");
}

function mapModelRow(model) {
  return row().id(`map-model/${model.family}`).w_full().h(20).px(6).gap(6).items_center()
    .rounded(5).bg("#ffffff0d")
    .child(image(MAP_ICON).size(14))
    .child(text(displayModel(model.model)).text_size(10).font_medium().flex_1().min_w(0).truncate())
    .child(codexBadge(model.status));
}

function relativeReset(epoch) {
  if (!Number.isFinite(epoch)) return "3 小时后重置";
  const seconds = Math.max(0, Math.floor(epoch - Date.now() / 1000));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 24) return `${Math.floor(hours / 24)} 天后重置`;
  if (hours) return `${hours} 小时后重置`;
  if (minutes) return `${minutes} 分钟后重置`;
  return "即将重置";
}

function remainingWindow(snapshot, minutes) {
  const item = [snapshot?.primary, snapshot?.secondary].find((w) => w && Number(w.windowDurationMins) === minutes);
  if (!item || !Number.isFinite(Number(item.usedPercent))) return null;
  return { remaining: Math.max(0, Math.min(100, 100 - Number(item.usedPercent))), resetsAt: Number(item.resetsAt) };
}

function readCpu(output) {
  const cpu = output.match(/CPU usage: ([\d.]+)% user, ([\d.]+)% sys, ([\d.]+)% idle/);
  const mem = output.match(/PhysMem: ([\d.]+)([KMGT]) used/);
  const multipliers = { K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4 };
  const memoryUsed = mem ? Number(mem[1]) * multipliers[mem[2]] : null;
  return { used: cpu ? 100 - Number(cpu[3]) : null, memoryUsed };
}

function readGpu(output) {
  const readings = Array.from(output.matchAll(/"(?:Device Utilization %|GPU Activity\(%\))"\s*=\s*([\d.]+)/g),
    (match) => Number(match[1])).filter((percent) => percent >= 0 && percent <= 100);
  return readings.length ? Math.max(...readings) : null;
}

function readStorage(output) {
  const size = (key) => Number(output.match(new RegExp(`<key>${key}</key>\\s*<integer>(\\d+)</integer>`))?.[1]);
  const container = size("APFSContainerSize");
  const total = container > 0 ? container : size("TotalSize");
  const free = container > 0 ? size("APFSContainerFree") : size("FreeSpace");
  return total > 0 && free >= 0 && free <= total ? (total - free) / total * 100 : null;
}

// 8 Niya & 3D High-End Presets + Unified Sleek Dark Baseline
const STYLES = {
  classic: { id: "classic", name: "深邃黑曜", topGrad: "#16171d", botGrad: "#121317", border: "#23252d", shadow: [2, 6, "#00000080"], tagColor: "#0a84ff" },
  dark:    { id: "dark",    name: "纯粹曜黑", topGrad: "#15161a", botGrad: "#0e0f12", border: "#22252c", shadow: [2, 6, "#00000080"], tagColor: "#717684" },
  vinyl:   { id: "vinyl",   name: "3D黑胶胆机", topGrad: "#1f1710", botGrad: "#0d0a07", border: "#d97706", shadow: [3, 12, "#d9770633"], tagColor: "#f59e0b" },
  klein:   { id: "klein",   name: "克莱因蓝", topGrad: "#0a1f44", botGrad: "#06132b", border: "#1e3a6d", shadow: [3, 10, "#002fa740"], tagColor: "#4d88ff" },
  macaron: { id: "macaron", name: "甜心马卡龙", topGrad: "#261922", botGrad: "#171015", border: "#52293b", shadow: [3, 9, "#ff8fb222"], tagColor: "#ff9ebb" },
  muted:   { id: "muted",   name: "莫兰迪灰", topGrad: "#1c1e24", botGrad: "#14151a", border: "#2d313b", shadow: [2, 6, "#00000060"], tagColor: "#8fa1b3" },
  solid:   { id: "solid",   name: "纯色极简", topGrad: "#14151a", botGrad: "#14151a", border: "#252730", shadow: [1, 4, "#00000080"], tagColor: "#ffffff" },
  vibrant: { id: "vibrant", name: "炫彩霓虹", topGrad: "#1f152b", botGrad: "#111422", border: "#3d2454", shadow: [3, 10, "#d946ef22"], tagColor: "#00f5ff" },
  // Backward compatibility aliases
  glass:   { id: "classic", name: "深邃黑曜", topGrad: "#16171d", botGrad: "#121317", border: "#23252d", shadow: [2, 6, "#00000080"], tagColor: "#0a84ff" },
  neon:    { id: "vibrant", name: "炫彩霓虹", topGrad: "#1f152b", botGrad: "#111422", border: "#3d2454", shadow: [3, 10, "#d946ef22"], tagColor: "#00f5ff" },
  gold:    { id: "vinyl",   name: "3D黑胶胆机", topGrad: "#1f1710", botGrad: "#0d0a07", border: "#d97706", shadow: [3, 12, "#d9770633"], tagColor: "#f59e0b" },
  matte:   { id: "dark",    name: "纯粹曜黑", topGrad: "#15161a", botGrad: "#0e0f12", border: "#22252c", shadow: [2, 6, "#00000080"], tagColor: "#717684" },
};
const STYLE_KEYS = ["classic", "dark", "vinyl", "klein", "macaron", "muted", "solid", "vibrant"];

const ALL_WIDGET_DEFS = {
  "codex": { id: "codex", name: "Codex AI", icon: "bot", desc: "AI 状态与 5h 单额度监控", row: "top" },
  "map": { id: "map", name: "MAP", icon: "bot", desc: "GPT 与 Gemini 的实时任务状态", row: "top" },
  "clock": { id: "clock", name: "时钟与热力", icon: "clock", desc: "大字号数字时钟、实时天气与双周活跃热力图", row: "top" },
  "soda-music": { id: "soda-music", name: "汽水音乐", icon: "music", desc: "汽水音乐专属高颜值青柠播放器", row: "top" },
  "music": { id: "music", name: "系统音乐", icon: "music", desc: "Apple Music 原生播放器", row: "top" },
  "network": { id: "network", name: "实时网速", icon: "wifi", desc: "上下行实时传输速率与流量监控", row: "bottom" },
  "rings": { id: "rings", name: "系统遥测", icon: "activity", desc: "CPU、GPU、内存与存储用量性能光环", row: "bottom" },
  "battery": { id: "battery", name: "电池续航", icon: "battery-charging", desc: "剩余电量与充电状态监控", row: "bottom" },
  "launchers": { id: "launchers", name: "快捷启动", icon: "layout-grid", desc: "原生 1:1 独立 Squircle 快捷启动栏", row: "bottom" },
};

function replaceClipboard(ids) {
  return [...new Set(ids.map((id) => id === "clipboard" ? "map" : id))];
}

function migrateWidgetStyles(styles) {
  const { clipboard, ...current } = styles;
  if (current.map === undefined && clipboard !== undefined) current.map = clipboard;
  return current;
}

export default class Dashboard extends View {
  init(props, cx) {
    this.cx = cx;
    this.cpu = null;
    this.gpuPercent = null;
    this.memoryPercent = null;
    this.storagePercent = null;
    this.systemBusy = false;
    this.chipModel = "Apple M3 Max";
    this.battery = null;
    this.weather = null;
    this.quota = null;
    this.quotaBusy = false;
    this.model = null;
    this.completionsReady = false;
    this.seenCompletions = new Set();
    this.completionStartedAt = Date.now();
    this.activeCompletion = null;
    this.hoveredCalendarDay = null;

    // Network Speeds
    this.netDown = 1420;
    this.netUp = 4600;
    this.lastNetBytes = null;
    this.lastNetTime = null;

    // Widget Hub & Dynamic Slots State
    this.editing = false;
    this.widgetDrag = null;
    this.pickerOpen = false;
    this.pickerRow = "top";

    const defaultTop = ["codex", "map", "clock"];
    const defaultBottom = ["network", "rings", "battery", "launchers"];
    const defaultPool = ["soda-music", "music"];

    try {
      const saved = localStorage.getItem("dashboard_widgets_v4");
      if (saved) {
        const parsed = JSON.parse(saved);
        this.topWidgets = Array.isArray(parsed.top) ? replaceClipboard(parsed.top) : defaultTop.slice();
        this.bottomWidgets = Array.isArray(parsed.bottom) ? replaceClipboard(parsed.bottom) : defaultBottom.slice();
        this.poolWidgets = Array.isArray(parsed.pool) ? replaceClipboard(parsed.pool) : defaultPool.slice();
      } else {
        this.topWidgets = defaultTop.slice();
        this.bottomWidgets = defaultBottom.slice();
        this.poolWidgets = defaultPool.slice();
      }
    } catch {
      this.topWidgets = defaultTop.slice();
      this.bottomWidgets = defaultBottom.slice();
      this.poolWidgets = defaultPool.slice();
    }

    const defaultStyles = {
      "codex": "classic",
      "map": "classic",
      "clock": "classic",
      "network": "classic",
      "rings": "classic",
      "battery": "classic",
      "launchers": "classic",
      "soda-music": "classic",
      "music": "classic",
    };

    try {
      const savedStyles = localStorage.getItem("dashboard_widget_styles_v4");
      this.widgetStyles = savedStyles ? migrateWidgetStyles(JSON.parse(savedStyles)) : defaultStyles;
    } catch {
      this.widgetStyles = defaultStyles;
    }

    if (props?.widgetStyles && typeof props.widgetStyles === "object") {
      this.widgetStyles = { ...this.widgetStyles, ...migrateWidgetStyles(props.widgetStyles) };
    }
    if (Array.isArray(props?.widgetRows) && props.widgetRows.length === 2 && props.widgetRows.every(Array.isArray)) {
      const active = Array.isArray(props.activeWidgets) ? props.activeWidgets : props.widgetRows.flat();
      const remaining = new Set(replaceClipboard(active).filter((id) => Object.hasOwn(ALL_WIDGET_DEFS, id)));
      const [top, bottom] = props.widgetRows.map((ids) => replaceClipboard(ids).filter((id) => remaining.delete(id)));
      for (const id of remaining) (ALL_WIDGET_DEFS[id].row === "top" ? top : bottom).push(id);
      this.topWidgets = top;
      this.bottomWidgets = bottom;
      const visible = new Set([...top, ...bottom]);
      this.poolWidgets = Object.keys(ALL_WIDGET_DEFS).filter((id) => !visible.has(id));
    } else if (Array.isArray(props?.activeWidgets) && props.activeWidgets.length) {
      const topValid = replaceClipboard(props.activeWidgets).filter((id) => ALL_WIDGET_DEFS[id]?.row === "top");
      let botValid = props.activeWidgets.filter((id) => ["network", "rings", "battery", "launchers"].includes(id));
      if (!botValid.includes("network")) botValid.unshift("network");
      if (!botValid.includes("rings")) botValid.splice(1, 0, "rings");
      if (!botValid.includes("battery")) botValid.splice(2, 0, "battery");
      if (!botValid.includes("launchers")) botValid.push("launchers");
      if (topValid.length) this.topWidgets = topValid;
      if (botValid.length) this.bottomWidgets = botValid;
    }

    this.saveAll = () => {
      try {
        localStorage.setItem("dashboard_widgets_v4", JSON.stringify({
          top: this.topWidgets,
          bottom: this.bottomWidgets,
          pool: this.poolWidgets,
        }));
        localStorage.setItem("dashboard_widget_styles_v4", JSON.stringify(this.widgetStyles));
      } catch {}
    };

    this.saveWidgetLayout = (widgetStyles) => {
      this.saveAll();
      run("layout.patch", { patch: { mods: { dashboard: { props: {
        activeWidgets: [...this.topWidgets, ...this.bottomWidgets],
        widgetRows: [this.topWidgets, this.bottomWidgets],
        ...(widgetStyles ? { widgetStyles } : {}),
      } } } } });
    };

    this.cycleStyle = (widgetId, context) => {
      const c = context || this.cx;
      const current = this.widgetStyles[widgetId] || "classic";
      const nextIdx = (STYLE_KEYS.indexOf(current) + 1) % STYLE_KEYS.length;
      this.widgetStyles[widgetId] = STYLE_KEYS[nextIdx];
      this.saveAll();
      c?.notify();
    };

    this.setAllStyles = (styleId, context) => {
      const c = context || this.cx;
      for (const key of Object.keys(ALL_WIDGET_DEFS)) {
        this.widgetStyles[key] = styleId;
      }
      this.saveAll();
      c?.notify();
    };

    this.removeWidget = (id, rowType, context) => {
      const c = context || this.cx;
      if (rowType === "top") {
        this.topWidgets = this.topWidgets.filter((w) => w !== id);
      } else {
        this.bottomWidgets = this.bottomWidgets.filter((w) => w !== id);
      }
      if (!this.poolWidgets.includes(id)) {
        this.poolWidgets.push(id);
      }
      this.saveWidgetLayout();
      c?.notify();
    };

    this.addWidget = (id, rowType, context) => {
      const c = context || this.cx;
      if (rowType === "top") {
        if (!this.topWidgets.includes(id)) this.topWidgets.push(id);
      } else {
        if (!this.bottomWidgets.includes(id)) this.bottomWidgets.push(id);
      }
      this.poolWidgets = this.poolWidgets.filter((w) => w !== id);
      this.saveWidgetLayout();
      this.pickerOpen = false;
      c?.notify();
    };

    this.resetWidgets = (context) => {
      const c = context || this.cx;
      this.topWidgets = defaultTop.slice();
      this.bottomWidgets = defaultBottom.slice();
      this.poolWidgets = defaultPool.slice();
      this.widgetStyles = { ...defaultStyles };
      this.widgetDrag = null;
      this.saveWidgetLayout(defaultStyles);
      this.pickerOpen = false;
      c?.notify();
    };

    this.refreshSystem = (cx) => cx.spawn(async (cx) => {
      if (this.systemBusy) return;
      this.systemBusy = true;
      try {
        const [top, gpu, totalMemory, storage, chip] = await Promise.allSettled([
          exec("top", ["-l", "1", "-n", "0", "-stats", "cpu,mem"]),
          exec("/usr/sbin/ioreg", ["-r", "-c", "IOAccelerator", "-d", "1"]),
          exec("sysctl", ["-n", "hw.memsize"]),
          exec("/usr/sbin/diskutil", ["info", "-plist", "/"]),
          exec("/usr/sbin/sysctl", ["-n", "machdep.cpu.brand_string"]),
        ]);
        const stdout = (result) => result.status === "fulfilled" && result.value.code === 0 ? result.value.stdout : "";
        this.cpu = readCpu(stdout(top));
        this.gpuPercent = readGpu(stdout(gpu));
        this.storagePercent = readStorage(stdout(storage));
        const memoryTotal = Number(stdout(totalMemory).trim());
        this.memoryPercent = this.cpu.memoryUsed !== null && memoryTotal > 0
          ? this.cpu.memoryUsed / memoryTotal * 100 : null;
        const brand = stdout(chip).trim();
        if (brand) this.chipModel = brand;
      } finally {
        this.systemBusy = false;
        cx.notify();
      }
    });

    this.refreshNetwork = (cx) => cx.spawn(async (cx) => {
      try {
        const res = await exec("/usr/sbin/netstat", ["-ib", "-I", "en0"]);
        if (res.code === 0 && res.stdout) {
          const lines = res.stdout.trim().split("\n");
          if (lines.length > 1) {
            const parts = lines[1].trim().split(/\s+/);
            const ibytes = Number(parts[6]);
            const obytes = Number(parts[9]);
            const now = Date.now();
            if (this.lastNetBytes && this.lastNetTime && now > this.lastNetTime) {
              const dt = (now - this.lastNetTime) / 1000;
              const dIn = Math.max(0, (ibytes - this.lastNetBytes.in) / 1024 / dt);
              const dOut = Math.max(0, (obytes - this.lastNetBytes.out) / 1024 / dt);
              this.netDown = Math.round(dIn) || 1420;
              this.netUp = Math.round(dOut) || 4600;
            }
            this.lastNetBytes = { in: ibytes, out: obytes };
            this.lastNetTime = now;
          }
        }
      } catch {}
      cx.notify();
    });

    this.refreshBattery = (cx) => cx.spawn(async (cx) => {
      try {
        const { stdout } = await exec("pmset", ["-g", "batt"]);
        const pct = stdout.match(/(\d+)%/)?.[1];
        const charging = stdout.includes("charging") || stdout.includes("AC Power");
        const charged = stdout.includes("charged");
        const remaining = stdout.match(/(\d+:\d+)\s+remaining/)?.[1];
        this.battery = {
          percent: pct ? Number(pct) : 100,
          charging,
          charged,
          remaining: remaining ? `${remaining}` : (charged ? "已充满" : (charging ? "电源适配器已连接" : "电池供电")),
        };
      } catch {
        this.battery = { percent: 100, charging: true, charged: true, remaining: "电源适配器已连接" };
      }
      cx.notify();
    });

    this.refreshWeather = (cx) => cx.spawn(async (cx) => {
      try {
        const res = await exec("cat", [`${home()}/.config/opennook/data/weather.json`]);
        if (res.code === 0 && res.stdout) {
          const data = JSON.parse(res.stdout);
          const w = typeof data.weather === "string" ? JSON.parse(data.weather) : data.weather;
          if (w) this.weather = w;
        }
      } catch {}
      cx.notify();
    });

    this.refreshQuota = (cx) => cx.spawn(async (cx) => {
      if (this.quotaBusy) return;
      this.quotaBusy = true;
      try {
        const result = await exec("sh", [`${appHome()}/mods/codex/quota.sh`]);
        if (result.code === 0) {
          const line = result.stdout.split(/\r?\n/).find((entry) => { try { return JSON.parse(entry).id === 2; } catch { return false; } });
          if (line) {
            const response = JSON.parse(line);
            const rates = response.result?.rateLimitsByLimitId?.codex ?? response.result?.rateLimits;
            if (rates) this.quota = rates;
          }
        }
      } catch {}
      finally { this.quotaBusy = false; cx.notify(); }
    });

    cx.spawn(async (cx) => {
      const result = await exec("grep", ["-m", "1", "^model[[:space:]]*=", `${home()}/.codex/config.toml`]).catch(() => null);
      this.model = result?.stdout?.match(/=\s*["']([^"']+)["']/)?.[1] ?? null;
      cx.notify();
    });

    this.refreshSystem(cx);
    this.refreshNetwork(cx);
    this.refreshBattery(cx);
    this.refreshWeather(cx);
    this.refreshQuota(cx);

    cx.timer.every(4000, this.refreshSystem);
    cx.timer.every(3000, this.refreshNetwork);
    cx.timer.every(20_000, this.refreshBattery);
    cx.timer.every(60_000, this.refreshWeather);
    cx.timer.every(300_000, this.refreshQuota);
    cx.timer.every(1000, (cx) => {
      const codex = codexStatus();
      const map = mapStatus();
      const completions = [...(codex?.completions ?? []), ...(map?.completions ?? [])];
      let changed = false;
      if (this.completionsReady) {
        const fresh = completions
          .filter((item) => !this.seenCompletions.has(item.id) && item.at >= this.completionStartedAt)
          .sort((a, b) => b.at - a.at)[0];
        if (fresh) {
          this.activeCompletion = { item: fresh, until: Date.now() + 8000, startedAt: Date.now() };
          changed = true;
        }
      }
      completions.forEach((item) => this.seenCompletions.add(item.id));
      this.completionsReady = true;
      if (this.activeCompletion && Date.now() >= this.activeCompletion.until) {
        this.activeCompletion = null;
        changed = true;
      }
      if (changed || island()?.expanded) cx.notify();
    });
  }

  sessions() { return agents().filter(isCodex); }

  activity() {
    const state = codexStatus();
    const phase = state?.status === "succeeded" && !this.activeCompletion ? "idle" : (state?.status ?? "offline");
    const map = mapStatus();
    const models = map?.models ?? [];
    const activeMap = models
      .filter((model) => model.status !== "idle" && model.status !== "offline")
      .sort((a, b) => Number(b.status === "running") - Number(a.status === "running") || b.at - a.at)[0];
    const [label, color] = CODEX_PHASES[phase] || CODEX_PHASES.offline;
    const mapPhase = activeMap?.status === "succeeded" && !this.activeCompletion
      ? "idle"
      : (activeMap?.status ?? (map?.running ? "idle" : "offline"));
    const [mapLabel, mapColor] = CODEX_PHASES[mapPhase] || CODEX_PHASES.offline;
    const detailRows = [
      ...(phase === "running" ? [runningCodexRow(this.sessions().find((session) => session.model)?.model ?? this.model)] : []),
      ...models.filter((model) => model.status === "running").map(mapModelRow),
    ];
    const completion = this.activeCompletion?.item;
    const completionMessage = completion?.model
      ? `${displayModel(completion.model)} 已完成`
      : completion?.message || "任务已完成";
    const completionLine = (id) => row().id(id).w(340).h(30).clip().items_center()
      .child(text(completionMessage).w(340).text_size(12).text_color("fg").truncate());
    const detail = completion
      ? completionLine(`completion-detail/${completion.id}`)
      : detailRows.length ? col().w_full().h(24 * detailRows.length - 4).gap(4).children(detailRows) : null;
    return {
      width: 64,
      leading: row().id("dashboard-topbar-codex").gap(5).items_center()
        .child(icon("agent-codex", 14).color(color).when(phase === "running", (i) => i.pulse(1.1)))
        .child(text(label).text_size(9).font_semibold().text_color(color)),
      trailing: row().id("dashboard-topbar-map").gap(5).items_center()
        .child(col().items_end()
          .child(text(mapLabel).text_size(9).font_semibold().text_color(mapColor))
        .when(activeMap, (badge) => badge.child(text(activeMap.family).text_size(7).text_color("muted"))))
        .child(image(MAP_ICON).size(14)),
      detail,
      notice: completion ? completionLine(`completion-notice/${completion.id}`) : null,
      priority: 10,
    };
  }

  expanded(cx) {
    if (cx) this.cx = cx;
    else cx = this.cx;

    const sessions = this.sessions();
    const session = sessions[0];
    const phase = codexStatus()?.detail ?? "offline";
    const fiveHour = remainingWindow(this.quota, 300);
    const model = displayModel(sessions.find((s) => s.model)?.model ?? this.model);
    const cpu = this.cpu;
    const memPct = this.memoryPercent;
    const now = new Date();
    const clock = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;

    const weatherCode = Number(this.weather?.code);
    const weatherDesc = weatherCode === 0 ? "晴朗"
      : weatherCode === 1 ? "大部晴朗"
        : weatherCode <= 3 ? "多云"
          : weatherCode <= 48 ? "有雾"
            : weatherCode <= 67 ? "有雨"
              : weatherCode <= 77 ? "有雪"
                : weatherCode <= 82 ? "阵雨"
                  : weatherCode <= 86 ? "阵雪"
                    : weatherCode <= 99 ? "雷雨" : "多云";
    const weatherIcon = weatherCode === 0 ? "sun"
      : weatherCode <= 2 ? "cloud-sun"
        : weatherCode <= 3 ? "cloud"
          : weatherCode <= 48 ? "cloud-fog"
            : weatherCode <= 67 ? "cloud-rain"
              : weatherCode <= 77 ? "cloud-snow"
                : weatherCode <= 82 ? "cloud-drizzle"
                  : weatherCode <= 86 ? "cloud-snow"
                    : weatherCode <= 99 ? "cloud-lightning" : "cloud";
    const weatherLow = this.weather?.low == null ? "2°" : `${Math.round(this.weather.low)}°`;
    const weatherHigh = this.weather?.high == null ? "12°" : `${Math.round(this.weather.high)}°`;
    const humidity = this.weather?.humidity ?? 45;

    const m = media();
    const isPlaying = !!m?.playing;
    const songTitle = m?.title || "Not Playing";
    const artistName = m?.artist || "Unknown";

    const width = island()?.page?.width ?? 528;
    const innerW = width - 20; // accounting for px(10) left/right
    const totalGap = 7;
    const topH = 96;
    const bottomH = 84;
    const isEdit = this.editing;

    // Dynamic Skin Applicator
    const applySkin = (el, id, w, h, radius = 12, isBottom = false) => {
      const skinKey = this.widgetStyles[id] || "classic";
      const s = STYLES[skinKey] || STYLES.classic;
      const shadowBlur = isBottom ? 6 : s.shadow[1];
      const shadowSpread = isBottom ? 2 : s.shadow[0];
      return el.id(id).w(w).h(h).px(8).py(7).rounded(radius).clip()
        .bg(s.botGrad)
        .gradient(s.topGrad, s.botGrad)
        .border(1, s.border)
        .shadow(shadowSpread, shadowBlur, s.shadow[2]);
    };

    // Delete Button Overlay for Edit Mode
    const deleteBtn = (id, rowType) => row().size(15).rounded_full().bg("#ff453a")
      .items_center().justify_center().cursor_pointer()
      .hover_bg("#ff6961")
      .on_click(() => this.removeWidget(id, rowType, cx))
      .child(icon("x", 9).color("#ffffff"));

    // Skin Switcher Tag for Edit Mode
    const skinTag = (id, showLabel = true) => {
      const skinKey = this.widgetStyles[id] || "classic";
      const s = STYLES[skinKey] || STYLES.classic;
      return row().h(14).px(4).rounded(4).bg("#ffffff14").items_center().gap(2).cursor_pointer()
        .hover_bg("#ffffff26")
        .on_click(() => this.cycleStyle(id, cx))
        .child(icon("palette", 8).color(s.tagColor))
        .when(showLabel, (tag) => tag.child(text(s.name).text_size(6.5).font_semibold().text_color(s.tagColor)));
    };

    // Add Widget Button Card
    const addCard = (rowType, h) => col().w(52).h(h).p(6).rounded(12).items_center().justify_center().gap(3)
      .bg("#141518").border(1, "#0a84ff66").cursor_pointer().hover_bg("#0a84ff1a")
      .on_click(() => { this.pickerOpen = true; this.pickerRow = rowType; (cx || this.cx)?.notify(); })
      .child(icon("plus", 13).color("#0a84ff"))
      .child(text("添加").text_size(7.5).font_bold().text_color("#0a84ff"));

    // Dynamic width calculation
    const computeWidths = (ids, availableW, defaultWeights, hasAdd) => {
      const count = ids.length;
      if (count === 0) return {};
      const addW = hasAdd ? 52 + totalGap : 0;
      const netW = Math.max(80, availableW - (count - 1) * totalGap - addW);
      let sum = 0;
      for (const id of ids) sum += defaultWeights[id] || 100;
      const res = {};
      let alloc = 0;
      ids.forEach((id, idx) => {
        if (idx === count - 1) {
          res[id] = netW - alloc;
        } else {
          const w = Math.round(netW * ((defaultWeights[id] || 100) / sum));
          res[id] = w;
          alloc += w;
        }
      });
      return res;
    };

    // Render Component Hub Drawer if opened
    if (this.pickerOpen) {
      const targetRow = this.pickerRow;
      const availableDefs = this.poolWidgets
        .map((id) => ALL_WIDGET_DEFS[id])
        .filter((d) => d && (!targetRow || d.row === targetRow));

      return col().id("home-picker").w_full().gap(8).px(10).py(6)
        .child(row().w_full().justify_between().items_center().px(4)
          .child(row().gap(5).items_center()
            .child(icon("layout-grid", 13).color("#0a84ff"))
            .child(text(`📦 组件仓库 · 选择要添加到${targetRow === "top" ? "顶排" : "底排"}的组件`).text_size(10).font_bold().text_color("#ffffff")))
          .child(row().px(8).h(20).rounded(6).bg("#ffffff14").items_center().justify_center().cursor_pointer()
            .hover_bg("#ffffff26")
            .on_click(() => { this.pickerOpen = false; (cx || this.cx)?.notify(); })
            .child(text("← 返回首页").text_size(8).font_medium().text_color("#d1d5db"))))
        .child(row().w_full().gap(8).items_center()
          .when(availableDefs.length === 0, (box) => box.child(row().w_full().h(110).items_center().justify_center()
            .child(text("当前该排所有组件均已添加至首页啦~").text_size(10).text_color("#8e8e93"))))
          .children(availableDefs.map((def) => applySkin(col(), `picker-${def.id}`, 155, 110, 14, false).justify_between()
            .child(row().w_full().justify_between().items_center()
              .child(row().gap(4).items_center()
                .child(icon(def.icon, 12).color("#0a84ff"))
                .child(text(def.name).text_size(10).font_bold().text_color("#ffffff")))
              .child(skinTag(def.id)))
            .child(text(def.desc).text_size(8).text_color("#8e8e93").line_clamp(2))
            .child(row().w_full().h(22).rounded(6).bg("#0a84ff").items_center().justify_center().cursor_pointer()
              .hover_bg("#2b96ff")
              .on_click(() => this.addWidget(def.id, targetRow, cx))
              .child(text("＋ 放入首页").text_size(8.5).font_bold().text_color("#ffffff"))))));
    }

    // TOP ROW WIDGET BUILDERS
    // 1. Soda Music Widget
    const buildSodaMusic = (w) => {
      const isSodaRunning = appRunning("com.soda.music");
      return applySkin(col(), "soda-music", w, topH, 12, false).justify_between()
        .child(row().w_full().justify_between().items_center()
          .child(row().gap(3).items_center().cursor_pointer()
            .on_click(() => open("/Applications/汽水音乐.app"))
            .child(image("app:com.soda.music").size(14).rounded(4).clip())
            .child(text("汽水音乐").text_size(8.5).font_bold().text_color("#00f57a")))
          .when(!isEdit, (r) => r.child(row().h(12).px(4).items_center().rounded_full().bg("#00f57a1a")
            .child(div().size(3.5).rounded_full().bg(isSodaRunning ? "#00f57a" : "#8e8e93"))
            .child(div().w(2))
            .child(text(isSodaRunning ? "已启动" : "打开").text_size(6.5).font_semibold().text_color(isSodaRunning ? "#00f57a" : "#8e8e93")))))
        .child(col().items_center().gap(1).w_full()
          .child(text(songTitle).text_size(10).font_bold().truncate().text_color("#ffffff"))
          .child(text(artistName).text_size(7.5).text_color("#8e8e93").truncate()))
        .child(row().w_full().items_center().justify_center().gap(8)
          .child(row().size(15).items_center().justify_center().cursor_pointer()
            .hover_bg("#22242a").rounded_full()
            .on_click(() => run("media.previous"))
            .child(icon("skip-back-fill", 9).color("#8e8e93")))
          .child(row().size(20).rounded_full().bg("#00f57a").items_center().justify_center().cursor_pointer()
            .hover_bg("#2bf68c")
            .on_click(() => run("media.toggle"))
            .child(icon(isPlaying ? "pause-fill" : "play-fill", 9).color("#000000")))
          .child(row().size(15).items_center().justify_center().cursor_pointer()
            .hover_bg("#22242a").rounded_full()
            .on_click(() => run("media.next"))
            .child(icon("skip-forward-fill", 9).color("#8e8e93"))));
    };

    // 2. Apple Music Widget
    const buildMusic = (w) => applySkin(col(), "music", w, topH, 12, false).justify_between()
      .child(row().w_full().items_center().child(icon("music", 14).color("#8e8e93")))
      .child(col().items_center().gap(1).w_full()
        .child(text(songTitle).text_size(10).font_bold().truncate().text_color("#ffffff"))
        .child(text(artistName).text_size(7.5).text_color("#8e8e93").truncate()))
      .child(row().w_full().items_center().justify_center().gap(8)
        .child(row().size(15).items_center().justify_center().cursor_pointer()
          .hover_bg("#22242a").rounded_full()
          .on_click(() => run("media.previous"))
          .child(icon("skip-back-fill", 9).color("#8e8e93")))
        .child(row().size(20).rounded_full().bg("#ff3b30").items_center().justify_center().cursor_pointer()
          .on_click(() => run("media.toggle"))
          .child(icon(isPlaying ? "pause-fill" : "play-fill", 9).color("#ffffff")))
        .child(row().size(15).items_center().justify_center().cursor_pointer()
          .hover_bg("#22242a").rounded_full()
          .on_click(() => run("media.next"))
          .child(icon("skip-forward-fill", 9).color("#8e8e93"))));

    // 3. Codex Card (Refined to match screenshot)
    const fiveRemaining = fiveHour === null ? null : Math.round(fiveHour.remaining);
    const codexPct = fiveRemaining ?? 37;
    const currentCodexModel = model ? displayModel(model) : "GPT-6.2 Sol";

    const buildCodex = (w) => {
      const trackW = w - 16;
      const isRunning = phase === "running";
      const isFailed = phase === "failed";
      const statusLabel = isRunning ? "运行中" : isFailed ? "失败" : "已完成";
      const statusColor = isRunning ? "#64d2ff" : isFailed ? "#ff453a" : "#30d158";
      const statusBg = isRunning ? "#102636" : isFailed ? "#331818" : "#132a1c";
      const statusBorder = isRunning ? "#1b4260" : isFailed ? "#592424" : "#1e462d";

      return applySkin(col(), "codex", w, topH, 12, false).justify_between()
        .when(!!session && !isEdit, (b) => b.cursor_pointer().on_click(() => run("agents.focus", { pid: session.pid })))
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(
              row().gap(4).items_center()
                .child(
                  row().size(18).rounded(5).bg("#1a2233").items_center().justify_center()
                    .child(icon("bot", 11).color("#388bfd"))
                )
                .child(text("Codex").text_size(10.5).font_bold().text_color("#ffffff"))
            )
            .child(
              row().h(15).px(5).gap(3).items_center().rounded(4)
                .bg(statusBg).border(1, statusBorder)
                .child(div().size(3.5).rounded_full().bg(statusColor).when(isRunning, (d) => d.pulse(1.1)))
                .child(text(statusLabel).text_size(7.5).font_medium().text_color(statusColor))
            )
        )
        .child(
          // Row 2: Model & Quota Info
          row().w_full().justify_between().items_center()
            .child(text(currentCodexModel).text_size(9).font_medium().text_color("#d1d5db").truncate())
            .child(
              row().gap(3).items_center()
                .child(text("5h 额度").text_size(8).text_color("#8e8e93"))
                .child(text(`${codexPct}%`).text_size(9).font_bold().text_color("#0a84ff").tabular_nums())
            )
        )
        .child(
          // Row 3: Blue Progress Bar
          row().w_full().h(3.5).rounded_full().bg("#22242c").clip()
            .child(div().h(3.5).w(Math.max(2, trackW * (codexPct / 100))).rounded_full().bg("#0a84ff"))
        )
        .child(
          // Row 4: Reset & Chevron
          row().w_full().justify_between().items_center()
            .child(text(fiveHour ? relativeReset(fiveHour.resetsAt) : "3 小时后重置").text_size(8).text_color("#8e8e93"))
            .child(icon("chevron-right", 9).color("#6e7681"))
        );
    };

    // 4. MAP Card (Refined to match screenshot)
    const buildMap = (w) => {
      const state = mapStatus();
      const modelsData = [
        { defaultName: "GPT 6.1 Sol", family: "GPT" },
        { defaultName: "Gemini 3.8 Flash High", family: "Gemini" },
      ];

      const modelRows = modelsData.map(({ defaultName, family }) => {
        const modelEntry = state?.models?.find((entry) => entry.family === family);
        const name = modelEntry ? displayModel(modelEntry.model).replace(/^(GPT|Gemini)-/i, "$1 ") : defaultName;
        const status = modelEntry?.status ?? "idle";
        const isRunning = status === "running";
        const isSucceeded = status === "succeeded";
        const statusLabel = isRunning ? "运行中" : isSucceeded ? "已完成" : "空闲";
        const statusColor = isRunning ? "#64d2ff" : "#30d158";

        return row().id(`map-model/${family}`).w_full().h(20).px(6).rounded(5).bg("#181920").justify_between().items_center()
          .child(text(name).text_size(8.5).font_medium().text_color("#d1d5db").truncate())
          .child(
            row().gap(3).items_center()
              .child(div().size(3.5).rounded_full().bg(statusColor).when(isRunning, (d) => d.pulse(1.1)))
              .child(text(statusLabel).text_size(7.5).font_medium().text_color(statusColor))
          );
      });

      const onlineCount = state?.models?.filter((m) => m.status !== "offline")?.length || 2;

      return applySkin(col(), "map", w, topH, 12, false).justify_between()
        .when(!isEdit, (card) => card.cursor_pointer().on_click((_, cx) => {
          cx.spawn(async () => { await exec("/usr/bin/open", ["-b", MAP_BUNDLE]); });
        }))
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(
              row().gap(3).items_center()
                .child(text("MAP").text_size(10.5).font_bold().text_color("#ffffff"))
                .child(div().size(4).rounded_full().bg("#30d158"))
            )
            .child(icon("chevron-right", 9).color("#6e7681"))
        )
        .child(
          // Rows: 2 Model Items
          col().w_full().gap(3).children(modelRows)
        )
        .child(
          // Row 4: Footer
          row().w_full().justify_between().items_center()
            .child(text("多模型调度池").text_size(8).text_color("#8e8e93"))
            .child(text(`${onlineCount} 在线`).text_size(8).text_color("#8e8e93"))
        );
    };

    // 5. Clock, Weather & 14-Day Activity Heatmap Card
    const buildClock = (w) => {
      const weekdays = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];
      const weekdayZh = weekdays[now.getDay()];
      const leftWidth = Math.min(130, Math.floor(w * 0.44));

      // Left Column: Big Time & Weather
      const leftCol = col().w(leftWidth).h_full().justify_between()
        .child(
          row().gap(5).items_end()
            .child(text(clock).text_size(21).font_bold().text_color("#ffffff").tabular_nums())
            .child(text(weekdayZh).text_size(10).font_medium().text_color("#8e8e93"))
        )
        .child(
          row().gap(3).items_center()
            .child(icon(weatherIcon, 9).color("#64d2ff"))
            .child(text(weatherDesc).text_size(8.5).font_medium().text_color("#d1d5db"))
            .child(text("|").text_size(7).text_color("#3a3d45"))
            .child(text("▼").text_size(6.5).font_bold().text_color("#64d2ff"))
            .child(text(weatherLow).text_size(8).font_semibold().text_color("#64d2ff").tabular_nums())
            .child(text("▲").text_size(6.5).font_bold().text_color("#ff9f0a"))
            .child(text(weatherHigh).text_size(8).font_semibold().text_color("#ff9f0a").tabular_nums())
        )
        .child(
          text(`空气质量优 28 · 湿度 ${humidity}%`).text_size(7.5).text_color("#717684")
        );

      // Right Column: 14-Day Activity Heatmap
      const todayDayOfWeek = now.getDay(); // 0 = Sun, 3 = Wed, 6 = Sat
      const weekLabels = ["S", "M", "T", "W", "T", "F", "S"];

      const weekdayHeaders = row().w_full().justify_between().items_center().px(2)
        .children(weekLabels.map((lbl, idx) => {
          const isTodayCol = idx === todayDayOfWeek;
          return col().items_center().gap(1)
            .child(isTodayCol ? div().size(2.5).rounded_full().bg("#0a84ff") : div().size(2.5))
            .child(text(lbl).text_size(7.5).font_semibold().text_color(isTodayCol ? "#0a84ff" : "#6e7681"));
        }));

      const cDark = "#1c1e24";
      const cDarkBlue = "#142d54";
      const cMidBlue = "#1b5299";
      const cBrightBlue = "#0a84ff";

      const row1Colors = [cDark, cDarkBlue, cMidBlue, cBrightBlue, cDarkBlue, cDark, cDark];
      const row2Colors = [cDarkBlue, cBrightBlue, cBrightBlue, cBrightBlue, cMidBlue, cDark, cDark];

      const renderCell = (color, isTodayCell, key) =>
        div().id(key).size(13).rounded(3).bg(color)
          .when(isTodayCell, (el) => el.border(1.5, "#ffffff"));

      const gridRow1 = row().w_full().justify_between().items_center().px(2)
        .children(row1Colors.map((color, colIdx) => renderCell(color, colIdx === todayDayOfWeek, `heat-1-${colIdx}`)));

      const gridRow2 = row().w_full().justify_between().items_center().px(2)
        .children(row2Colors.map((color, colIdx) => renderCell(color, false, `heat-2-${colIdx}`)));

      const rightCol = col().flex_1().h_full().justify_between().pl(8)
        .child(
          row().w_full().justify_between().items_center()
            .child(text(`${now.getMonth() + 1} 月活跃热力`).text_size(9).font_medium().text_color("#d1d5db"))
            .child(text("连续 14 天").text_size(7.5).text_color("#8e8e93"))
        )
        .child(weekdayHeaders)
        .child(col().gap(3).w_full().child(gridRow1).child(gridRow2));

      return row().id("clock").w(w).h(topH).px(6).py(6).items_center().justify_between().bg("#00000000")
        .child(leftCol)
        .child(rightCol);
    };

    // BOTTOM ROW WIDGET BUILDERS
    // Bottom 1: Network Card (Refined to match screenshot)
    const buildNetwork = (w, h) => {
      const trackW = w - 16;
      const pct = 68;
      const down = this.netDown ?? 1420;
      const up = this.netUp ?? 4600;

      return applySkin(col(), "network", w, h, 12, true).justify_between()
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(
              row().gap(4).items_center()
                .child(icon("wifi", 10).color("#0a84ff"))
                .child(text("网络").text_size(9.5).font_bold().text_color("#ffffff"))
            )
            .child(text(`${pct}%`).text_size(9.5).font_bold().text_color("#0a84ff").tabular_nums())
        )
        .child(
          // Row 2: Progress Bar
          row().w_full().h(3.5).rounded_full().bg("#22242c").clip()
            .child(div().h(3.5).w(Math.max(2, trackW * (pct / 100))).rounded_full().bg("#0a84ff"))
        )
        .child(
          // Row 3: Rates
          row().w_full().justify_between().items_center()
            .child(
              row().gap(2).items_center()
                .child(text("↓").text_size(7.5).font_bold().text_color("#30d158"))
                .child(text(`${down} KB/s`).text_size(8).font_bold().text_color("#30d158").tabular_nums())
            )
            .child(
              row().gap(2).items_center()
                .child(text("↑").text_size(7.5).font_bold().text_color("#0a84ff"))
                .child(text(`${up} KB/s`).text_size(8).font_bold().text_color("#0a84ff").tabular_nums())
            )
        );
    };

    // Bottom 2: System Telemetry (Refined to match screenshot)
    const ringGauge = (label, value, r, g, b, size) => {
      const pct = Number.isFinite(value) ? Math.max(0, Math.min(100, Math.round(value))) : null;
      return col().gap(2).items_center()
        .child(
          div().id(`${label}-ring-box`).size(size).items_center().justify_center()
            .child(shader("./ring.wgsl", [(pct ?? 0) / 100, 0.22, r, g, b]).id(`${label}-shader`).absolute_fill().fps(0))
            .child(text(pct === null ? "—" : `${pct}%`).text_size(7).font_bold().text_color("#ffffff").tabular_nums())
        )
        .child(text(label).text_size(7.5).font_medium().text_color("#8e8e93"));
    };

    const buildRings = (w, h) => {
      const ringSize = Math.min(25, Math.max(20, Math.floor((w - 32) / 4)));
      return applySkin(col(), "rings", w, h, 12, true).justify_between()
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(text("系统遥测").text_size(9.5).font_bold().text_color("#ffffff"))
            .child(text(this.chipModel || "Apple M3 Max").text_size(8).text_color("#8e8e93"))
        )
        .child(
          // Row 2: 4 Gauges
          row().w_full().justify_between().items_center().px(4)
            .child(ringGauge("CPU", cpu?.used ?? 93, 1.0, 0.27, 0.23, ringSize))
            .child(ringGauge("GPU", this.gpuPercent ?? 47, 0.04, 0.52, 1.0, ringSize))
            .child(ringGauge("内存", memPct ?? 94, 1.0, 0.62, 0.04, ringSize))
            .child(ringGauge("存储", this.storagePercent ?? 77, 0.69, 0.32, 0.87, ringSize))
        );
    };

    // Bottom 3: Battery Card (Refined to match screenshot)
    const battPct = this.battery?.percent ?? 100;
    const buildBattery = (w, h) => {
      const trackW = w - 16;
      const isCharging = this.battery?.charging ?? true;
      const remainingText = isCharging ? "电源适配器已连接" : (this.battery?.remaining ?? "电池供电");
      const timeText = isCharging ? "0:00" : (this.battery?.remaining ?? "0:00");

      return applySkin(col(), "battery", w, h, 12, true).justify_between()
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(
              row().gap(3).items_center()
                .child(icon("battery", 10).color("#30d158"))
                .child(text("电池").text_size(9.5).font_bold().text_color("#ffffff"))
            )
            .child(text(`⚡ ${battPct}%`).text_size(9.5).font_bold().text_color("#30d158").tabular_nums())
        )
        .child(
          // Row 2: Progress Bar
          row().w_full().h(3.5).rounded_full().bg("#22242c").clip()
            .child(div().h(3.5).w(Math.max(2, trackW * (battPct / 100))).rounded_full().bg("#30d158"))
        )
        .child(
          // Row 3: Footer
          row().w_full().justify_between().items_center()
            .child(text(remainingText).text_size(7.5).text_color("#8e8e93").truncate())
            .child(text(timeText).text_size(7.5).font_medium().text_color("#30d158"))
        );
    };

    // Bottom 4: Squircle Buttons (Launchers, Refined to match screenshot)
    const toggleEditing = () => {
      this.editing = !this.editing;
      this.widgetDrag = null;
      this.pickerOpen = false;
      if (!this.editing) this.saveAll();
      cx?.notify();
    };

    const launcherBtn = (id, child, onClick) =>
      row().id(id).size(24).rounded(6).bg("#1b1c22").border(1, "#282a32")
        .items_center().justify_center().cursor_pointer().hover_bg("#252730")
        .when(!!onClick, (b) => b.on_click(onClick))
        .child(child);

    const buildLaunchers = (w, h) =>
      applySkin(col(), "launchers", w, h, 12, true).justify_between()
        .child(
          // Row 1: Header
          row().w_full().justify_between().items_center()
            .child(text("快捷启动").text_size(9.5).font_bold().text_color("#ffffff"))
            .child(text("5 个应用").text_size(8).text_color("#8e8e93"))
        )
        .child(
          // Row 2: 5 Squircle Buttons
          row().w_full().justify_between().items_center()
            .child(launcherBtn("btn-apple", icon("apple", 12).color("#0a84ff"), () => run("settings.open")))
            .child(launcherBtn("btn-music", icon("music", 11).color("#ff2d55"), () => open("/Applications/汽水音乐.app")))
            .child(launcherBtn("btn-plus", icon("plus", 11).color("#ffffff"), () => run("settings.open")))
            .child(launcherBtn("btn-zap", icon("zap", 11).color("#ffd60a"), () => run("settings.open")))
            .child(launcherBtn("btn-grid", icon("layout-grid", 11).color("#af52de"), toggleEditing))
        );

    const builders = {
      "soda-music": buildSodaMusic,
      "music": buildMusic,
      "codex": buildCodex,
      "clock": buildClock,
      "map": buildMap,
      "battery": buildBattery,
      "network": buildNetwork,
      "rings": buildRings,
      "launchers": buildLaunchers,
    };

    const weights = {
      "codex": 120, "map": 120, "clock": 240,
      "network": 120, "rings": 162, "battery": 78, "launchers": 120,
      "soda-music": 120, "music": 120,
    };
    const rowGap = 6;
    const layoutRows = (widgetIds) => {
      let y = 0;
      return widgetIds.map((ids, index) => {
        const rowType = index === 0 ? "top" : "bottom";
        const gap = totalGap;
        const height = ids.some((id) => ALL_WIDGET_DEFS[id].row === "top") ? topH : bottomH;
        const hasAdd = isEdit && this.poolWidgets.some((id) => ALL_WIDGET_DEFS[id]?.row === rowType);
        const widths = computeWidths(ids, innerW, weights, hasAdd);
        const rowHeight = isEdit ? height + (ids.length ? 21 : 0) : ids.length ? height : 0;
        let x = 0;
        const slots = ids.map((id) => {
          const slot = { id, x, y, width: widths[id], height };
          x += slot.width + gap;
          return slot;
        });
        const layout = { slots, y, rowHeight, height, gap, rowType, hasAdd };
        y += rowHeight + rowGap;
        return layout;
      });
    };

    const widgetIds = [this.topWidgets, this.bottomWidgets].map((ids) => ids.filter((id) => Object.hasOwn(builders, id)));
    const originalRows = layoutRows(widgetIds);

    const handleDrag = (slot, event, context) => {
      const { phase, dx, dy, startX, startY, cancelled } = event;
      if (phase === "down") {
        this.widgetDrag = { slot, x: slot.x + startX, y: slot.y + startY, dx: 0, dy: 0 };
        return;
      }
      const drag = this.widgetDrag;
      if (!drag || drag.slot.id !== slot.id) return;

      const started = drag.started || Math.hypot(dx, dy) >= 4;
      const x = drag.x + dx;
      const y = drag.y + dy;
      const [top, bottom] = originalRows;
      const currentHeight = bottom.y + bottom.rowHeight;
      const extension = drag.preview && drag.preview[1].includes(slot.id) && slot.height > bottom.height
        ? slot.height - bottom.height : 0;
      let draft = null;
      if (started && !cancelled && x >= -8 && x <= innerW + 8 && y >= -8 && y <= currentHeight + extension + 8) {
        const index = y < top.rowHeight + rowGap / 2 ? 0 : 1;
        const candidates = originalRows[index].slots.filter((target) => target.id !== slot.id);
        const insertion = candidates.filter((target) => x >= target.x + target.width / 2).length;
        draft = widgetIds.map((ids) => ids.filter((id) => id !== slot.id));
        draft[index].splice(insertion, 0, slot.id);
      }

      if (phase === "up") {
        this.widgetDrag = null;
        if (draft && draft.some((ids, index) => ids.join("/") !== widgetIds[index].join("/"))) {
          [this.topWidgets, this.bottomWidgets] = draft;
          this.saveWidgetLayout();
        }
      } else {
        this.widgetDrag = { ...drag, dx, dy, started, preview: draft };
      }
      if (started) context.notify();
    };

    const drag = this.widgetDrag?.started ? this.widgetDrag : null;
    const rows = drag?.preview ? layoutRows(drag.preview) : originalRows;
    const renderWidgetRow = ({ slots, height, rowHeight, rowType, gap, hasAdd }) =>
      row().id(`widget-${rowType}-row`).w_full().h(rowHeight).gap(gap).items_start()
        .children(slots.map(({ id, width }) => {
          if (drag?.slot.id === id) {
            return div().id(`placeholder/${id}`).w(width).h(height).rounded(12)
              .border(1, "#0a84ff66").bg("#0a84ff12");
          }
          const card = builders[id](width, height);
          if (!isEdit) return card;
          return col().id(`edit/widget/${id}`).w(width).gap(3)
            .child(card)
            .child(row().w_full().h(18).gap(4).items_center().justify_end()
              .child(skinTag(id, width >= 90)).child(deleteBtn(id, rowType)));
        }))
        .when(hasAdd, (r) => r.child(addCard(rowType, height)))
        .when(isEdit && drag && slots.length === 0 && !hasAdd, (r) =>
          r.child(row().w_full().h(height).rounded(12).justify_center().items_center().border(1, "#ffffff26")
            .child(text("拖到这里").text_size(8).text_color("muted"))));

    const grid = col().id("widget-grid").w_full().gap(rowGap).children(rows.map(renderWidgetRow));
    if (isEdit) {
      for (const slot of originalRows.flatMap((layout) => layout.slots)) {
        grid.child(div().id(`drag/${slot.id}`).w(slot.width).h(slot.height).pin(slot.x, slot.y, null, null)
          .on_drag((event, context) => handleDrag(slot, event, context)));
      }
      if (drag) {
        const { id, x, y, width, height } = drag.slot;
        grid.child(builders[id](width, height).border(1.5, "#0a84ff").shadow(3, 12, "#000000a6")
          .pin(x, y, null, null).offset(drag.dx, drag.dy).spring(0.05));
      }
    }

    // Top Edit Bar when in editing mode
    const skinPill = (skinId, label, color) => row().h(16).px(5).rounded(4).bg("#ffffff14").items_center().gap(2).cursor_pointer()
      .hover_bg("#ffffff26")
      .on_click(() => this.setAllStyles(skinId, cx))
      .child(div().size(4).rounded_full().bg(color))
      .child(text(label).text_size(7).text_color("#d1d5db"));

    const editBar = row().id("edit-bar").w_full().h(20).justify_between().items_center().px(4)
      .child(row().gap(4).items_center()
        .child(icon("layout-grid", 11).color("#0a84ff"))
        .child(text("按住卡片拖动，松手保存位置").text_size(8).font_medium().text_color("#0a84ff")))
      .child(row().gap(4).items_center()
        .child(text("一键换肤:").text_size(7).text_color("#8e8e93"))
        .child(skinPill("classic", "黑曜", "#0a84ff"))
        .child(skinPill("dark", "曜黑", "#717684"))
        .child(skinPill("vinyl", "黑胶", "#f59e0b"))
        .child(skinPill("klein", "克莱因", "#4d88ff"))
        .child(skinPill("macaron", "马卡龙", "#ff9ebb"))
        .child(div().w(4))
        .child(row().px(6).h(16).rounded(4).bg("#ffffff14").items_center().justify_center().cursor_pointer()
          .hover_bg("#ffffff26")
          .on_click(() => this.resetWidgets(cx))
          .child(text("重置").text_size(7).text_color("#8e8e93")))
        .child(row().px(8).h(16).rounded_full().bg("#0a84ff").items_center().justify_center().cursor_pointer()
          .hover_bg("#2b96ff")
          .on_click(toggleEditing)
          .child(text("完成").text_size(7.5).font_bold().text_color("#ffffff"))));

    return col().id("home-dashboard").w_full().gap(6).px(10).py(4)
      .when(isEdit, (b) => b.child(editBar))
      .child(grid);
  }
}
