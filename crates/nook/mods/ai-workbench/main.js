import { View, row, col, div, text, icon } from "nook/ui";
import { agents, codexStatus, exec, island, mapStatus, run } from "nook";

export default class AiWorkbench extends View {
  init(_props, cx) {
    this.cx = cx;
    this.tasks = [];
    this.grantedPermissions = new Set();

    this.refresh = (cx) => {
      const liveAgents = agents() || [];
      const codex = codexStatus();
      const map = mapStatus();

      // Default high-fidelity task presets matching design specification
      const presetTasks = [
        {
          id: "claude-island",
          brand: "Claude Code",
          iconName: "sparkle",
          iconColor: "#e06c45",
          iconBg: "#2a1814",
          title: "重构 Native Island 布局",
          status: "waiting_input",
          statusLabel: "待处理输入",
          statusColor: "#ff9f0a",
          statusBg: "#2b1d0c",
          statusBorder: "#4d3215",
          dotColor: "#ff9f0a",
          file: "openNook/hud/src/island.tsx",
          model: "claude-3-7-sonnet",
          time: "2 分钟前",
          actionType: "jump_review",
          actionLabel: "确认 3 个文件修改",
          actionTime: "刚刚",
        },
        {
          id: "codex-telemetry",
          brand: "Codex",
          iconName: "bot",
          iconColor: "#388bfd",
          iconBg: "#142034",
          title: "Token 与配额遥测接口对接",
          status: "running",
          statusLabel: "运行中",
          statusColor: "#30d158",
          statusBg: "#132a1c",
          statusBorder: "#1e462d",
          dotColor: "#30d158",
          file: "services/telemetry/codex_worker.go",
          model: "gpt-6.2-sol",
          time: "8 分钟前",
          actionType: "progress",
          progressLabel: "● 执行单元测试 [4/7]",
          actionTime: "2 分钟前",
        },
        {
          id: "gemini-hud",
          brand: "Gemini CLI",
          iconName: "gem",
          iconColor: "#7c9bf8",
          iconBg: "#181e38",
          title: "桌面 HUD 视觉比对与暗黑模式适配",
          status: "running",
          statusLabel: "运行中",
          statusColor: "#64d2ff",
          statusBg: "#102636",
          statusBorder: "#1b4260",
          dotColor: "#64d2ff",
          file: "scripts/visual-regression.py",
          model: "gemini-2.5-pro",
          time: "14 分钟前",
          actionType: "progress",
          progressLabel: "分析截图 diff 88%",
          actionTime: "5 分钟前",
        },
        {
          id: "claude-audio",
          brand: "Claude Code",
          iconName: "activity",
          iconColor: "#e06c45",
          iconBg: "#2a1814",
          title: "CoreAudio 低延迟流管道实现",
          status: "waiting_permission",
          statusLabel: "待确认权限",
          statusColor: "#ff9f0a",
          statusBg: "#2b1d0c",
          statusBorder: "#4d3215",
          dotColor: "#ff9f0a",
          file: "native/audio/coreaudio_driver.m",
          model: "claude-3-7-sonnet",
          time: "28 分钟前",
          actionType: "permission",
          actionTime: "12 分钟前",
        },
        {
          id: "codex-bus",
          brand: "Codex",
          iconName: "zap",
          iconColor: "#8e8e93",
          iconBg: "#1e2028",
          title: "全局 ⌥ + 空格 事件总线优化",
          status: "succeeded",
          statusLabel: "已完成",
          statusColor: "#30d158",
          statusBg: "#132a1c",
          statusBorder: "#1e462d",
          dotColor: "#30d158",
          file: "daemon/keybinding_listener.swift",
          model: "gpt-6.2-sol",
          time: "1 小时前",
          actionType: "pr",
          progressLabel: "PR #142 已生成",
          actionTime: "35 分钟前",
        },
        {
          id: "gemini-audit",
          brand: "Gemini CLI",
          iconName: "terminal",
          iconColor: "#8e8e93",
          iconBg: "#1e2028",
          title: "第三方 Cargo 与 NPM 依赖漏洞扫描",
          status: "idle",
          statusLabel: "就绪/闲置",
          statusColor: "#8e8e93",
          statusBg: "#1f2127",
          statusBorder: "#2c2e36",
          dotColor: "#8e8e93",
          file: "ci/security-audit.yml",
          model: "gemini-2.5-flash",
          time: "3 小时前",
          actionType: "idle",
          progressLabel: "0 高危风险 · 待命",
          actionTime: "1 小时前",
        },
      ];

      // Merge real active sessions if present
      const dynamicList = [];
      for (const agent of liveAgents) {
        const isRunning = agent.status === "working";
        dynamicList.push({
          id: `agent-${agent.pid}`,
          pid: agent.pid,
          brand: agent.kind || "Agent",
          iconName: agent.kind === "Claude" ? "sparkle" : agent.kind === "Gemini" ? "gem" : "bot",
          iconColor: agent.kind === "Claude" ? "#e06c45" : agent.kind === "Gemini" ? "#7c9bf8" : "#388bfd",
          iconBg: agent.kind === "Claude" ? "#2a1814" : agent.kind === "Gemini" ? "#181e38" : "#142034",
          title: agent.title || "交互式开发会话",
          status: isRunning ? "running" : "idle",
          statusLabel: isRunning ? "运行中" : "就绪/闲置",
          statusColor: isRunning ? "#64d2ff" : "#8e8e93",
          statusBg: isRunning ? "#102636" : "#1f2127",
          statusBorder: isRunning ? "#1b4260" : "#2c2e36",
          dotColor: isRunning ? "#64d2ff" : "#8e8e93",
          file: agent.project || agent.cwd || "workspace",
          model: agent.model || "default",
          time: "刚刚",
          actionType: isRunning ? "progress" : "idle",
          progressLabel: isRunning ? "● 正在执行指令..." : "等待交互",
          actionTime: "实时",
        });
      }

      this.tasks = dynamicList.length > 0
        ? [...dynamicList, ...presetTasks.slice(dynamicList.length)]
        : presetTasks;

      cx?.notify();
    };

    this.refresh(cx);
    cx.timer.every(2000, this.refresh);
  }

