import { View, row, col, div, text, icon } from "nook/ui";
import { agents, codexStatus, exec, home, island, modDir, run } from "nook";

const isCodex = (session) => String(session.kind ?? "").toLowerCase() === "codex";

const PHASES = {
  offline: ["未运行", "muted", "minus"],
  idle: ["空闲", "muted", "minus"],
  running: ["运行中", "blue", "activity"],
  succeeded: ["已完成", "green", "check"],
  failed: ["失败", "red", "triangle-alert"],
  interrupted: ["已中断", "orange", "pause"],
};

function displayModel(slug) {
  if (!slug) return "Codex";
  return slug.replace(/^gpt-/i, "GPT-").replace(/(^|[-_])([a-z])/g, (_, sep, letter) => `${sep === "-" || sep === "_" ? " " : ""}${letter.toUpperCase()}`);
}

function relativeReset(epoch) {
  if (!Number.isFinite(epoch)) return "重置时间未知";
  const seconds = Math.max(0, Math.floor(epoch - Date.now() / 1000));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours) return `${hours} 小时 ${minutes} 分钟后重置`;
  if (minutes) return `${minutes} 分钟后重置`;
  return "即将重置";
}

function remainingWindow(snapshot, minutes) {
  const windows = [snapshot?.primary, snapshot?.secondary].filter(Boolean);
  const window = windows.find((item) => Number(item.windowDurationMins) === minutes);
  if (!window || !Number.isFinite(Number(window.usedPercent))) return null;
  return {
    remaining: Math.max(0, Math.min(100, 100 - Number(window.usedPercent))),
    resetsAt: Number(window.resetsAt),
  };
}

function quotaTile(id, title, quota, color, tileWidth) {
  const fillWidth = quota ? tileWidth * quota.remaining / 100 : 0;
  return col().id(id).flex_1().gap(7).p(10).rounded(12).bg("surface")
    .child(row().w_full().items_center().justify_between()
      .child(text(title).text_size(12).font_semibold().text_color("muted"))
      .child(text(quota ? `${Math.round(quota.remaining)}%` : "--").text_size(18).font_semibold()
        .text_color(quota ? color : "faint").tabular_nums()))
    .child(row().id(`${id}-track`).w_full().h(6).rounded_full().bg("raised")
      .when(quota, (track) => track.child(div().id(`${id}-fill`).h(6).w(fillWidth).rounded_full().bg(color))))
    .child(text(quota ? relativeReset(quota.resetsAt) : "额度暂不可用").text_size(10).text_color("faint").truncate());
}

export default class Codex extends View {
  init(props, cx) {
    this.quota = null;
    this.quotaError = false;
    this.quotaBusy = false;
    this.model = null;
    this.lastQuotaAt = null;

    const refreshQuota = (cx) => cx.spawn(async (cx) => {
      if (this.quotaBusy) return;
      this.quotaBusy = true;
      try {
        const script = `${modDir()}/quota.sh`;
        const result = await exec("sh", [script]);
        if (result.code !== 0) throw new Error("Codex usage read failed");
        const responseLine = result.stdout.split(/\r?\n/).find((line) => {
          try { return JSON.parse(line).id === 2; } catch { return false; }
        });
        if (!responseLine) throw new Error("Codex usage response missing");
        const response = JSON.parse(responseLine);
        if (response.error || !response.result) throw new Error("Codex usage unavailable");
        const rates = response.result.rateLimitsByLimitId?.codex ?? response.result.rateLimits;
        if (!rates) throw new Error("Codex usage unavailable");
        this.quota = rates;
        this.quotaError = false;
        this.lastQuotaAt = Date.now();
      } catch {
        this.quotaError = true;
      } finally {
        this.quotaBusy = false;
        cx.notify();
      }
    });

    const readModel = (cx) => cx.spawn(async (cx) => {
      const config = await exec("grep", ["-m", "1", "^model[[:space:]]*=", `${home()}/.codex/config.toml`]).catch(() => null);
      const slug = config?.stdout?.match(/=\s*["']([^"']+)["']/)?.[1];
      if (slug) {
        this.model = slug;
        cx.notify();
      }
    });

    this.refreshQuota = () => refreshQuota(cx);
    refreshQuota(cx);
    readModel(cx);
    cx.timer.every(300_000, refreshQuota);
  }

