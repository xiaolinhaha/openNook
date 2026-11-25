import { View, row, col, div, text, icon } from "nook/ui";
import { agents, codexStatus, exec, home, island, mapStatus, run } from "nook";

const MAP_BUNDLE = "com.xiaolinhaha.map";

function formatRelativeTime(epochMs) {
  if (!epochMs || !Number.isFinite(epochMs)) return "刚刚";
  const diffSec = Math.max(0, Math.floor((Date.now() - epochMs) / 1000));
  if (diffSec < 45) return "刚刚";
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)} 分钟前`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} 小时前`;
  const days = Math.floor(diffSec / 86400);
  return days === 1 ? "昨天" : `${days} 天前`;
}

function shortenPath(p) {
  if (!p) return "~";
  const userHome = home();
  if (userHome && p.startsWith(userHome)) {
    p = "~" + p.slice(userHome.length);
  }
  const parts = p.split("/").filter(Boolean);
  if (parts.length > 2) {
    return parts.slice(-2).join("/");
  }
  return p;
}

function formatModelName(model) {
  if (!model) return "AI";
  return model
    .replace(/^antigravity:/i, "")
    .replace(/^codex:/i, "")
    .replace(/^grok:/i, "")
    .replace(/^gpt-/i, "GPT-")
    .replace(/^gemini-/i, "Gemini-")
    .replace(/(^|[-_])([a-z])/g, (_, sep, letter) => `${sep === "-" || sep === "_" ? " " : ""}${letter.toUpperCase()}`);
}

export default class AiWorkbench extends View {
  init(_props, cx) {
    this.cx = cx;
    this.tasks = [];

    this.refresh = (cx) => {
      const liveAgents = agents() || [];
      const map = mapStatus();
      const mapSessions = map?.sessions || [];

      const runningList = [];

      // 1. Standalone Codex / CLI running sessions
      for (const agent of liveAgents) {
        if (agent.status === "working") {
          const isCodex = String(agent.kind ?? "").toLowerCase() === "codex";
          const brand = isCodex ? "Codex" : (agent.kind || "Agent");

          runningList.push({
            id: `agent-${agent.pid}`,
            pid: agent.pid,
            source: "codex_cli",
            brand,
            iconName: isCodex ? "bot" : "sparkle",
            iconColor: isCodex ? "#388bfd" : "#64d2ff",
            iconBg: isCodex ? "#142034" : "#102636",
            title: agent.title || "终端交互会话",
            file: shortenPath(agent.cwd || "~"),
            model: formatModelName(agent.model || "gpt-6.1-sol"),
            time: "实时",
            status: "running",
            statusLabel: "运行中",
            statusColor: "#30d158",
            statusBg: "#132a1c",
            statusBorder: "#1e462d",
            dotColor: "#30d158",
            actionText: "● 执行中",
          });
        }
      }

      // 2. Real MAP Multi-AI running sessions (Exclude completed/interrupted)
      for (const s of mapSessions) {
        if (s.status === "running") {
          let brand = "MAP";
          let iconName = "bot";
          let iconColor = "#388bfd";
          let iconBg = "#142034";

          if (s.harness === "antigravity") {
            brand = "MAP · Gemini";
            iconName = "sparkle";
            iconColor = "#7c9bf8";
            iconBg = "#181e38";
          } else if (s.harness === "codex") {
            brand = "MAP · Codex";
            iconName = "bot";
            iconColor = "#388bfd";
            iconBg = "#142034";
          } else if (s.harness === "grok") {
            brand = "MAP · Grok";
            iconName = "zap";
            iconColor = "#ff9f0a";
            iconBg = "#2b1d0c";
          }

          const cleanTitle = (s.title || "未命名任务").replace(/^(antigravity|codex|grok)\s*·\s*/i, "").trim();

          runningList.push({
            id: `map-${s.id}`,
            sessionId: s.id,
            source: "map",
            brand,
            iconName,
            iconColor,
            iconBg,
            title: cleanTitle,
            file: shortenPath(s.cwd),
            model: formatModelName(s.model),
            time: formatRelativeTime(s.updated_at || s.at),
            status: "running",
            statusLabel: "运行中",
            statusColor: "#64d2ff",
            statusBg: "#102636",
            statusBorder: "#1b4260",
            dotColor: "#64d2ff",
            actionText: "● 执行中",
          });
        }
      }

      // 3. If NO running tasks, show available AIs (MAP 3 kinds + Codex GPT)
      if (runningList.length === 0) {
        const availableAis = [
          {
            id: "available-map-gemini",
            source: "map",
            brand: "MAP · Gemini",
            iconName: "sparkle",
            iconColor: "#7c9bf8",
            iconBg: "#181e38",
            title: "多模型智能调度池",
            file: "MAP 会话池",
            model: "Gemini 3.8 Flash High",
            time: "随时待命",
            status: "idle",
            statusLabel: "就绪",
            statusColor: "#8e8e93",
            statusBg: "#181a20",
            statusBorder: "#272932",
            dotColor: "#8e8e93",
            actionText: "待命",
          },
          {
            id: "available-map-codex",
            source: "map",
            brand: "MAP · Codex",
            iconName: "bot",
            iconColor: "#388bfd",
            iconBg: "#142034",
            title: "多模型智能调度池",
            file: "MAP 会话池",
            model: "GPT-6.1 Sol",
            time: "随时待命",
            status: "idle",
            statusLabel: "就绪",
            statusColor: "#8e8e93",
            statusBg: "#181a20",
            statusBorder: "#272932",
            dotColor: "#8e8e93",
            actionText: "待命",
          },
          {
            id: "available-map-grok",
            source: "map",
            brand: "MAP · Grok",
            iconName: "zap",
            iconColor: "#ff9f0a",
            iconBg: "#2b1d0c",
            title: "多模型智能调度池",
            file: "MAP 会话池",
            model: "Grok 4.6",
            time: "随时待命",
            status: "idle",
            statusLabel: "就绪",
            statusColor: "#8e8e93",
            statusBg: "#181a20",
            statusBorder: "#272932",
            dotColor: "#8e8e93",
            actionText: "待命",
          },
          {
            id: "available-codex",
            source: "codex_cli",
            brand: "Codex",
            iconName: "bot",
            iconColor: "#388bfd",
            iconBg: "#142034",
            title: "终端交互式会话",
            file: "~",
            model: "GPT-6.1 Sol",
            time: "随时待命",
            status: "idle",
            statusLabel: "就绪",
            statusColor: "#8e8e93",
            statusBg: "#181a20",
            statusBorder: "#272932",
            dotColor: "#8e8e93",
            actionText: "待命",
          },
        ];
        this.tasks = availableAis;
      } else {
        this.tasks = runningList;
      }

      cx?.notify();
    };

    this.refresh(cx);
    cx.timer.every(2000, this.refresh);
  }