  activity() {
    return {
      width: 48,
      leading: row().gap(4).items_center()
        .child(icon("bot", 13).color("#388bfd"))
        .child(text("AI").text_size(9).font_bold().text_color("#388bfd")),
      detail: text(`${this.tasks.length} 活跃智能体`).text_size(10).text_color("fg"),
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
        // Activate Terminal or system editor
        cx?.spawn(async () => {
          await exec("/usr/bin/osascript", [
            "-e",
            'tell application "Terminal" to activate',
          ]).catch(() => null);
        });
      }
    };

    const handleGrantPermission = (task) => {
      this.grantedPermissions.add(task.id);
      cx?.notify();
    };

    const renderCard = (task) => {
      const isGranted = this.grantedPermissions.has(task.id);
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

      // Right Action / Status Area
      let rightArea = row().gap(6).items_center().flex_none();

      if (task.actionType === "jump_review") {
        rightArea = row().gap(6).items_center().flex_none()
          .child(
            row().gap(3).items_center()
              .child(icon("files", 10).color("#64d2ff"))
              .child(text(task.actionLabel).text_size(8.5).font_medium().text_color("#d1d5db"))
          )
          .child(text(task.actionTime).text_size(7.5).text_color("#6e7681"))
          .child(
            row().h(20).px(8).rounded_full().bg("#0a84ff").items_center().justify_center().cursor_pointer()
              .hover_bg("#2b96ff")
              .on_click(() => handleFocus(task))
              .child(text("跳转 ↗").text_size(8.5).font_bold().text_color("#ffffff"))
          );
      } else if (task.actionType === "permission") {
        rightArea = row().gap(6).items_center().flex_none()
          .child(
            isGranted
              ? row().h(18).px(6).rounded(4).bg("#132a1c").border(1, "#1e462d").items_center().gap(3)
                  .child(icon("check", 9).color("#30d158"))
                  .child(text("已授权").text_size(7.5).font_medium().text_color("#30d158"))
              : row().h(20).px(8).rounded(5).bg("#ff9f0a22").border(1, "#ff9f0a55").items_center().justify_center().cursor_pointer()
                  .hover_bg("#ff9f0a44")
                  .on_click(() => handleGrantPermission(task))
                  .child(icon("shield-check", 10).color("#ff9f0a"))
                  .child(div().w(3))
                  .child(text("授予终端权限").text_size(8).font_bold().text_color("#ff9f0a"))
          )
          .child(text(task.actionTime).text_size(7.5).text_color("#6e7681"))
          .child(icon("chevron-right", 10).color("#6e7681"));
      } else {
        rightArea = row().gap(6).items_center().flex_none()
          .child(text(task.progressLabel).text_size(8.5).font_medium().text_color("#d1d5db"))
          .child(text(task.actionTime).text_size(7.5).text_color("#6e7681"))
          .child(icon("chevron-right", 10).color("#6e7681"));
      }

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

    return col().id("ai-workbench-page").w(width).gap(6).px(4).py(6)
      .children(this.tasks.map(renderCard));
  }
}
