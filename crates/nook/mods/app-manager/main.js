import { View, InputState, row, col, div, text, icon, image, input } from "nook/ui";
import { exec, home, island, modDir, open, run } from "nook";

export default class AppManager extends View {
  init(_props, cx) {
    this.cx = cx;
    this.apps = [];
    this.activeFilter = "all"; // "all" | "accessory" | "regular" | "background"
    this.query = "";
    this.killingPid = null;
    this.notice = "";

    this.search = new InputState({ placeholder: "搜索应用名或 PID..." });
    this.search.on("change", ({ value }, cx) => {
      this.query = (value ?? "").trim().toLowerCase();
      cx?.notify();
    });

    this.refresh = (cx) => {
      cx?.spawn(async (cx) => {
        try {
          const probePath = `${modDir()}/probe`;
          const result = await exec(probePath);
          if (result.code === 0 && result.stdout) {
            this.apps = JSON.parse(result.stdout);
          }
        } catch (err) {
          // If probe binary fails, fallback to simple ps inspection
          try {
            const fallback = await exec("/bin/ps", ["-u", home().split("/").pop(), "-o", "pid,rss,comm"]);
            if (fallback.code === 0) {
              const lines = fallback.stdout.trim().split("\n").slice(1);
              const list = [];
              for (const l of lines) {
                const parts = l.trim().split(/\s+/, 3);
                if (parts.length === 3 && parts[2].includes(".app/")) {
                  const comm = parts[2];
                  const name = comm.split(".app")[0].split("/").pop();
                  const pid = parseInt(parts[0], 10);
                  list.push({
                    pid,
                    pids: [pid],
                    name: decodeURIComponent(name),
                    bundleId: "",
                    path: comm.split(".app")[0] + ".app",
                    policy: "accessory",
                    memoryMB: Math.round(parseInt(parts[1], 10) / 102.4) / 10,
                    processes: 1,
                  });
                }
              }
              this.apps = list;
            }
          } catch (e) {
            console.error("AppManager refresh error:", e);
          }
        }
        cx?.notify();
      });
    };

    this.refresh(cx);
    cx.timer.every(2500, this.refresh);
  }

  killApp(app, cx) {
    const pid = app.pid;
    const pids = (app.pids && app.pids.length ? app.pids : [pid]).map((n) => Number(n));
    this.killingPid = pid;
    this.notice = `正在终止 ${app.name} (${pids.length} 个进程)...`;
    cx?.notify();

    cx?.spawn(async (cx) => {
      const probePath = `${modDir()}/probe`;
      // Main process first, then the helpers it spawned (node workers, renderers).
      const ordered = [pid, ...pids.filter((p) => p !== pid)];
      try {
        for (const p of ordered) {
          const result = await exec(probePath, ["--kill", String(p)]);
          if (result.code !== 0) await exec("/bin/kill", ["-9", String(p)]);
        }
        this.apps = this.apps.filter((a) => a.pid !== pid);
        this.notice = `已强制退出 ${app.name}`;
      } catch (e) {
        for (const p of ordered) {
          try { await exec("/bin/kill", ["-9", String(p)]); } catch (_) {}
        }
        this.apps = this.apps.filter((a) => a.pid !== pid);
        this.notice = `已终止 ${app.name}`;
      } finally {
        this.killingPid = null;
        cx?.notify();
        cx?.timer?.after(3000, () => {
          this.notice = "";
          cx?.notify();
        });
      }
    });
  }

  revealInFinder(path, cx) {
    if (!path) return;
    cx?.spawn(async () => {
      await exec("/usr/bin/open", ["-R", path]);
    });
  }

  activity() {
    const total = this.apps.length;
    const accessory = this.apps.filter((a) => a.policy === "accessory").length;
    return {
      width: 48,
      leading: row().gap(3).items_center()
        .child(icon("list", 13).color("#0a84ff"))
        .child(text(String(total)).text_size(10).font_bold().text_color("#0a84ff")),
      detail: text(`${total} 运行 · ${accessory} 托盘`).text_size(10).text_color("fg"),
      priority: 6,
    };
  }

