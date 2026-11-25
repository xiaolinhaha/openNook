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

      const list = [];

      // 1. Standalone Codex / CLI Agents
      for (const agent of liveAgents) {
        const isWorking = agent.status === "working";
        const isCodex = String(agent.kind ?? "").toLowerCase() === "codex";
        const brand = isCodex ? "Codex" : (agent.kind || "Agent");

        list.push({
          id: `agent-${agent.pid}`,
          pid: agent.pid,
          source: "codex_cli",
          brand,
          iconName: isCodex ? "bot" : "sparkle",
          iconColor: isCodex ? "#388bfd" : "#64d2ff",
          iconBg: isCodex ? "#142034" : "#102636",
          title: agent.title || "终端交互式会话",
          file: shortenPath(agent.cwd || "~"),
          model: formatModelName(agent.model || "gpt-6.1-sol"),
          time: "实时",
          status: isWorking ? "running" : "idle",
          statusLabel: isWorking ? "运行中" : "待命",
          statusColor: isWorking ? "#30d158" : "#8e8e93",
          statusBg: isWorking ? "#132a1c" : "#1f2127",
          statusBorder: isWorking ? "#1e462d" : "#2c2e36",
          dotColor: isWorking ? "#30d158" : "#8e8e93",
          actionLabel: isWorking ? "● 执行中" : "就绪",
        });
      }

      // 2. Real MAP Multi-AI Sessions
      for (const s of mapSessions) {
        const isRunning = s.status === "running";
        const isCompleted = s.status === "completed" || s.status === "succeeded";
        const isInterrupted = s.status === "interrupted";
        const isFailed = s.status === "failed";

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

        let statusLabel = "待命";
        let statusColor = "#8e8e93";
        let statusBg = "#1f2127";
        let statusBorder = "#2c2e36";
        let actionLabel = "就绪";

        if (isRunning) {
          statusLabel = "运行中";
          statusColor = "#64d2ff";
          statusBg = "#102636";
          statusBorder = "#1b4260";
          actionLabel = "● 正在执行中...";
        } else if (isCompleted) {
          statusLabel = "已完成";
          statusColor = "#30d158";
          statusBg = "#132a1c";
          statusBorder = "#1e462d";
          actionLabel = "✓ 本轮已完成";
        } else if (isInterrupted) {
          statusLabel = "已中断";
          statusColor = "#ff9f0a";
          statusBg = "#2b1d0c";
          statusBorder = "#4d3215";
          actionLabel = "已停止";
        } else if (isFailed) {
          statusLabel = "失败";
          statusColor = "#ff453a";
          statusBg = "#2a1515";
          statusBorder = "#4a1e1e";
          actionLabel = "执行失败";
        }

        list.push({
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
          status: s.status,
          statusLabel,
          statusColor,
          statusBg,
          statusBorder,
          dotColor: statusColor,
          actionLabel,
        });
      }

      this.tasks = list;
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
      detail: text(count > 0 ? `${count} 个任务运行中` : `${this.tasks.length} 活跃智能体`).text_size(10).text_color("fg"),
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
      } else {
        // Activate MAP app
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

      // Top Line: Brand · Title + Status Pill
      const statusPill = row().h(16).px(5).gap(3).items_center().rounded(4).flex_none()
        .bg(task.statusBg).border(1, task.statusBorder)
        .child(div().size(3.5).rounded_full().bg(task.statusColor).when(isRunning, (d) => d.pulse(1.1)))
        .child(text(task.statusLabel).text_size(7.5).font_medium().text_color(task.statusColor));

      const topLine = row().w_full().gap(6).items_center()
        .child(text(`${task.brand} · ${task.title}`).text_size(10).font_bold().text_color("#ffffff").truncate())
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
      const rightArea = row().gap(6).items_center().flex_none()
        .child(text(task.actionLabel).text_size(8.5).font_medium().text_color(isRunning ? "#64d2ff" : "#8e8e93"))
        .child(
          row().h(20).px(8).rounded_full().bg(isRunning ? "#0a84ff" : "#1f222b")
            .border(1, isRunning ? "#2b96ff" : "#2d303b")
            .items_center().justify_center().cursor_pointer()
            .hover_bg(isRunning ? "#2b96ff" : "#282c38")
            .on_click(() => handleFocus(task))
            .child(text("跳转 ↗").text_size(8.5).font_bold().text_color(isRunning ? "#ffffff" : "#c5c8d2"))
        );

      return row().id(`task-card/${task.id}`).w_full().h(52).px(10).py(6).rounded(10).clip()
        .bg("#14151a")
        .gradient("#16171d", "#121317")
        .border(1, "#23252d")
        .hover_bg("#191b22")
        .cursor_pointer()
        .items_center().justify_between()
        .on_click(() => handleFocus(task))
        .child(
          row().flex_1().min_w(0).gap(9).items_center()
            .child(brandIcon)
            .child(infoCol)
        )
        .child(rightArea);
    };

    if (this.tasks.length === 0) {
      return col().id("ai-workbench-page").w(width).gap(6).px(4).py(10)
        .child(
          row().w_full().h(60).rounded(10).bg("#14151a").border(1, "#23252d")
            .items_center().justify_center().gap(6)
            .child(icon("bot", 16).color("#8e8e93"))
            .child(text("暂无进行中的 AI 会话 · 请在 MAP 或 Codex 中开启新对话").text_size(9.5).text_color("#8e8e93"))
        );
    }

    return col().id("ai-workbench-page").w(width).gap(6).px(4).py(6)
      .children(this.tasks.map(renderCard));
  }
}