  sessions() {
    return agents().filter(isCodex);
  }

  modelName(sessions) {
    return displayModel(sessions.find((s) => s.model)?.model ?? this.model);
  }

  activity() {
    const sessions = this.sessions();
    const phase = codexStatus()?.status ?? "offline";
    const [label, color, glyph] = PHASES[phase];
    const quota = remainingWindow(this.quota, 300);
    const tail = quota ? `5h 剩余 ${Math.round(quota.remaining)}%` : this.quotaBusy ? "额度同步中" : "额度 --";
    return {
      leading: icon("agent-codex", 16),
      width: 80,
      trailing: row().id("codex-activity").gap(5)
        .child(icon(glyph, 12).color(color).when(phase === "running", (i) => i.pulse(1.1)))
        .child(text(label).text_size(11).font_medium().text_color(color)),
      detail: text(`${this.modelName(sessions)} · ${label} · ${tail}`).text_size(11).text_color("muted").truncate(),
      priority: 100,
    };
  }

  expanded() {
    const sessions = this.sessions();
    const phase = codexStatus()?.detail ?? "offline";
    const [label, color, glyph] = PHASES[phase];
    const width = island()?.page?.width ?? 528;
    const tileWidth = Math.max(0, (width - 74) / 2);
    const fiveHour = remainingWindow(this.quota, 300);
    const weekly = remainingWindow(this.quota, 10080);
    const model = this.modelName(sessions);
    const panel = col().id("codex-panel").w_full().gap(9).px(12).py(10)
      .child(row().id("codex-heading").w_full().gap(7).items_center()
        .child(icon("agent-codex", 17))
        .child(text("Codex").text_size(14).font_semibold())
        .child(div().flex_1())
        .child(icon(glyph, 12).color(color).when(phase === "running", (i) => i.pulse(1.1)))
        .child(text(label).text_size(11).font_medium().text_color(color)))
      .child(row().id("codex-model-row").w_full().gap(7).items_center()
        .child(text(`模型：${model}`).text_size(11).text_color("muted").truncate())
        .child(div().flex_1())
        .child(text("↻ 刷新额度").text_size(10).text_color("muted")
          .cursor_pointer()
          .on_click(() => this.refreshQuota?.())))
      .child(row().id("codex-quota-tiles").w_full().gap(9)
        .child(quotaTile("quota-5h", "5 小时", fiveHour, fiveHour?.remaining <= 15 ? "orange" : "green", tileWidth))
        .child(quotaTile("quota-weekly", "每周", weekly, weekly?.remaining <= 15 ? "orange" : "accent", tileWidth)));

    if (sessions.length) {
      panel.child(col().id("codex-session-list").w_full().gap(5)
        .children(sessions.slice(0, 3).map((session) => row().id(`codex-session-${session.pid}`).w_full().gap(8).px(10).py(8).rounded(10)
          .hover_bg("surface")
          .on_click(() => run("agents.focus", { pid: session.pid }))
          .child(icon("agent-codex", 14).color("muted"))
          .child(text(session.title || session.project || "Codex 会话").flex_1().text_size(11).font_medium().truncate())
          .child(text(session.status === "working" ? "运行中 · 打开" : "打开会话").text_size(10).text_color(session.status === "working" ? "accent" : "muted")))));
    }
    if (this.quotaError) {
      panel.child(text("额度数据暂不可用，点击“刷新额度”重试").px(2).text_size(10).text_color("faint"));
    }
    if (this.lastQuotaAt !== null) {
      panel.child(text(`更新于 ${new Date(this.lastQuotaAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}`).px(2).text_size(9).text_color("faint"));
    }
    return panel;
  }
}