  activity() {
    const runningTasks = this.tasks.filter((t) => t.status === "running");
    const count = runningTasks.length;
    return {
      width: 48,
      leading: row().gap(4).items_center()
        .child(icon("bot", 13).color("#388bfd"))
        .child(text("AI").text_size(9).font_bold().text_color("#388bfd")),
      detail: text(count > 0 ? `${count} 个任务运行中` : "就绪待命").text_size(10).text_color("fg"),
      priority: 8,
    };
  }

  expanded(cx) {
    if (cx) this.cx = cx;
    else cx = this.cx;

    const state = island();
    const width = (state?.page?.width ?? 560) - 24;

    const handleFocus = (task) => {
      if (task.pid) {
        run("agents.focus", { pid: task.pid });
      } else if (task.source === "codex_cli") {
        cx?.spawn(async () => {
          await exec("/usr/bin/osascript", [
            "-e",
            'tell application "Terminal" to activate',
          ]).catch(() => null);
        });
      } else {
        cx?.spawn(async () => {
          await exec("/usr/bin/open", ["-b", MAP_BUNDLE]).catch(() => null);
        });
      }
    };

    const renderCard = (task) => {
      const isRunning = task.status === "running";

      // Left Brand Icon
      const brandIcon = row().size(32).rounded(8).bg(task.iconBg)
        .items_center().justify_center().flex_none()
        .child(icon(task.iconName, 15).color(task.iconColor));

      // Status Pill
      const statusPill = row().h(16).px(5).gap(3).items_center().rounded(4).flex_none()
        .bg(task.statusBg).border(1, task.statusBorder)
        .child(div().size(3.5).rounded_full().bg(task.statusColor).when(isRunning, (d) => d.pulse(1.1)))
        .child(text(task.statusLabel).text_size(7.5).font_medium().text_color(task.statusColor));

      // Top Line: Brand · Title + Status Pill
      const topLine = row().w_full().gap(6).items_center()
        .child(text(`${task.brand} · ${task.title}`).text_size(10.5).font_bold().text_color("#ffffff").truncate())
        .child(statusPill);

      // Sub Line: Dot + File · Model · Time
      const subLine = row().w_full().gap(4).items_center()
        .child(div().size(3).rounded_full().bg(task.dotColor).flex_none())
        .child(text(task.file).text_size(8.5).text_color("#8e8e93").truncate())
        .child(text("·").text_size(7).text_color("#4b505c").flex_none())
        .child(text(task.model).text_size(8.5).text_color("#8e8e93").flex_none())
        .child(text("·").text_size(7).text_color("#4b505c").flex_none())
        .child(text(task.time).text_size(8).text_color("#6e7681").flex_none());

      const infoCol = col().flex_1().min_w(0).gap(3).justify_center()
        .child(topLine)
        .child(subLine);

      // Right Action Area
      const rightArea = row().gap(8).items_center().flex_none().pl(10)
        .child(
          text(task.actionText || (isRunning ? "● 执行中" : "待命"))
            .text_size(8.5).font_medium()
            .text_color(isRunning ? "#64d2ff" : "#8e8e93")
        )
        .child(
          row().h(22).px(9).rounded(6)
            .bg(isRunning ? "#0a84ff" : "#181920")
            .border(1, isRunning ? "#2b96ff" : "#262832")
            .items_center().justify_center().cursor_pointer()
            .hover_bg(isRunning ? "#2b96ff" : "#22242e")
            .on_click(() => handleFocus(task))
            .child(
              text("跳转 ↗").text_size(8.5).font_bold()
                .text_color(isRunning ? "#ffffff" : "#c5c8d2")
            )
        );

      return row().id(`task-card/${task.id}`).w_full().h(52).px(12).py(7).rounded(10).clip()
        .bg("#0d0e12")
        .border(1, "#1c1e26")
        .hover_bg("#14161d")
        .cursor_pointer()
        .items_center().justify_between()
        .on_click(() => handleFocus(task))
        .child(
          row().flex_1().min_w(0).gap(10).items_center()
            .child(brandIcon)
            .child(infoCol)
        )
        .child(rightArea);
    };

    return col().id("ai-workbench-page").w(width).gap(6).px(4).py(6)
      .children(this.tasks.map(renderCard));
  }
}
