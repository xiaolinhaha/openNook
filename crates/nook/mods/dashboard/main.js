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
  const [label, color, glyph] = CODEX_PHASES[phase];
  return row().h(14).px(4).gap(3).items_center().rounded(4).flex_none()
    .bg(phase === "idle" || phase === "offline" ? "#ffffff14" : `${color}22`)
    .child(icon(glyph, 8).color(color).when(phase === "running", (i) => i.pulse(1.1)))
    .child(text(label).text_size(8).font_semibold().text_color(color));
}

function completionRow(item, preview = false) {
  const time = new Date(item.at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
  const source = item.model ? "MAP" : "Codex";
  const message = item.model
    ? `${displayModel(item.model)} 已完成`
    : item.message.replace(/^[#>*\s]+/, "") || "Codex 任务已完成";
  return row().id(`codex-completion/${item.id}`).w_full().h(20).px(6).gap(6).items_center()
    .when(!preview, (r) => r.rounded(5).bg("#ffffff0d"))
    .child(item.model ? image(MAP_ICON).size(13) : icon("check", 11).color("#30d158"))
    .when(preview, (r) => r.child(text(`${source} 已完成`).text_size(10).font_semibold().text_color("#30d158").flex_none()))
    .child(text(message).flex_1().min_w(0).text_size(10).text_color("muted").truncate())
    .when(!preview, (r) => r.child(text(time).text_size(9).text_color("faint").tabular_nums().flex_none()));
}

function displayModel(slug) {
  if (!slug) return "Codex";
  if (slug.startsWith("gemini-")) {
    return slug.split("-").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(" ");
  }
  return slug.replace(/^gpt-/i, "GPT-").replace(/(^|[-_])([a-z])/g, (_, sep, letter) => `${sep === "-" || sep === "_" ? " " : ""}${letter.toUpperCase()}`);
}

function mapModelRow(model) {
  return row().id(`map-model/${model.family}`).w_full().h(20).px(6).gap(6).items_center()
    .rounded(5).bg("#ffffff0d")
    .child(image(MAP_ICON).size(14))
    .child(text(displayModel(model.model)).text_size(10).font_medium().flex_1().min_w(0).truncate())
    .child(codexBadge(model.status));
}

function relativeReset(epoch) {
  if (!Number.isFinite(epoch)) return "重置时间未知";
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

// 8 Niya & 3D High-End Presets + Backward Compatibility
const STYLES = {
  vinyl:   { id: "vinyl",   name: "3D黑胶胆机", topGrad: "#1f1710", botGrad: "#0d0a07", border: "#d97706", shadow: [3, 12, "#d9770633"], tagColor: "#f59e0b" },
  classic: { id: "classic", name: "3D玻璃", topGrad: "#252830", botGrad: "#141518", border: "#3c4250", shadow: [3, 8, "#00000080"], tagColor: "#0a84ff" },
  klein:   { id: "klein",   name: "克莱因蓝", topGrad: "#002fa7", botGrad: "#001a66", border: "#3369ff", shadow: [3, 10, "#002fa766"], tagColor: "#4d88ff" },
  macaron: { id: "macaron", name: "甜心马卡龙", topGrad: "#382333", botGrad: "#201824", border: "#ff8fb2", shadow: [3, 9, "#ff8fb233"], tagColor: "#ff9ebb" },
  dark:    { id: "dark",    name: "纯粹曜黑", topGrad: "#15161a", botGrad: "#0d0e10", border: "#22252c", shadow: [2, 6, "#00000080"], tagColor: "#717684" },
  muted:   { id: "muted",   name: "莫兰迪灰", topGrad: "#262b33", botGrad: "#191c22", border: "#464e5c", shadow: [2, 6, "#00000060"], tagColor: "#8fa1b3" },
  solid:   { id: "solid",   name: "纯色极简", topGrad: "#050505", botGrad: "#000000", border: "#404040", shadow: [1, 4, "#00000080"], tagColor: "#ffffff" },
  vibrant: { id: "vibrant", name: "炫彩霓虹", topGrad: "#2c0b3d", botGrad: "#0d1838", border: "#d946ef", shadow: [3, 10, "#d946ef40"], tagColor: "#00f5ff" },
  // Backward compatibility aliases
  glass:   { id: "classic", name: "3D玻璃", topGrad: "#252830", botGrad: "#141518", border: "#3c4250", shadow: [3, 8, "#00000080"], tagColor: "#0a84ff" },
  neon:    { id: "vibrant", name: "炫彩霓虹", topGrad: "#2c0b3d", botGrad: "#0d1838", border: "#d946ef", shadow: [3, 10, "#d946ef40"], tagColor: "#00f5ff" },
  gold:    { id: "vinyl",   name: "3D黑胶胆机", topGrad: "#1f1710", botGrad: "#0d0a07", border: "#d97706", shadow: [3, 12, "#d9770633"], tagColor: "#f59e0b" },
  matte:   { id: "dark",    name: "纯粹曜黑", topGrad: "#15161a", botGrad: "#0d0e10", border: "#22252c", shadow: [2, 6, "#00000080"], tagColor: "#717684" },
};
const STYLE_KEYS = ["vinyl", "classic", "klein", "macaron", "dark", "muted", "solid", "vibrant"];

const ALL_WIDGET_DEFS = {
  "soda-music": { id: "soda-music", name: "汽水音乐", icon: "music", desc: "汽水音乐专属高颜值青柠播放器", row: "top" },
  "music": { id: "music", name: "系统音乐", icon: "music", desc: "Apple Music 原生播放器", row: "top" },
  "codex": { id: "codex", name: "Codex AI", icon: "bot", desc: "AI 状态与 5h 单额度监控", row: "top" },
  "clock": { id: "clock", name: "时钟与天气", icon: "clock", desc: "大字号数字时钟与天气预报", row: "top" },
  "map": { id: "map", name: "MAP", icon: "bot", desc: "GPT 与 Gemini 的实时任务状态", row: "top" },
  "battery": { id: "battery", name: "电池续航", icon: "battery-charging", desc: "剩余电量与充电状态监控", row: "bottom" },
  "network": { id: "network", name: "实时网速", icon: "gauge", desc: "上下行实时传输速率监控", row: "bottom" },
  "rings": { id: "rings", name: "3D 性能光环", icon: "activity", desc: "CPU、GPU、内存与启动盘存储用量", row: "bottom" },
  "launchers": { id: "launchers", name: "应用快捷启动", icon: "layout-grid", desc: "Apple 原生 1:1 独立 Squircle 启动栏", row: "bottom" },
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
    this.battery = null;
    this.weather = null;
    this.quota = null;
    this.quotaBusy = false;
    this.model = null;

    // Widget Hub & Dynamic Slots State
    this.editing = false;
    this.widgetDrag = null;
    this.pickerOpen = false;
    this.pickerRow = "top";

    const defaultTop = ["soda-music", "codex", "clock", "map"];
    const defaultBottom = ["battery", "network", "rings", "launchers"];
    const defaultPool = ["music"];

    try {
      const saved = localStorage.getItem("dashboard_widgets_v3");
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

    // Default Per-Widget Styles (Klein Blue for Soda Music)
    const defaultStyles = {
      "soda-music": "klein",
      "codex": "classic",
      "clock": "macaron",
      "map": "dark",
      "battery": "muted",
      "network": "vibrant",
      "rings": "classic",
      "launchers": "solid",
    };

    try {
      const savedStyles = localStorage.getItem("dashboard_widget_styles_v3");
      this.widgetStyles = savedStyles ? migrateWidgetStyles(JSON.parse(savedStyles)) : defaultStyles;
      if (this.widgetStyles["soda-music"] === "vinyl") {
        this.widgetStyles["soda-music"] = "klein";
      }
      if (this.widgetStyles["clock"] === "klein") {
        this.widgetStyles["clock"] = "macaron";
      }
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
      let botValid = props.activeWidgets.filter((id) => ["battery", "network", "rings", "launchers"].includes(id));
      if (!botValid.includes("battery")) botValid.unshift("battery");
      if (!botValid.includes("network")) botValid.splice(1, 0, "network");
      if (!botValid.includes("rings")) botValid.splice(2, 0, "rings");
      if (!botValid.includes("launchers")) botValid.push("launchers");
      if (topValid.length) this.topWidgets = topValid;
      if (botValid.length) this.bottomWidgets = botValid;
    }

    this.saveAll = () => {
      try {
        localStorage.setItem("dashboard_widgets_v3", JSON.stringify({
          top: this.topWidgets,
          bottom: this.bottomWidgets,
          pool: this.poolWidgets,
        }));
        localStorage.setItem("dashboard_widget_styles_v3", JSON.stringify(this.widgetStyles));
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
        const [top, gpu, totalMemory, storage] = await Promise.allSettled([
          exec("top", ["-l", "1", "-n", "0", "-stats", "cpu,mem"]),
          exec("/usr/sbin/ioreg", ["-r", "-c", "IOAccelerator", "-d", "1"]),
          exec("sysctl", ["-n", "hw.memsize"]),
          exec("/usr/sbin/diskutil", ["info", "-plist", "/"]),
        ]);
        const stdout = (result) => result.status === "fulfilled" && result.value.code === 0 ? result.value.stdout : "";
        this.cpu = readCpu(stdout(top));
        this.gpuPercent = readGpu(stdout(gpu));
        this.storagePercent = readStorage(stdout(storage));
        const memoryTotal = Number(stdout(totalMemory).trim());
        this.memoryPercent = this.cpu.memoryUsed !== null && memoryTotal > 0
          ? this.cpu.memoryUsed / memoryTotal * 100 : null;
      } finally {
        this.systemBusy = false;
        cx.notify();
      }
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
          remaining: remaining ? `${remaining}` : (charged ? "已充满" : (charging ? "充电中" : "电源供电")),
        };
      } catch {
        this.battery = { percent: 100, charging: true, charged: true, remaining: "已充满" };
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
    this.refreshBattery(cx);
    this.refreshWeather(cx);
    this.refreshQuota(cx);

    cx.timer.every(5000, this.refreshSystem);
    cx.timer.every(30_000, this.refreshBattery);
    cx.timer.every(60_000, this.refreshWeather);
    cx.timer.every(300_000, this.refreshQuota);
    cx.timer.every(1000, (cx) => {
      if (island()?.expanded) cx.notify();
    });
  }

  sessions() { return agents().filter(isCodex); }

  activity() {
    const state = codexStatus();
    const phase = state?.status ?? "offline";
    const map = mapStatus();
    const models = map?.models ?? [];
    const activeMap = models
      .filter((model) => model.status !== "idle" && model.status !== "offline")
      .sort((a, b) => Number(b.status === "running") - Number(a.status === "running") || b.at - a.at)[0];
    const [label, color] = CODEX_PHASES[phase];
    const mapPhase = activeMap?.status ?? (map?.running ? "idle" : "offline");
    const [mapLabel, mapColor] = CODEX_PHASES[mapPhase];
    const completions = [...(state?.completions ?? []), ...(map?.completions ?? [])]
      .sort((a, b) => b.at - a.at).slice(0, 3);
    const latest = completions[0];
    const detailRows = [...models.map(mapModelRow), ...completions.map((item) => completionRow(item))];
    const detail = detailRows.length
      ? col().w_full().h(24 * detailRows.length - 4).gap(4).children(detailRows)
      : text("仪表盘").text_size(11).text_color("muted").truncate();
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
      notice: latest && Date.now() - latest.at < 6_000 ? completionRow(latest, true) : null,
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
    const date = `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日`;

    const weatherTemp = this.weather?.temperature ? `${Math.round(this.weather.temperature)}°` : "17°";
    const weatherPlace = this.weather?.place ?? "上海市";
    const weatherDesc = this.weather?.code === 0 ? "晴" : (this.weather?.code <= 3 ? "多云" : "阴");
    const weatherText = `☁️ ${weatherTemp} ${weatherDesc} · ${weatherPlace}`;

    const m = media();
    const isPlaying = !!m?.playing;
    const songTitle = m?.title || "Not Playing";
    const artistName = m?.artist || "Unknown";

    const width = island()?.page?.width ?? 528;
    const innerW = width - 20; // 508px accounting for px(10) left/right
    const totalGap = 7;
    const topH = 88;
    const bottomH = 48;
    const isEdit = this.editing;

    // Dynamic Skin Applicator
    const applySkin = (el, id, w, h, radius, isBottom) => {
      const skinKey = this.widgetStyles[id] || "classic";
      const s = STYLES[skinKey] || STYLES.classic;
      const shadowBlur = isBottom ? 6 : s.shadow[1];
      const shadowSpread = isBottom ? 2 : s.shadow[0];
      return el.id(id).w(w).h(h).p(isBottom ? 6 : 7).rounded(radius).clip()
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
      const s = STYLES[skinKey] || STYLES.glass;
      return row().h(14).px(4).rounded(4).bg("#ffffff14").items_center().gap(2).cursor_pointer()
        .hover_bg("#ffffff26")
        .on_click(() => this.cycleStyle(id, cx))
        .child(icon("palette", 8).color(s.tagColor))
        .when(showLabel, (tag) => tag.child(text(s.name).text_size(6.5).font_semibold().text_color(s.tagColor)));
    };

    // Add Widget Button Card
    const addCard = (rowType, h) => col().w(52).h(h).p(6).rounded(14).items_center().justify_center().gap(3)
      .bg("#141518").border(1, "#00f57a66").cursor_pointer().hover_bg("#00f57a1a")
      .on_click(() => { this.pickerOpen = true; this.pickerRow = rowType; (cx || this.cx)?.notify(); })
      .child(icon("plus", 13).color("#00f57a"))
      .child(text("添加").text_size(7.5).font_bold().text_color("#00f57a"));

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
            .child(icon("layout-grid", 13).color("#00f57a"))
            .child(text(`📦 组件仓库 · 选择要添加到${targetRow === "top" ? "顶排" : "底排"}的组件`).text_size(10).font_bold().text_color("#ffffff")))
          .child(row().px(8).h(20).rounded(6).bg("#ffffff14").items_center().justify_center().cursor_pointer()
            .hover_bg("#ffffff26")
            .on_click(() => { this.pickerOpen = false; (cx || this.cx)?.notify(); })
            .child(text("← 返回首页").text_size(8).font_medium().text_color("#d1d5db"))))
        .child(row().w_full().gap(8).items_center()
          .when(availableDefs.length === 0, (box) => box.child(row().w_full().h(110).items_center().justify_center()
            .child(text("当前该排所有组件均已添加至首页啦~").text_size(10).text_color("#8e8e93"))))
          .children(availableDefs.map((def) => applySkin(col(), `picker-${def.id}`, 155, 110, 16, false).justify_between()
            .child(row().w_full().justify_between().items_center()
              .child(row().gap(4).items_center()
                .child(icon(def.icon, 12).color("#00f57a"))
                .child(text(def.name).text_size(10).font_bold().text_color("#ffffff")))
              .child(skinTag(def.id)))
            .child(text(def.desc).text_size(8).text_color("#8e8e93").line_clamp(2))
            .child(row().w_full().h(22).rounded(6).bg("#00f57a").items_center().justify_center().cursor_pointer()
              .hover_bg("#2bf68c")
              .on_click(() => this.addWidget(def.id, targetRow, cx))
              .child(text("＋ 放入首页").text_size(8.5).font_bold().text_color("#000000"))))));
    }

    // TOP ROW WIDGET BUILDERS
    // 1. Soda Music Widget (汽水音乐专属高颜值青柠播放器)
    const buildSodaMusic = (w) => {
      const isSodaRunning = appRunning("com.soda.music");
      return applySkin(col(), "soda-music", w, topH, 16, false).justify_between()
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

    // 2. Generic Apple Music Widget
    const buildMusic = (w) => applySkin(col(), "music", w, topH, 16, false).justify_between()
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

    // 3. Codex Single-Quota Card
    const fiveRemaining = fiveHour === null ? null : Math.round(fiveHour.remaining);

    const buildCodex = (w) =>
      applySkin(col(), "codex", w, topH, 16, false).justify_between()
        .when(!!session && !isEdit, (b) => b.cursor_pointer().on_click(() => run("agents.focus", { pid: session.pid })))
        .child(row().w_full().gap(4).items_center()
          .child(row().gap(4).items_center().flex_1()
            .child(icon("bot", 11).color("#0a84ff"))
            .child(text("Codex").text_size(10).font_bold()))
          .child(codexBadge(phase)))
        .when(model !== "Codex", (card) => card.child(text(model).text_size(8.5).text_color("muted").truncate()))
        .child(col().w_full().gap(4)
          .child(row().w_full().justify_between().items_center()
            .child(row().gap(3).items_center()
              .child(icon("zap", 9).color("#30d158"))
              .child(text("5h 额度").text_size(8).font_medium().text_color("muted")))
            .child(text(fiveRemaining === null ? "—" : `${fiveRemaining}%`)
              .text_size(14).font_bold().text_color(fiveRemaining === null ? "muted" : "#30d158").tabular_nums()))
          .child(row().w_full().h(4).rounded_full().bg("#ffffff14").clip()
            .when(fiveRemaining > 0, (track) => track
              .child(div().h(4).w(Math.max(0, (w - 14) * fiveRemaining / 100)).rounded_full().bg("#30d158")))))
        .child(row().w_full().gap(4).items_center()
          .child(text(fiveHour ? relativeReset(fiveHour.resetsAt) : "额度暂不可用")
            .text_size(7.5).text_color("muted").flex_1().min_w(0).truncate())
          .when(!!session && !isEdit, (r) => r.child(icon("chevron-right", 9).color("muted"))));

    // 4. Clock Card
    const buildClock = (w) => applySkin(col(), "clock", w, topH, 16, false).items_center().justify_between()
      .child(row().w_full().items_center().child(text(date).text_size(8).font_medium().text_color("#e6e4dc").truncate()))
      .child(text(clock).text_size(26).font_bold().text_color("#ffe28a").tabular_nums())
      .child(text(weatherText).text_size(7.5).font_medium().text_color("#8e8e93").truncate());

    const buildMap = (w) => {
      const state = mapStatus();
      const models = ["GPT", "Gemini"].map((family) => {
        const model = state?.models.find((entry) => entry.family === family);
        const version = model ? displayModel(model.model).slice(family.length).replace(/^[-\s]+/, "") : "";
        const isGpt = family === "GPT";
        return row().id(`map-card/${family}`).w_full().h(23).px(5).gap(5).items_center()
          .rounded(5).bg("#ffffff0d")
          .child(icon(isGpt ? "agent-codex" : "agent-gemini", 11).color(isGpt ? "#64d2ff" : "#c29bff"))
          .child(col().flex_1().min_w(0)
            .child(text(family).text_size(8.5).font_semibold())
            .when(version, (c) => c.child(text(version).text_size(7.5).text_color("muted").truncate())))
          .child(model || !state?.running
            ? codexBadge(model?.status ?? "offline")
            : text("暂无记录").text_size(7).text_color("muted").flex_none());
      });

      return applySkin(col(), "map", w, topH, 16, false).justify_between()
        .when(!isEdit, (card) => card.cursor_pointer().on_click((_, cx) => {
          cx.spawn(async () => { await exec("/usr/bin/open", ["-b", MAP_BUNDLE]); });
        }))
        .child(row().w_full().justify_between().items_center()
          .child(row().gap(3).items_center()
            .child(image(MAP_ICON).size(14))
            .child(text("MAP").text_size(10).font_bold()))
          .when(!isEdit, (r) => r.child(icon("chevron-right", 10).color("muted"))))
        .child(col().w_full().gap(4).children(models));
    };

    // Bottom 1: Battery Card
    const battPct = this.battery?.percent ?? 100;
    const buildBattery = (w, h) => {
      const trackW = w - 16;
      return applySkin(col(), "battery", w, h, 12, true).justify_between()
        .child(row().w_full().justify_between().items_center()
          .child(text(`${battPct}%`).text_size(12).font_bold().text_color("#ffffff").tabular_nums())
          .child(icon("battery-charging", 12).color("#30d158")))
        .child(row().id("batt-track").w_full().h(3.5).rounded_full().bg("#ffffff14")
          .child(div().id("batt-fill").h(3.5).w(Math.max(2, trackW * (battPct / 100))).rounded_full().bg("#30d158")))
        .child(text(`⚡️ ${this.battery?.remaining ?? "电量充足"}`).text_size(7.5).font_medium().text_color("#30d158").truncate());
    };

    // Bottom 2: Network Card
    const buildNetwork = (w, h) => {
      const trackW = w - 16;
      return applySkin(col(), "network", w, h, 12, true).justify_between()
        .child(row().w_full().justify_between().items_center()
          .child(row().gap(2).items_center()
            .child(icon("gauge", 9).color("#ff9f0a"))
            .child(text("网络").text_size(7.5).font_bold().text_color("#8e8e93")))
          .child(text("68%").text_size(9).font_bold().text_color("#ffffff").tabular_nums()))
        .child(row().id("speed-track").w_full().h(3.5).rounded_full().bg("#ffffff14")
          .child(div().id("speed-fill").h(3.5).w(Math.max(2, trackW * 0.68)).rounded_full().bg("#ff9f0a")))
        .child(row().w_full().justify_between().items_center()
          .child(text("↓ 1420").text_size(8).font_bold().text_color("#ff9f0a").tabular_nums())
          .child(text("↑ 4600").text_size(8).font_bold().text_color("#8e8e93").tabular_nums()));
    };

    // Bottom 3: Rings Card
    const ringGauge = (label, value, r, g, b, colorHex, size) => {
      const pct = Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : null;
      return col().gap(2).items_center()
        .child(text(label).text_size(7.5).font_bold().text_color("#8e8e93"))
        .child(div().id(`${label}-ring-box`).size(size).items_center().justify_center()
          .child(shader("./ring.wgsl", [(pct ?? 0) / 100, 0.22, r, g, b]).id(`${label}-shader`).absolute_fill().fps(0))
          .child(text(pct === null ? "—" : `${Math.round(pct)}%`)
            .text_size(7).font_bold().text_color(pct === null ? "muted" : colorHex).tabular_nums()));
    };

    const buildRings = (w, h) => {
      const size = Math.min(26, Math.max(14, (w - 16) / 4));
      return applySkin(row(), "rings", w, h, 12, true).px(8).items_center().justify_between()
        .child(ringGauge("CPU", cpu?.used, 0.04, 0.52, 1.0, "#0a84ff", size))
        .child(ringGauge("GPU", this.gpuPercent, 0.75, 0.35, 0.95, "#bf5af2", size))
        .child(ringGauge("内存", memPct, 1.0, 0.62, 0.08, "#ff9f0a", size))
        .child(ringGauge("存储", this.storagePercent, 0.37, 0.92, 0.83, "#5eead4", size));
    };

    // Bottom 4: Squircle Buttons with Soda Music & Edit Mode Button
    const squircleBtn = (id, child, onClick, size = 28) => row().id(id).size(size).rounded(7)
      .items_center().justify_center().bg("#141518")
      .gradient("#272b35", "#141518")
      .border(1, "#3d4250").shadow(2, 4, "#00000066")
      .cursor_pointer().hover_bg("#22242a")
      .when(!!onClick, (b) => b.on_click(onClick))
      .child(child);

    const toggleEditing = () => {
      this.editing = !this.editing;
      this.widgetDrag = null;
      this.pickerOpen = false;
      if (!this.editing) this.saveAll();
      cx?.notify();
    };
    const editButton = (size = 28) =>
      squircleBtn("btn-edit-mode", icon("layout-grid", 12).color(isEdit ? "#00f57a" : "#8e8e93"), toggleEditing, size);

    const buildLaunchers = (w, h) => {
      const gap = Math.min(5, w * 0.035);
      const size = Math.min(28, (w - 4 * gap) / 5);
      const appSize = Math.min(22, size - 4);
      return row().id("launcher-row").w(w).h(h).items_center().gap(gap)
        .child(squircleBtn("btn-appstore", image("app:com.apple.AppStore").size(appSize).rounded(6).clip(), () => run("settings.open"), size))
        .child(squircleBtn("btn-soda", image("app:com.soda.music").size(appSize).rounded(6).clip(), () => open("/Applications/汽水音乐.app"), size))
        .child(squircleBtn("btn-plus-1", icon("plus", 12).color("#8e8e93"), () => run("settings.open"), size))
        .child(squircleBtn("btn-zap", icon("zap", 14).color("#ff9f0a"), () => run("settings.open"), size))
        .child(editButton(size));
    };

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
      "soda-music": 95, "music": 88, "codex": 130, "clock": 120, "map": 151,
      "battery": 82, "network": 88, "rings": 140, "launchers": 160,
    };
    const rowGap = 6;
    const layoutRows = (widgetIds) => {
      let y = 0;
      return widgetIds.map((ids, index) => {
        const rowType = index === 0 ? "top" : "bottom";
        const gap = index === 0 ? totalGap : 5;
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
      // The lower row can grow when a tall card moves into it.
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
            return div().id(`placeholder/${id}`).w(width).h(height).rounded(14)
              .border(1, "#64d2ff66").bg("#64d2ff12");
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
      // Capture surfaces stay at their original positions while the cards make room for the drop.
      // This keeps pointer capture on the same keyed element even when moving between rows.
      for (const slot of originalRows.flatMap((layout) => layout.slots)) {
        grid.child(div().id(`drag/${slot.id}`).w(slot.width).h(slot.height).pin(slot.x, slot.y, null, null)
          .on_drag((event, context) => handleDrag(slot, event, context)));
      }
      if (drag) {
        const { id, x, y, width, height } = drag.slot;
        grid.child(builders[id](width, height).border(1.5, "#64d2ff").shadow(3, 12, "#000000a6")
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
        .child(icon("layout-grid", 11).color("#00f57a"))
        .child(text("按住卡片拖动，松手保存位置").text_size(8).font_medium().text_color("#00f57a")))
      .child(row().gap(4).items_center()
        .child(text("一键换肤:").text_size(7).text_color("#8e8e93"))
        .child(skinPill("glass", "玻璃", "#0a84ff"))
        .child(skinPill("neon", "霓虹", "#00f57a"))
        .child(skinPill("gold", "黑金", "#e5b869"))
        .child(skinPill("matte", "哑光", "#8e8e93"))
        .child(div().w(4))
        .child(row().px(6).h(16).rounded(4).bg("#ffffff14").items_center().justify_center().cursor_pointer()
          .hover_bg("#ffffff26")
          .on_click(() => this.resetWidgets(cx))
          .child(text("重置").text_size(7).text_color("#8e8e93")))
        .child(row().px(8).h(16).rounded_full().bg("#00f57a").items_center().justify_center().cursor_pointer()
          .hover_bg("#2bf68c")
          .on_click(toggleEditing)
          .child(text("完成").text_size(7.5).font_bold().text_color("#000000"))));

    return col().id("home-dashboard").w_full().gap(6).px(10).py(4)
      .when(isEdit, (b) => b.child(editBar))
      .when(!isEdit && !this.topWidgets.includes("launchers") && !this.bottomWidgets.includes("launchers"), (b) =>
        b.child(row().w_full().justify_end().child(editButton(24))))
      .child(grid);
  }
}