  expanded(cx) {
    if (cx) this.cx = cx;
    else cx = this.cx;

    const state = island();
    const width = (state?.page?.width ?? 580) - 16;

    // Filter apps
    let filtered = this.apps;
    if (this.activeFilter === "accessory") {
      filtered = filtered.filter((a) => a.policy === "accessory");
    } else if (this.activeFilter === "regular") {
      filtered = filtered.filter((a) => a.policy === "regular");
    } else if (this.activeFilter === "background") {
      filtered = filtered.filter((a) => a.policy === "background");
    }

    if (this.query) {
      filtered = filtered.filter((a) =>
        a.name.toLowerCase().includes(this.query) ||
        String(a.pid).includes(this.query) ||
        (a.bundleId && a.bundleId.toLowerCase().includes(this.query))
      );
    }

    const totalCount = this.apps.length;
    const accessoryCount = this.apps.filter((a) => a.policy === "accessory").length;
    const regularCount = this.apps.filter((a) => a.policy === "regular").length;
    const totalMemory = (this.apps.reduce((sum, a) => sum + (a.memoryMB || 0), 0) / 1024).toFixed(1);

    // 1. Navigation / Filter tabs and search bar
    const filterBtn = (key, label, count, color, badge) => {
      const active = this.activeFilter === key;
      let btn = row()
        .gap(4)
        .px(8)
        .py(3)
        .rounded(6)
        .items_center()
        .cursor_pointer()
        .bg(active ? "#ffffff1f" : "#ffffff0a")
        .hover_bg("#ffffff1a")
        .on_click((_, cx) => {
          this.activeFilter = key;
          cx.notify();
        });

      if (color) {
        btn = btn.child(div().size(6).rounded_full().bg(color));
      }
      btn = btn.child(
        text(label).text_size(11).font_medium().text_color(active ? "#f5f5f7" : "#a1a1a6")
      );
      if (count !== undefined) {
        btn = btn.child(
          text(String(count)).text_size(10).font_mono().text_color(active ? "#ffffff" : "#6e6e73")
        );
      }
      if (badge) {
        btn = btn.child(
          row()
            .px(4)
            .py(1)
            .rounded(4)
            .bg("#bf5af226")
            .child(text(badge).text_size(9).text_color("#bf5af2"))
        );
      }
      return btn;
    };

    const searchBox = row()
      .w(180)
      .h(26)
      .px(8)
      .gap(6)
      .items_center()
      .rounded(6)
      .bg("#ffffff10")
      .border(0.5, "#ffffff1a")
      .child(icon("search", 12).color("#8e8e93"))
      .child(input(this.search).flex_1().h(18).text_size(11).text_color("#f5f5f7").placeholder_color("#8e8e93"));

    const refreshBtn = row()
      .size(26)
      .rounded(6)
      .items_center()
      .justify_center()
      .bg("#ffffff0d")
      .hover_bg("#ffffff1a")
      .cursor_pointer()
      .on_click((_, cx) => this.refresh(cx))
      .child(icon("rotate-ccw", 12).color("#a1a1a6"));

    const topFilterRow = row()
      .w_full()
      .items_center()
      .justify_between()
      .pb(8)
      .child(
        row()
          .gap(6)
          .items_center()
          .child(filterBtn("all", "全部运行中", totalCount))
          .child(filterBtn("accessory", "菜单栏 / 托盘", accessoryCount, "#bf5af2", "原生不可见"))
          .child(filterBtn("regular", "窗口应用", regularCount, "#0a84ff"))
      )
      .child(
        row()
          .gap(6)
          .items_center()
          .child(searchBox)
          .child(refreshBtn)
      );

    // 2. Summary & telemetry strip
    const summaryRow = row()
      .w_full()
      .items_center()
      .justify_between()
      .py(6)
      .child(
        row()
          .gap(6)
          .items_center()
          .child(div().size(6).rounded_full().bg("#30d158"))
          .child(text(`共 ${filtered.length} 个应用显示中`).text_size(11).text_color("#a1a1a6"))
          .child(text("· 已排除系统底层守护进程").text_size(10).text_color("#6e6e73"))
      )
      .child(
        row()
          .gap(8)
          .items_center()
          .child(
            text(this.notice ? this.notice : `总占用内存 ${totalMemory} GB`)
              .text_size(10)
              .font_mono()
              .text_color(this.notice ? "#30d158" : "#8e8e93")
          )
      );

    // 3. App list rows
    const displayedApps = filtered;

    const appRows = displayedApps.map((app) => {
      const isAccessory = app.policy === "accessory";
      const isKilling = this.killingPid === app.pid;

      const policyBadge = isAccessory
        ? row().px(5).py(1).rounded(4).bg("#bf5af21f").border(0.5, "#bf5af240")
            .child(text("菜单栏 / 托盘").text_size(9).text_color("#bf5af2").font_medium())
        : row().px(5).py(1).rounded(4).bg("#0a84ff18").border(0.5, "#0a84ff35")
            .child(text("窗口应用").text_size(9).text_color("#64d2ff").font_medium());

      const killButton = row()
        .px(8)
        .py(3)
        .rounded(5)
        .bg("#ff453a1a")
        .hover_bg("#ff453a35")
        .border(0.5, "#ff453a40")
        .cursor_pointer()
        .items_center()
        .gap(3)
        .on_click((_, cx) => this.killApp(app, cx))
        .child(icon("x", 10).color("#ff453a"))
        .child(
          text(isKilling ? "正在退出…" : "强制退出")
            .text_size(10)
            .font_semibold()
            .text_color("#ff453a")
        );

      const revealBtn = row()
        .px(6)
        .py(3)
        .rounded(5)
        .bg("#ffffff0d")
        .hover_bg("#ffffff1f")
        .cursor_pointer()
        .items_center()
        .on_click((_, cx) => this.revealInFinder(app.path, cx))
        .child(text("定位").text_size(10).text_color("#a1a1a6"));

      return row()
        .w_full()
        .h(42)
        .px(8)
        .rounded(8)
        .items_center()
        .justify_between()
        .bg("#ffffff08")
        .hover_bg("#ffffff14")
        .border(0.5, "#ffffff0f")
        .child(
          // Left: Icon + Name + PID + BundleId
          row()
            .gap(8)
            .items_center()
            .flex_1()
            .child(
              app.path
                ? image(`app-file:${app.path}`).size(22)
                : icon("layout-grid", 16).color("#0a84ff")
            )
            .child(
              col()
                .gap(1)
                .child(
                  row()
                    .gap(6)
                    .items_center()
                    .child(
                      text(app.name)
                        .text_size(12)
                        .font_semibold()
                        .text_color("#f5f5f7")
                        .truncate()
                    )
                    .child(
                      row()
                        .px(4)
                        .py(0.5)
                        .rounded(3)
                        .bg("#ffffff14")
                        .child(text(app.processes > 1 ? `PID ${app.pid} · ${app.processes} 进程` : `PID ${app.pid}`).text_size(9).font_mono().text_color("#8e8e93"))
                    )
                    .child(policyBadge)
                )
                .child(
                  text(app.bundleId || app.path || "无 Bundle 标识")
                    .text_size(9)
                    .font_mono()
                    .text_color("#636366")
                    .truncate()
                )
            )
        )
        .child(
          // Right: Memory + Action buttons
          row()
            .gap(10)
            .items_center()
            .child(
              row()
                .w(70)
                .justify_end()
                .child(
                  text(`${app.memoryMB} MB`)
                    .text_size(11)
                    .font_mono()
                    .font_medium()
                    .text_color("#d1d1d6")
                )
            )
            .child(
              row()
                .gap(4)
                .items_center()
                .child(revealBtn)
                .child(killButton)
            )
        );
    });

    const emptyRow = row()
      .w_full()
      .h(100)
      .items_center()
      .justify_center()
      .child(
        col()
          .items_center()
          .gap(6)
          .child(icon("search", 20).color("#6e6e73"))
          .child(text("未找到匹配的应用").text_size(12).text_color("#8e8e93"))
      );

    // Keep the page inside the 460pt island canvas, or the rounded bottom is clipped
    // and a downward swipe misses the scroller and closes the island.
    const listHeight = filtered.length === 0 ? 100 : Math.min(220, filtered.length * 45);

    const appList = col()
      .id("app-list-scroll")
      .w_full()
      .h(listHeight)
      .max_h(listHeight)
      .overflow_y_scroll()
      .gap(3)
      .pr(2)
      .children(filtered.length === 0 ? [emptyRow] : appRows);

    // 4. Footer hints
    const footer = row()
      .w_full()
      .pt(8)
      .items_center()
      .justify_between()
      .child(
        row()
          .gap(4)
          .items_center()
          .child(icon("info", 11).color("#8e8e93"))
          .child(
            text("点击【强制退出】可向进程发送 SIGKILL 信号直接终止；【定位】可在访达中打开。")
              .text_size(10)
              .text_color("#6e6e73")
          )
      )
      .child(
        text(`共 ${filtered.length} 个应用`)
          .text_size(10)
          .font_mono()
          .text_color("#6e6e73")
      );

    return col()
      .w(width)
      .gap(2)
      .px(4)
      .pt(2)
      .pb(2)
      .child(topFilterRow)
      .child(div().w_full().h(0.5).bg("#ffffff14"))
      .child(summaryRow)
      .child(appList)
      .child(div().w_full().h(0.5).bg("#ffffff14"))
      .child(footer);
  }
}
