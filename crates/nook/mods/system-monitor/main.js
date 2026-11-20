import { View, row, col, div, text, icon } from "nook/ui";
import { exec, island } from "nook";

const units = { K: 1024, M: 1024 ** 2, G: 1024 ** 3, T: 1024 ** 4 };

function bytes(value, unit) {
  return Number(value) * (units[unit] ?? 1);
}

function prettyBytes(value) {
  if (!Number.isFinite(value) || value < 0) return "—";
  const labels = ["B", "KB", "MB", "GB", "TB"];
  let n = value, i = 0;
  while (n >= 1024 && i < labels.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${labels[i]}`;
}

function readTop(output, memoryTotal) {
  const cpu = output.match(/CPU usage: ([\d.]+)% user, ([\d.]+)% sys, ([\d.]+)% idle/);
  const mem = output.match(/PhysMem: ([\d.]+)([KMGT]) used/);
  if (!cpu || !mem) throw new Error("Could not parse top output");
  const used = bytes(mem[1], mem[2]);
  return {
    cpu: 100 - Number(cpu[3]),
    user: Number(cpu[1]),
    system: Number(cpu[2]),
    memoryUsed: used,
    memoryTotal,
    memoryPct: used / memoryTotal * 100,
  };
}

function readDisk(output) {
  const totalMatch = output.match(/Container Total Space:\s*[^\n]*\(([\d,]+) Bytes\)/);
  const freeMatch = output.match(/Container Free Space:\s*[^\n]*\(([\d,]+) Bytes\)/);
  if (!totalMatch || !freeMatch) throw new Error("Could not read APFS container capacity");
  const total = Number(totalMatch[1].replaceAll(",", ""));
  const available = Number(freeMatch[1].replaceAll(",", ""));
  const used = Math.max(0, total - available);
  return { total, used, available, pct: used / total * 100 };
}

function gauge(label, percent, detail, color, percentSuffix = "") {
  const p = Math.max(0, Math.min(100, Number(percent) || 0));
  const trackWidth = Math.max(0, (island()?.page.width ?? 528) - 28);
  return col().id(`gauge-${label}`).gap(6)
    .child(row().w_full().justify_between()
      .child(text(label).text_size(12).font_medium().text_color("muted"))
      .child(row().gap(8)
        .child(text(detail).text_size(12).text_color("muted"))
        .child(text(`${Math.round(p)}%${percentSuffix}`).text_size(13).font_semibold().font_mono().text_color(color).tabular_nums())))
    .child(row().id(`track-${label}`).w_full().h(6).rounded_full().bg("raised")
      .child(div().id(`fill-${label}`).h(6).w(trackWidth * p / 100).rounded_full().bg(color)));
}

export default class SystemMonitor extends View {
  init(props, cx) {
    this.stats = null;
    this.disk = null;
    this.error = null;
    this.busy = false;
    this.refresh = (cx) => cx.spawn(async (cx) => {
      if (this.busy) return;
      this.busy = true;
      try {
        const [top, disk, totalMemory] = await Promise.all([
          exec("top", ["-l", "1", "-n", "0", "-stats", "cpu,mem"]),
          exec("diskutil", ["info", "/"]),
          exec("sysctl", ["-n", "hw.memsize"]),
        ]);
        if (top.code !== 0) throw new Error(top.stderr || "top failed");
        if (disk.code !== 0) throw new Error(disk.stderr || "diskutil failed");
        if (totalMemory.code !== 0) throw new Error(totalMemory.stderr || "sysctl failed");
        const physicalMemory = Number(totalMemory.stdout.trim());
        if (!Number.isFinite(physicalMemory) || physicalMemory <= 0) throw new Error("Could not read physical memory");
        this.stats = readTop(top.stdout, physicalMemory);
        this.disk = readDisk(disk.stdout);
        this.error = null;
      } catch (error) {
        this.error = String(error?.message ?? error);
      } finally {
        this.busy = false;
        cx.notify();
      }
    });
    this.refresh(cx);
    cx.timer.every(5000, this.refresh);
  }

  activity() {
    if (!this.stats) return { leading: icon("chart-column", 15).color("blue"), trailing: text("系统").text_size(12).text_color("muted"), priority: 100 };
    return {
      leading: icon("chart-column", 15).color(this.stats.cpu > 80 ? "orange" : "blue"),
      trailing: text(`CPU ${Math.round(this.stats.cpu)}%`).text_size(12).font_medium().tabular_nums(),
      detail: text(`内存 ${Math.round(this.stats.memoryPct)}%`).text_size(12).text_color("muted"),
      priority: 100,
    };
  }

  expanded() {
    if (!this.stats || !this.disk) {
      return row().id("loading").gap(9).py(12)
        .child(icon(this.error ? "info" : "activity", 16).color(this.error ? "orange" : "blue"))
        .child(text(this.error ?? "正在读取系统状态…").text_size(13).text_color("muted").truncate().flex_1());
    }
    const s = this.stats, d = this.disk;
    return col().id("system-monitor").w_full().gap(16).px(14).py(16)
      .child(row().id("heading").w_full().gap(8).items_center()
        .child(icon("chart-column", 17).color("blue"))
        .child(text("系统状态").text_size(15).font_semibold())
        .child(div().flex_1())
        .child(text("每 5 秒更新").text_size(10).text_color("faint")))
      .child(gauge("处理器", s.cpu, `${s.user.toFixed(0)}% 用户 · ${s.system.toFixed(0)}% 系统`, s.cpu > 80 ? "orange" : "blue"))
      .child(gauge("内存", s.memoryPct, `${prettyBytes(s.memoryUsed)} / ${prettyBytes(s.memoryTotal)}`, s.memoryPct > 85 ? "orange" : "green"))
      .child(gauge("磁盘", d.pct, `${prettyBytes(d.available)} 可用 / ${prettyBytes(d.total)} 总量`, d.pct > 90 ? "orange" : "purple", " 已用"));
  }
}
