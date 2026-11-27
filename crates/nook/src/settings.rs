//! The Settings window, and the first launch's onboarding in it. Native-looking (a sidebar,
//! grouped rows, switches, AppKit menus for choices), and backed by the same files an agent
//! edits: every change is written to layout.json, which the island applies live.

use crate::home;
use nook_core::permissions::Access;
use rune::tok::TextStyle;
use rune::{
    Color, Cx, Drag, El, Phase, PopupMenu, Spring, TextEdit, Tx, col, el, icon, row, text,
    text_input,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const SIZE: (f32, f32) = (880.0, 640.0);
const SIDEBAR: f32 = 190.0;
const ROW_H: f32 = 52.0;
const SNAPPY: Spring = Spring::new(0.25, 0.0);

/// Hex-like dark palette: a black sidebar, a raised content panel, cards on it.
mod c {
    use rune::Color;
    pub fn hex(s: &str) -> Color {
        rune::color::hex(s).unwrap_or(Color::WHITE)
    }
    pub fn sidebar() -> Color {
        hex("#0c0c0d")
    }
    pub fn panel() -> Color {
        hex("#16171a")
    }
    pub fn card() -> Color {
        hex("#1e2025")
    }
    pub fn line() -> Color {
        hex("#282b33")
    }
    pub fn selected() -> Color {
        hex("#282b33")
    }
    pub fn hover() -> Color {
        hex("#ffffff0d")
    }
    pub fn fg() -> Color {
        hex("#f2f2f3")
    }
    pub fn muted() -> Color {
        hex("#8e8e93")
    }
    pub fn faint() -> Color {
        hex("#5f5f63")
    }
    pub fn accent() -> Color {
        hex("#3d6df5")
    }
    pub fn off() -> Color {
        hex("#39393d")
    }
    pub fn red() -> Color {
        hex("#ff453a")
    }
    pub fn green() -> Color {
        hex("#30d158")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Widgets,
    General,
    Appearance,
    SystemStats,
    Notes,
    Launchers,
    Permissions,
    Mods,
    Yours,
    About,
}

const PAGES: [(Page, &str, &str); 10] = [
    (Page::Widgets, "小组件", "layout-grid"),
    (Page::General, "通用", "settings"),
    (Page::Appearance, "外观", "palette"),
    (Page::SystemStats, "系统状态", "activity"),
    (Page::Notes, "笔记与剪贴板", "notebook"),
    (Page::Launchers, "常用应用", "gauge"),
    (Page::Permissions, "权限", "shield-check"),
    (Page::Mods, "模块管理", "list"),
    (Page::Yours, "自定义", "pencil"),
    (Page::About, "关于", "info"),
];

#[derive(Clone, Copy)]
pub struct ThemeSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub bg_color: Color,
    pub border_color: Color,
    pub accent_color: Color,
    pub tag_color: Color,
}

pub const THEMES: [ThemeSpec; 8] = [
    ThemeSpec {
        id: "vinyl",
        name: "3D黑胶胆机",
        bg_color: Color {
            r: 0.12,
            g: 0.09,
            b: 0.06,
            a: 1.0,
        },
        border_color: Color {
            r: 0.85,
            g: 0.47,
            b: 0.02,
            a: 1.0,
        },
        accent_color: Color {
            r: 0.98,
            g: 0.75,
            b: 0.14,
            a: 1.0,
        },
        tag_color: Color {
            r: 0.96,
            g: 0.62,
            b: 0.07,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "classic",
        name: "3D玻璃",
        bg_color: Color {
            r: 0.14,
            g: 0.15,
            b: 0.18,
            a: 1.0,
        },
        border_color: Color {
            r: 0.24,
            g: 0.26,
            b: 0.31,
            a: 1.0,
        },
        accent_color: Color::WHITE,
        tag_color: Color {
            r: 0.04,
            g: 0.52,
            b: 1.0,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "dark",
        name: "纯粹曜黑",
        bg_color: Color {
            r: 0.07,
            g: 0.07,
            b: 0.09,
            a: 1.0,
        },
        border_color: Color {
            r: 0.15,
            g: 0.16,
            b: 0.20,
            a: 1.0,
        },
        accent_color: Color {
            r: 0.95,
            g: 0.95,
            b: 0.96,
            a: 1.0,
        },
        tag_color: Color {
            r: 0.56,
            g: 0.56,
            b: 0.58,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "klein",
        name: "克莱因蓝",
        bg_color: Color {
            r: 0.0,
            g: 0.18,
            b: 0.65,
            a: 1.0,
        },
        border_color: Color {
            r: 0.20,
            g: 0.41,
            b: 1.0,
            a: 1.0,
        },
        accent_color: Color::WHITE,
        tag_color: Color {
            r: 0.40,
            g: 0.64,
            b: 1.0,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "macaron",
        name: "甜心马卡龙",
        bg_color: Color {
            r: 0.21,
            g: 0.13,
            b: 0.20,
            a: 1.0,
        },
        border_color: Color {
            r: 1.0,
            g: 0.56,
            b: 0.70,
            a: 1.0,
        },
        accent_color: Color {
            r: 1.0,
            g: 0.70,
            b: 0.78,
            a: 1.0,
        },
        tag_color: Color {
            r: 0.50,
            g: 1.0,
            b: 0.82,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "muted",
        name: "莫兰迪灰",
        bg_color: Color {
            r: 0.16,
            g: 0.18,
            b: 0.20,
            a: 1.0,
        },
        border_color: Color {
            r: 0.28,
            g: 0.31,
            b: 0.35,
            a: 1.0,
        },
        accent_color: Color {
            r: 0.76,
            g: 0.79,
            b: 0.83,
            a: 1.0,
        },
        tag_color: Color {
            r: 0.56,
            g: 0.60,
            b: 0.65,
            a: 1.0,
        },
    },
    ThemeSpec {
        id: "solid",
        name: "纯色极简",
        bg_color: Color::BLACK,
        border_color: Color {
            r: 0.33,
            g: 0.33,
            b: 0.33,
            a: 1.0,
        },
        accent_color: Color::WHITE,
        tag_color: Color::WHITE,
    },
    ThemeSpec {
        id: "vibrant",
        name: "炫彩霓虹",
        bg_color: Color {
            r: 0.18,
            g: 0.05,
            b: 0.25,
            a: 1.0,
        },
        border_color: Color {
            r: 0.85,
            g: 0.27,
            b: 0.94,
            a: 1.0,
        },
        accent_color: Color {
            r: 0.0,
            g: 0.96,
            b: 1.0,
            a: 1.0,
        },
        tag_color: Color {
            r: 0.93,
            g: 0.28,
            b: 0.60,
            a: 1.0,
        },
    },
];

pub struct WidgetDef {
    pub id: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub icon: &'static str,
}

pub const WIDGET_DEFS: [WidgetDef; 8] = [
    WidgetDef {
        id: "soda-music",
        title: "汽水音乐",
        desc: "汽水音乐流媒体播放器，支持封面、音轨与青柠/克莱因视觉",
        category: "music",
        icon: "music",
    },
    WidgetDef {
        id: "codex",
        title: "笔记本与 Codex",
        desc: "快捷笔记备忘录与 Codex AI 实时额度/用量监视器",
        category: "notes",
        icon: "notebook",
    },
    WidgetDef {
        id: "clock",
        title: "时钟与天气",
        desc: "大字号数字时钟、日期星期与实时动态天气预报",
        category: "clock",
        icon: "clock",
    },
    WidgetDef {
        id: "map",
        title: "MAP",
        desc: "GPT 与 Gemini 的实时任务状态",
        category: "notes",
        icon: "bot",
    },
    WidgetDef {
        id: "battery",
        title: "电池电量与续航",
        desc: "当前电量百分比、充电指示与剩余续航估算",
        category: "system",
        icon: "battery-charging",
    },
    WidgetDef {
        id: "network",
        title: "实时网络速率",
        desc: "上下行实时传输速率监控与流量指示",
        category: "system",
        icon: "gauge",
    },
    WidgetDef {
        id: "rings",
        title: "3D 性能光环",
        desc: "CPU、GPU、内存与启动盘存储用量",
        category: "system",
        icon: "activity",
    },
    WidgetDef {
        id: "launchers",
        title: "常用应用抽屉",
        desc: "一键启动常用 App（终端、音乐、代码、浏览器等）",
        category: "launchers",
        icon: "layout-grid",
    },
];

#[derive(Clone, Debug)]
pub struct ModInfo {
    pub name: String,
    pub title: String,
    pub icon: Option<String>,
    pub picture: Option<(u32, u32, Vec<u8>)>,
    pub description: Option<String>,
    pub error: bool,
}

const AGENTS: [(&str, &str); 5] = [
    ("claude", "Claude Code"),
    ("codex", "Codex"),
    ("opencode", "OpenCode"),
    ("pi", "Pi"),
    ("gemini", "Gemini"),
];

pub struct Settings {
    pub page: Page,
    pub onboarding: Option<u8>,
    layout: Value,
    mods: Vec<ModInfo>,
    login: bool,
    wish: TextEdit,
    agents: Vec<(&'static str, &'static str)>,
    agent: &'static str,
    herdr: bool,
    note: Option<(String, bool)>,
    refreshed: Option<Instant>,
    place: TextEdit,
    finding: Option<std::sync::mpsc::Receiver<Result<Vec<Value>, String>>>,
    found: Vec<Value>,
    place_note: Option<(String, bool)>,
    audio_access: Access,
    location_access: Access,
    asked_for: Option<String>,
    pub category_filter: String,
    pub widget_styles: HashMap<String, String>,
    pub active_widgets: Vec<String>,
}

impl Settings {
    pub fn new() -> Self {
        let mut s = Self {
            page: Page::Widgets,
            onboarding: None,
            layout: Value::Null,
            mods: Vec::new(),
            login: false,
            wish: TextEdit::new(""),
            agents: Vec::new(),
            agent: "claude",
            herdr: false,
            note: None,
            refreshed: None,
            place: TextEdit::new(""),
            finding: None,
            found: Vec::new(),
            place_note: None,
            audio_access: Access::NotAsked,
            location_access: Access::NotAsked,
            asked_for: nook_core::permissions::asked_for(),
            category_filter: "all".into(),
            widget_styles: HashMap::new(),
            active_widgets: Vec::new(),
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        let dir = home::app_dir();
        self.layout = std::fs::read_to_string(dir.join("layout.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| json!({}));
        let mods = dir.join("mods");
        self.mods = nook_mods::files::scan(&mods)
            .into_iter()
            .map(|f| {
            let m = nook_mods::files::read_manifest(&f.dir).unwrap_or_default();
                let picture = m
                    .icon
                    .as_deref()
                    .filter(|i| crate::icons::is_picture(i))
                    .and_then(|i| picture(&f.dir.join(i)));
                ModInfo {
                    error: mods.join(format!("{}.error", f.name)).exists(),
                    title: m.title.unwrap_or_else(|| f.name.clone()),
                    icon: m.icon,
                    picture,
                    description: m.description,
                    name: f.name,
                }
            })
            .collect();
        self.login = nook_core::login_item::is_enabled();
        let path = nook_mods::user_path(&home::home());
        self.agents = AGENTS
            .iter()
            .copied()
            .filter(|(cmd, _)| {
                path.split(':')
                    .any(|d| std::path::Path::new(d).join(cmd).is_file())
            })
            .collect();
        if !self.agents.iter().any(|(c, _)| *c == self.agent) {
            self.agent = self.agents.first().map_or("claude", |a| a.0);
        }
        self.audio_access = nook_core::permissions::audio_capture();
        self.location_access = nook_core::permissions::location();
        self.herdr = std::path::Path::new("/Applications/Herdr.app").exists()
            || path
                .split(':')
                .any(|d| std::path::Path::new(d).join("herdr-app").is_file());

        let props = self
            .layout
            .get("mods")
            .and_then(|m| m.get("dashboard"))
            .and_then(|d| d.get("props"));
        let mut styles = HashMap::new();
        if let Some(s) = props
            .and_then(|p| p.get("widgetStyles"))
            .and_then(Value::as_object)
        {
            for (k, v) in s {
                if let Some(val) = v.as_str() {
                    styles.insert(k.clone(), val.to_string());
                }
            }
        }
        if !styles.contains_key("soda-music")
            || styles.get("soda-music").map(String::as_str) == Some("vinyl")
        {
            styles.insert("soda-music".into(), "klein".into());
        }
        if !styles.contains_key("codex") {
            styles.insert("codex".into(), "classic".into());
        }
        if !styles.contains_key("clock") {
            styles.insert("clock".into(), "macaron".into());
        }
        let map_style = styles.remove("clipboard").unwrap_or_else(|| "dark".into());
        styles.entry("map".into()).or_insert(map_style);
        if !styles.contains_key("battery") {
            styles.insert("battery".into(), "muted".into());
        }
        if !styles.contains_key("network") {
            styles.insert("network".into(), "vibrant".into());
        }
        if !styles.contains_key("rings") {
            styles.insert("rings".into(), "classic".into());
        }
        if !styles.contains_key("launchers") {
            styles.insert("launchers".into(), "solid".into());
        }
        self.widget_styles = styles;

        let mut active = Vec::new();
        if let Some(arr) = props
            .and_then(|p| p.get("activeWidgets"))
            .and_then(Value::as_array)
        {
            for v in arr {
                if let Some(w) = v.as_str() {
                    let widget = if w == "clipboard" { "map" } else { w };
                    if widget != "map" || !active.iter().any(|id| id == "map") {
                        active.push(widget.to_string());
                    }
                }
            }
        }
        if active.is_empty() {
            active = vec![
                "codex".into(),
                "clock".into(),
                "map".into(),
                "battery".into(),
                "network".into(),
                "rings".into(),
                "launchers".into(),
            ];
        } else {
            if !active.iter().any(|w| w == "network") {
                active.push("network".into());
            }
            if !active.iter().any(|w| w == "rings") {
                active.push("rings".into());
            }
        }
        self.active_widgets = active;

        self.refreshed = Some(Instant::now());
    }

    pub fn set_widget_style(&mut self, widget_id: &str, style: &str) {
        self.widget_styles
            .insert(widget_id.to_string(), style.to_string());
        self.save_widgets_to_layout();
    }

    pub fn toggle_active_widget(&mut self, widget_id: &str) {
        if let Some(pos) = self.active_widgets.iter().position(|w| w == widget_id) {
            self.active_widgets.remove(pos);
        } else {
            self.active_widgets.push(widget_id.to_string());
        }
        self.save_widgets_to_layout();
    }

    pub fn reset_widgets_to_defaults(&mut self) {
        self.active_widgets = vec![
            "codex".into(),
            "clock".into(),
            "map".into(),
            "battery".into(),
            "network".into(),
            "rings".into(),
            "launchers".into(),
        ];
        self.widget_styles
            .insert("soda-music".into(), "klein".into());
        self.widget_styles.insert("codex".into(), "classic".into());
        self.widget_styles.insert("clock".into(), "macaron".into());
        self.widget_styles.insert("map".into(), "dark".into());
        self.widget_styles.insert("battery".into(), "muted".into());
        self.widget_styles
            .insert("network".into(), "vibrant".into());
        self.widget_styles.insert("rings".into(), "classic".into());
        self.widget_styles
            .insert("launchers".into(), "solid".into());
        self.write_widget_layout(true);
    }

    pub fn save_widgets_to_layout(&mut self) {
        self.write_widget_layout(false);
    }

    fn write_widget_layout(&mut self, reset_rows: bool) {
        let active = self.active_widgets.clone();
        let styles = self.widget_styles.clone();
        self.edit_layout(move |v| {
            if !v["mods"].is_object() {
                v["mods"] = json!({});
            }
            if !v["mods"]["dashboard"].is_object() {
                v["mods"]["dashboard"] = json!({});
            }
            if !v["mods"]["dashboard"]["props"].is_object() {
                v["mods"]["dashboard"]["props"] = json!({});
            }
            v["mods"]["dashboard"]["props"]["activeWidgets"] = json!(active);
            v["mods"]["dashboard"]["props"]["widgetStyles"] = json!(styles);
            if reset_rows && let Some(props) = v["mods"]["dashboard"]["props"].as_object_mut() {
                props.remove("widgetRows");
            }
        });
    }

    fn island(&self, key: &str) -> Option<&Value> {
        self.layout.get("island").and_then(|i| i.get(key))
    }

    fn hidden(&self, name: &str) -> bool {
        self.layout
            .get("hidden")
            .and_then(Value::as_array)
            .is_some_and(|h| h.iter().any(|v| v.as_str() == Some(name)))
    }

    fn edit_layout(&mut self, f: impl FnOnce(&mut Value)) {
        self.layout = home::edit_layout(f);
    }

    fn set_island(&mut self, key: &str, value: Value) {
        let key = key.to_owned();
        self.edit_layout(move |v| {
            if !v["island"].is_object() {
                v["island"] = json!({});
            }
            v["island"][key] = value;
        });
    }

    fn set_location_mode(&mut self, mode: &str) {
        let mode = mode.to_owned();
        self.edit_layout(move |v| {
            if !v["location"].is_object() {
                v["location"] = json!({});
            }
            v["location"]["mode"] = Value::from(mode);
        });
    }

    fn find_place(&mut self) {
        let query = self.place.text.trim().to_owned();
        if query.is_empty() || self.finding.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.finding = Some(rx);
        self.place_note = Some((format!("正在查找“{query}”…"), true));
        std::thread::spawn(move || {
            let _ = tx.send(geocode(&query));
        });
    }

    fn toggle_hidden(&mut self, name: &str) {
        let name = name.to_owned();
        self.edit_layout(move |v| {
            let mut hidden: Vec<Value> = v
                .get("hidden")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Some(i) = hidden.iter().position(|h| h.as_str() == Some(&name)) {
                hidden.remove(i);
            } else {
                hidden.push(Value::from(name));
            }
            v["hidden"] = Value::Array(hidden);
        });
    }

    fn prompt(&self) -> String {
        let wish = self.wish.text.trim();
        let wish = if wish.is_empty() {
            "Ask me what I'd like in my notch, then build it."
        } else {
            wish
        };
        format!(
            "You're customizing openNook, the island in my MacBook's notch. Everything it shows is a mod in \
             ~/.config/opennook, and ~/.config/opennook/AGENTS.md explains all of it: read it first. Then: {wish}\n\n\
             Check every mod you write with `{exe} check <mod folder>` and look at the island with `{exe} shot` \
             (add --mod <name> for its page) before you call it done.",
            exe = exe().display()
        )
    }

    fn start_in_herdr(&mut self) {
        let args = json!({ "name": "openNook", "folder": home::app_dir(), "argv": [self.agent], "task": self.prompt() });
        let result = std::process::Command::new("herdr-app")
            .env("PATH", nook_mods::user_path(&home::home()))
            .args(["do", "space.new", &args.to_string()])
            .output();
        self.note = Some(match result {
            Ok(o) if o.status.success() => {
                let _ = open::that_detached("/Applications/Herdr.app");
                ("已在 Herdr 中启动，任务说明已交给编程助手。".into(), true)
            }
            Ok(o) => (
                format!("Herdr：{}", String::from_utf8_lossy(&o.stderr).trim()),
                false,
            ),
            Err(e) => (format!("Herdr 启动失败：{e}"), false),
        });
    }

    fn finish_onboarding(&mut self) {
        self.onboarding = None;
        self.page = Page::Widgets;
        let _ = std::fs::write(onboarded_marker(), "");
    }
}

pub fn onboarded_marker() -> PathBuf {
    home::app_dir().join("data").join("onboarded")
}

pub fn first_launch() -> bool {
    !onboarded_marker().exists()
}

pub fn exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| "opennook".into())
}

pub fn skill() -> String {
    format!(
        "---\nname: opennook\ndescription: Customize openNook, the island in the MacBook notch: write or change its mods \
         (JavaScript), its layout and theme. Use when someone asks to change what the notch shows or how it looks or moves.\n---\n\n\
         # openNook\n\nopenNook's whole interface is files in `~/.config/opennook`. Read `~/.config/opennook/AGENTS.md` before \
         changing anything: it covers mods, the `nook` and `nook/ui` APIs (`nook.d.ts`, `ui.d.ts`), layout.json and shaders.\n\n\
         - Check a mod: `{exe} check ~/.config/opennook/mods/<name>` (never call it done without a pass).\n\
         - Look at it: `{exe} shot island.png` (`--mod <name>` for its page, `--hover` for the peek).\n\
         - Edits apply live; a broken one keeps the last good version and writes `mods/<name>.error`.\n",
        exe = exe().display()
    )
}

fn set_up_agents() -> Result<String, String> {
    let home = home::home();
    let dir = home.join(".claude/skills/opennook");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    nook_mods::files::write_if_changed(&dir.join("SKILL.md"), skill())
        .map_err(|e| e.to_string())?;
    let bin = home.join(".local/bin");
    let _ = std::fs::create_dir_all(&bin);
    let link = bin.join("opennook");
    let ok_link = match std::fs::read_link(&link) {
        Ok(target) if target == exe() => true,
        Ok(_) | Err(_) => {
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(exe(), &link).is_ok()
        }
    };
    Ok(if ok_link {
        "已为 Claude Code 安装 openNook 技能，并将 `opennook` 添加到 PATH。".into()
    } else {
        "已为 Claude Code 安装 openNook 技能。".into()
    })
}

fn geocode(query: &str) -> Result<Vec<Value>, String> {
    let url = reqwest::Url::parse_with_params(
        "https://geocoding-api.open-meteo.com/v1/search",
        &[("count", "5"), ("name", query)],
    )
    .map_err(|e| e.to_string())?;
    let body: Value = reqwest::blocking::get(url)
        .and_then(|r| r.json())
        .map_err(|e| format!("couldn't look it up: {e}"))?;
    let hits = body
        .get("results")
        .and_then(Value::as_array)
        .filter(|h| !h.is_empty())
        .ok_or_else(|| format!("no place called {query}"))?;
    Ok(hits
        .iter()
        .map(|hit| {
            let region = [hit.get("admin1"), hit.get("country")]
                .iter()
                .filter_map(|v| v.and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(", ");
        json!({
            "mode": "set",
            "name": hit.get("name"),
            "latitude": hit.get("latitude"),
            "longitude": hit.get("longitude"),
            "timezone": hit.get("timezone"),
            "region": region,
        })
        })
        .collect())
}

impl Settings {
    fn use_place(&mut self, mut place: Value) {
        let name = place
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("that place")
            .to_owned();
        if let Some(p) = place.as_object_mut() {
            p.remove("region");
        }
        self.edit_layout(move |v| v["location"] = place);
        self.place = TextEdit::new("");
        self.found.clear();
        self.place_note = Some((format!("当前地点：{name}"), true));
    }
}

pub fn poll(s: &mut Settings, _t: f64, tx: &mut Tx) -> bool {
    tx.wake_after(Duration::from_secs(2));
    if let Some(found) = s.finding.as_ref().and_then(|rx| rx.try_recv().ok()) {
        s.finding = None;
        match found {
            Ok(mut places) if places.len() == 1 => s.use_place(places.remove(0)),
            Ok(places) => {
                let label = |p: &Value| {
                    let name = p.get("name").and_then(Value::as_str).unwrap_or("?");
                    match p
                        .get("region")
                        .and_then(Value::as_str)
                        .filter(|r| !r.is_empty())
                    {
                        Some(r) => format!("{name} — {r}"),
                        None => name.to_owned(),
                    }
                };
                let options: Vec<(String, String)> = places
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i.to_string(), label(p)))
                    .collect();
                s.found = places;
                s.place_note = Some(("请选择地点".into(), true));
                tx.popup_menu(
                    PopupMenu::picker(options, ""),
                    "place/set",
                    |s: &mut Settings, picked, _| {
                        let place = picked
                            .and_then(|i| i.parse::<usize>().ok())
                            .and_then(|i| s.found.get(i).cloned());
                    match place {
                        Some(p) => s.use_place(p),
                        None => {
                            s.found.clear();
                            s.place_note = None;
                        }
                    }
                    },
                );
            }
            Err(e) => s.place_note = Some((format!("无法使用该地点：{e}"), false)),
        }
        return true;
    }
    if s.finding.is_some() {
        tx.wake_after(Duration::from_millis(200));
    }
    if s.refreshed
        .is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
    {
        s.refresh();
        return true;
    }
    false
}

// Components ----------------------------------------------------------------------------

fn font(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight)
}

fn label(s: impl Into<String>, size: f32, weight: f32, color: Color) -> El {
    text(s.into()).font(font(size, weight)).color(color)
}

fn glyph(name: &str, size: f32, color: Color) -> El {
    match crate::icons::get(name, std::path::Path::new("/")) {
        Some(i) => icon(i, size).color(color),
        None => el().size(size, size),
    }
}

fn switch(key: &str, on: bool) -> El {
    let (w, h, k) = (40.0, 24.0, 20.0);
    let inset = (h - k) / 2.0;
    el().key(format!("{key}/switch"))
        .size(w, h)
        .radius(h / 2.0)
        .fill(if on { c::accent() } else { c::off() })
        .motion(SNAPPY)
        .child(
            el().key(format!("{key}/knob"))
                .size(k, k)
                .absolute(if on { w - k - inset } else { inset }, inset)
                .radius(k / 2.0)
                .fill(Color::WHITE)
                .shadow(1.0, 3.0, Color::BLACK.alpha(0.3)),
        )
}

fn setting(key: &str, glyph_name: &str, title: &str, detail: Option<&str>, end: El) -> El {
    let mut words = col()
        .gap(2.0)
        .grow(1.0)
        .shrink()
        .child(label(title, 14.5, 500.0, c::fg()));
    if let Some(d) = detail {
        words = words.child(label(d, 12.5, 400.0, c::muted()).truncate());
    }
    row()
        .key(key.to_owned())
        .min_h(ROW_H)
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .items_center()
        .child(
            row()
                .w(22.0)
                .justify_center()
                .child(glyph(glyph_name, 18.0, c::muted())),
        )
        .child(words)
        .child(end)
}

fn toggle(
    key: &str,
    glyph_name: &str,
    title: &str,
    detail: Option<&str>,
    on: bool,
    flip: impl Fn(&mut Settings) + 'static,
) -> El {
    setting(key, glyph_name, title, detail, switch(key, on))
        .hover(c::hover())
        .on_press(flip)
}

fn choice(
    key: &'static str,
    glyph_name: &str,
    title: &str,
    options: &'static [(&'static str, &'static str)],
    current: &str,
    pick: fn(&mut Settings, &str),
) -> El {
    let shown = options
        .iter()
        .find(|(id, _)| *id == current)
        .map_or(current, |(_, l)| l)
        .to_owned();
    let value_key = format!("{key}/value");
    let current = current.to_owned();
    let end = row()
        .key(value_key.clone())
        .gap(6.0)
        .items_center()
        .child(label(shown, 14.5, 600.0, c::fg()))
        .child(glyph("chevron-right", 14.0, c::muted()));
    setting(key, glyph_name, title, None, end)
        .hover(c::hover())
        .on_press_tx(move |_: &mut Settings, tx: &mut Tx| {
        let menu = PopupMenu::picker(options.iter().copied(), &current);
            tx.popup_menu(
                menu,
                value_key.as_str(),
                move |s: &mut Settings, picked, _| {
            if let Some(id) = picked {
                pick(s, id);
            }
                },
            );
    })
}

fn slider(key: &'static str, value: f32, set: fn(&mut Settings, f32, bool)) -> El {
    let (w, h, k, bar) = (170.0, 22.0, 18.0, 4.0);
    let travel = w - k;
    let to_value = move |x: f32| ((x - k / 2.0) / travel).clamp(0.0, 1.0);
    el().key(format!("{key}/slider"))
        .size(w, h)
        .motion(SNAPPY)
        .on_drag(move |s: &mut Settings, d: &mut Drag, tx| match d.phase {
            Phase::Down => set(s, to_value(d.x), false),
            Phase::Move => {
                set(s, to_value(d.x), false);
                tx.instant();
            }
            Phase::Up => set(s, to_value(d.x), true),
        })
        .child(
            el().size(w, bar)
                .absolute(0.0, (h - bar) / 2.0)
                .radius(bar / 2.0)
                .fill(c::off()),
        )
        .child(
            el().key(format!("{key}/fill"))
                .size(k / 2.0 + value * travel, bar)
                .absolute(0.0, (h - bar) / 2.0)
                .radius(bar / 2.0)
                .fill(c::accent()),
        )
        .child(
            el().key(format!("{key}/knob"))
                .size(k, k)
                .absolute(value * travel, (h - k) / 2.0)
                .radius(k / 2.0)
                .fill(Color::WHITE)
                .shadow(1.0, 3.0, Color::BLACK.alpha(0.3)),
        )
}

fn card(rows: Vec<El>) -> El {
    let n = rows.len();
    let mut card = col()
        .w_full()
        .radius(12.0)
        .fill(c::card())
        .border(1.0, c::line())
        .clip_rounded(12.0);
    for (i, r) in rows.into_iter().enumerate() {
        card = card.child(r);
        if i + 1 < n {
            card = card.child(el().h(1.0).ml(50.0).mr(16.0).fill(c::line()));
        }
    }
    card
}

fn button(key: &str, title: &str, primary: bool, press: impl Fn(&mut Settings) + 'static) -> El {
    row()
        .key(key.to_owned())
        .px(14.0)
        .h(32.0)
        .radius(8.0)
        .items_center()
        .fill(if primary { c::accent() } else { c::card() })
        .border(1.0, if primary { c::accent() } else { c::line() })
        .hover(if primary {
            Color::WHITE.alpha(0.08)
        } else {
            c::hover()
        })
        .on_press(press)
        .child(label(title, 13.5, 600.0, c::fg()))
}

fn picture(path: &std::path::Path) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::open(path)
        .ok()?
        .resize(44, 44, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    Some((img.width(), img.height(), img.into_raw()))
}

fn mod_icon(m: &ModInfo) -> El {
    if let Some((w, h, rgba)) = &m.picture {
        let px = rune::Pixels::new();
        px.set(*w, *h, rgba.clone());
        return el().size(22.0, 22.0).pixels_fit(&px, rune::SurfaceFit::Fit);
    }
    match m.icon.as_deref() {
        Some(i) if i.starts_with("app:") => {
            let id = &i[4..];
            match crate::platform::app_icon_light(id, 44) {
                Some((w, h, rgba)) => {
                    let px = rune::Pixels::new();
                    px.set(w, h, rgba);
                    el().size(22.0, 22.0).pixels_fit(&px, rune::SurfaceFit::Fit)
                }
                None => glyph("layout-grid", 18.0, c::muted()),
            }
        }
        Some(i) => glyph(i, 18.0, c::muted()),
        None => glyph("layout-grid", 18.0, c::muted()),
    }
}

// Top Live Notch Island Preview ---------------------------------------------------------

fn notch_preview(s: &Settings) -> El {
    let soda_style = s
        .widget_styles
        .get("soda-music")
        .map(String::as_str)
        .unwrap_or("klein");
    let music_theme = THEMES
        .iter()
        .find(|t| t.id == soda_style)
        .unwrap_or(&THEMES[3]);

    let clock_style = s
        .widget_styles
        .get("clock")
        .map(String::as_str)
        .unwrap_or("macaron");
    let _clock_theme = THEMES
        .iter()
        .find(|t| t.id == clock_style)
        .unwrap_or(&THEMES[4]);

    let notes_style = s
        .widget_styles
        .get("codex")
        .map(String::as_str)
        .unwrap_or("classic");
    let notes_theme = THEMES
        .iter()
        .find(|t| t.id == notes_style)
        .unwrap_or(&THEMES[1]);

    let island = row()
        .key("preview/island")
        .w(560.0)
        .h(72.0)
        .radius(18.0)
        .fill(c::hex("#101217"))
        .border(1.0, c::hex("#ffffff1f"))
        .shadow(2.0, 10.0, Color::BLACK.alpha(0.7))
        .px(14.0)
        .py(6.0)
        .gap(12.0)
        .items_center()
        .child(
            // Music / Soda Music slot
            row()
                .w(165.0)
                .gap(8.0)
                .items_center()
                .child(
                    row()
                        .size(38.0, 38.0)
                        .radius(9.0)
                        .fill(music_theme.bg_color)
                        .border(1.0, music_theme.border_color)
                        .items_center()
                        .justify_center()
                        .child(glyph("music", 17.0, music_theme.accent_color)),
                )
                .child(
                    col()
                        .gap(2.0)
                        .grow(1.0)
                        .shrink()
                        .child(
                            row()
                                .gap(4.0)
                                .items_center()
                                .child(label("汽水音乐", 12.0, 600.0, Color::WHITE).truncate())
                                .child(
                                    row()
                                        .px(3.5)
                                        .py(0.5)
                                        .radius(3.0)
                                        .fill(music_theme.tag_color.alpha(0.25))
                                        .child(label(
                                            music_theme.name,
                                            8.5,
                                            600.0,
                                            music_theme.tag_color,
                                        )),
                                ),
                        )
                        .child(label("流媒体推荐 · 播放中", 10.0, 400.0, c::muted()).truncate())
                        .child(
                            row()
                                .gap(8.0)
                                .items_center()
                                .pt(1.0)
                                .child(glyph("play", 10.0, Color::WHITE))
                                .child(glyph("skip-forward", 10.0, c::muted())),
                        ),
                ),
        )
        .child(
            // Center slot: Clock & Date & Weather
            col()
                .w(175.0)
                .gap(1.0)
                .items_center()
                .justify_center()
                .child(label("17:09", 19.0, 700.0, Color::WHITE))
                .child(label("2026年2月14日 周六", 10.0, 500.0, c::muted()))
                .child(
                    row()
                        .gap(4.0)
                        .items_center()
                        .pt(1.0)
                        .child(glyph("cloud-sun", 11.0, c::hex("#ffcc00")))
                        .child(label("18° 多云", 10.0, 400.0, c::fg()))
                        .child(label("· 68%", 10.0, 400.0, c::green())),
                ),
        )
        .child(
            // Right slot: Notes / Codex
            row()
                .w(165.0)
                .h(46.0)
                .radius(9.0)
                .fill(notes_theme.bg_color)
                .border(1.0, notes_theme.border_color)
                .px(8.0)
                .py(4.0)
                .gap(6.0)
                .items_center()
                .child(glyph("notebook", 14.0, notes_theme.accent_color))
                .child(
                    col()
                        .gap(2.0)
                        .grow(1.0)
                        .shrink()
                        .child(
                            label("无笔记内容", 11.0, 600.0, notes_theme.accent_color).truncate(),
                )
                        .child(label("Codex 运行状态与额度", 9.5, 400.0, c::muted()).truncate()),
                ),
        );

    col()
        .key("notch_preview")
        .w_full()
        .h(116.0)
        .radius(12.0)
        .fill(c::hex("#0a0b0e"))
        .border(1.0, c::hex("#20232a"))
        .clip_rounded(12.0)
        .px(12.0)
        .pt(5.0)
        .pb(7.0)
        .gap(4.0)
        .items_center()
        .child(
            row()
                .w_full()
                .items_center()
                .justify_between()
                .child(label(
                    "MacBook 灵动岛仿真预览 (Live Notch Preview)",
                    11.0,
                    600.0,
                    c::muted(),
                ))
                .child(label(
                    "点击下方卡片可立即切换皮肤或增删组件",
                    10.5,
                    400.0,
                    c::faint(),
                )),
        )
        .child(island)
}

// Niya Component Gallery ----------------------------------------------------------------

fn card_5x5(
    key: &str,
    w: &WidgetDef,
    t: &ThemeSpec,
    is_active: bool,
    is_current_style: bool,
) -> El {
    let wid = w.id;
    let tid = t.id;
    let tag = if is_active && is_current_style {
        row()
            .px(5.0)
            .py(1.0)
            .radius(4.0)
            .fill(t.tag_color.alpha(0.3))
            .border(1.0, t.tag_color)
            .child(label("已在首页", 9.0, 600.0, t.tag_color))
    } else if is_active {
        row()
            .px(5.0)
            .py(1.0)
            .radius(4.0)
            .fill(c::hover())
            .border(1.0, c::line())
            .child(label("切换皮肤", 9.0, 500.0, c::muted()))
    } else {
        row()
            .px(5.0)
            .py(1.0)
            .radius(4.0)
            .fill(c::accent().alpha(0.25))
            .border(1.0, c::accent())
            .child(label("+ 添加", 9.0, 500.0, c::accent()))
    };

    let top_row = row()
        .w_full()
        .items_center()
        .justify_between()
        .child(
            row()
                .gap(4.0)
                .items_center()
                .child(glyph(w.icon, 12.0, t.accent_color))
                .child(
                    label(w.title, 11.0, 600.0, t.accent_color)
                        .truncate()
                        .max_w(80.0),
                ),
        )
        .child(tag);

    let middle = match w.category {
        _ if wid == "map" => label("GPT · Gemini", 10.0, 600.0, t.accent_color),
        "music" if t.id == "vinyl" => row()
            .gap(5.0)
            .items_center()
            .child(
                row()
                    .size(22.0, 22.0)
                    .radius(11.0)
                    .fill(c::hex("#0c0d10"))
                    .border(1.0, c::hex("#d97706"))
                    .items_center()
                    .justify_center()
            .child(
                        row()
                            .size(10.0, 10.0)
                            .radius(5.0)
                            .fill(c::hex("#047857"))
                            .items_center()
                            .justify_center()
                            .child(el().size(3.0, 3.0).radius(1.5).fill(c::hex("#fbbf24"))),
                    ),
            )
            .child(
                row()
                    .w(8.0)
                    .h(20.0)
                    .radius(3.0)
                    .fill(c::hex("#2b1508"))
                    .border(0.5, c::hex("#f59e0b"))
                    .items_center()
                    .justify_center()
                    .child(el().w(2.0).h(6.0).radius(1.0).fill(c::hex("#fef08a"))),
            )
            .child(label("3D黑胶 × 真空管", 9.0, 500.0, t.accent_color)),
        "music" => row()
            .gap(2.0)
            .items_center()
            .child(
                el().w(2.5)
                    .h(5.0)
                    .radius(1.0)
                    .fill(t.accent_color.alpha(0.8)),
            )
            .child(el().w(2.5).h(13.0).radius(1.0).fill(t.accent_color))
            .child(
                el().w(2.5)
                    .h(8.0)
                    .radius(1.0)
                    .fill(t.accent_color.alpha(0.8)),
            )
            .child(el().w(2.5).h(15.0).radius(1.0).fill(t.accent_color))
            .child(
                el().w(2.5)
                    .h(7.0)
                    .radius(1.0)
                    .fill(t.accent_color.alpha(0.8)),
            )
            .child(label("正在播放 · 推荐", 9.0, 400.0, t.accent_color.alpha(0.7)).ml(4.0)),
        "notes" => col()
            .gap(2.5)
            .child(
                el().w(80.0)
                    .h(2.5)
                    .radius(1.0)
                    .fill(t.accent_color.alpha(0.6)),
            )
            .child(
                el().w(55.0)
                    .h(2.5)
                    .radius(1.0)
                    .fill(t.accent_color.alpha(0.4)),
            ),
        "clock" => row()
            .gap(4.0)
            .items_center()
            .child(label("17:09", 13.0, 700.0, t.accent_color))
            .child(label("18° 多云", 9.0, 400.0, t.accent_color.alpha(0.7))),
        "system" => row()
            .gap(4.0)
            .items_center()
            .child(
                el().w(46.0)
                    .h(3.5)
                    .radius(1.5)
                    .fill(t.accent_color.alpha(0.2))
                    .child(el().w(30.0).h(3.5).radius(1.5).fill(t.accent_color)),
            )
            .child(label("68%", 9.0, 500.0, t.accent_color)),
        _ => row()
            .gap(4.0)
            .items_center()
            .child(glyph("gauge", 11.0, t.accent_color))
            .child(label(
                "终端 · 音乐 · 设置",
                9.0,
                400.0,
                t.accent_color.alpha(0.7),
            )),
    };

    let bot_row = match w.category {
        _ if wid == "map" => label("实时任务状态", 8.5, 400.0, t.accent_color.alpha(0.7)),
        "music" if t.id == "vinyl" => row()
            .gap(6.0)
            .items_center()
            .child(glyph("play", 9.0, t.accent_color))
            .child(glyph("skip-forward", 9.0, t.accent_color.alpha(0.7)))
            .child(label("Hi-Fi 胆机暖声", 8.5, 400.0, t.tag_color).ml(3.0)),
        "music" => row()
            .gap(6.0)
            .items_center()
            .child(glyph("play", 9.0, t.accent_color.alpha(0.8)))
            .child(glyph("skip-forward", 9.0, t.accent_color.alpha(0.6)))
            .child(label("流媒体音轨", 8.5, 400.0, t.accent_color.alpha(0.5)).ml(3.0)),
        "notes" => row()
            .gap(4.0)
            .items_center()
            .child(glyph("check", 8.5, t.tag_color))
            .child(label("备忘已就绪", 8.5, 400.0, t.accent_color.alpha(0.6))),
        "clock" => label("精准授时 · 实时", 8.5, 400.0, t.accent_color.alpha(0.5)),
        "system" => label("健康度 100% · 正常", 8.5, 400.0, t.accent_color.alpha(0.5)),
        _ => label("一键极速调起", 8.5, 400.0, t.accent_color.alpha(0.5)),
    };

    let box_el = col()
        .key(format!("card5x5/{key}"))
        .w(146.0)
        .h(76.0)
        .radius(10.0)
        .fill(t.bg_color)
        .border(
            1.0,
            if is_current_style {
                t.tag_color
            } else {
                t.border_color
            },
        )
        .padding(7.0)
        .justify_between()
        .hover(c::hover())
        .on_press(move |s: &mut Settings| {
            s.set_widget_style(wid, tid);
            if !s.active_widgets.contains(&wid.to_string()) {
                s.active_widgets.push(wid.to_string());
            }
            s.save_widgets_to_layout();
        })
        .child(top_row)
        .child(middle)
        .child(bot_row);

    col().gap(3.0).child(box_el).child(
        label(
            format!("{} ({} 5x5)", w.title, t.id),
            10.5,
            500.0,
            if is_current_style {
                t.tag_color
            } else {
                c::muted()
            },
        )
        .truncate()
        .max_w(146.0),
    )
}

fn card_2x2(
    key: &str,
    w: &WidgetDef,
    t: &ThemeSpec,
    is_active: bool,
    is_current_style: bool,
) -> El {
    let wid = w.id;
    let tid = t.id;
    let box_el = col()
        .key(format!("card2x2/{key}"))
        .size(76.0, 76.0)
        .radius(12.0)
        .fill(t.bg_color)
        .border(
            1.0,
            if is_current_style {
                t.tag_color
            } else {
                t.border_color
            },
        )
        .items_center()
        .justify_center()
        .gap(5.0)
        .hover(c::hover())
        .on_press(move |s: &mut Settings| {
            s.set_widget_style(wid, tid);
            if !s.active_widgets.contains(&wid.to_string()) {
                s.active_widgets.push(wid.to_string());
            }
            s.save_widgets_to_layout();
        })
        .child(glyph(w.icon, 20.0, t.accent_color))
        .child(if is_active && is_current_style {
            row()
                .size(14.0, 14.0)
                .radius(7.0)
                .fill(t.tag_color)
                .items_center()
                .justify_center()
                .child(glyph("check", 9.0, Color::BLACK))
            } else if is_active {
                label(t.name, 9.0, 500.0, c::muted())
            } else {
            row()
                .size(14.0, 14.0)
                .radius(7.0)
                .fill(c::accent().alpha(0.25))
                .items_center()
                .justify_center()
                .child(glyph("plus", 9.0, c::accent()))
        });

    col().gap(3.0).child(box_el).child(
        label(
            format!("{} ({} 2x2)", w.title, t.id),
            9.5,
            500.0,
            if is_current_style {
                t.tag_color
            } else {
                c::muted()
            },
        )
        .truncate()
        .max_w(76.0),
    )
}

fn widgets_page(s: &Settings) -> El {
    let filter = s.category_filter.as_str();

    let cat_options = [
        ("all", "全部小组件"),
        ("music", "🎵 音乐 (汽水音乐)"),
        ("notes", "📝 笔记与 AI"),
        ("clock", "⏰ 时钟与天气"),
        ("system", "💻 系统状态与电量"),
        ("launchers", "🚀 常用应用"),
    ];

    let mut cat_bar = row().gap(6.0).items_center();
    for (id, name) in cat_options {
        let is_sel = filter == id;
        cat_bar = cat_bar.child(
            row()
                .key(format!("cat/{id}"))
                .px(10.0)
                .h(26.0)
                .radius(6.0)
                .items_center()
                .fill(if is_sel { c::accent() } else { c::card() })
                .border(1.0, if is_sel { c::accent() } else { c::line() })
                .hover(c::hover())
                .on_press(move |s: &mut Settings| s.category_filter = id.to_string())
                .child(label(
                    name,
                    11.5,
                    if is_sel { 600.0 } else { 400.0 },
                    if is_sel { Color::WHITE } else { c::muted() },
                )),
        );
    }

    let mut active_tags = row().gap(6.0).items_center().grow(1.0).shrink();
    for wid in &s.active_widgets {
        let title = WIDGET_DEFS
            .iter()
            .find(|w| w.id == wid)
            .map_or(wid.as_str(), |w| w.title);
        let st = s
            .widget_styles
            .get(wid)
            .map(String::as_str)
            .unwrap_or("classic");
        let th = THEMES.iter().find(|t| t.id == st).unwrap_or(&THEMES[0]);
        let wid_copy = wid.clone();
        active_tags = active_tags.child(
            row()
                .key(format!("active/{wid}"))
                .px(7.0)
                .h(24.0)
                .radius(5.0)
                .fill(th.bg_color)
                .border(1.0, th.border_color)
                .items_center()
                .gap(5.0)
                .child(label(
                    format!("{title} · {}", th.name),
                    10.5,
                    500.0,
                    th.accent_color,
                ))
                .child(
                    row()
                        .size(13.0, 13.0)
                        .radius(6.5)
                        .fill(c::red().alpha(0.8))
                        .items_center()
                        .justify_center()
                        .hover(c::red())
                        .on_press(move |s: &mut Settings| {
                            s.toggle_active_widget(&wid_copy);
                        })
                        .child(glyph("x", 8.0, Color::WHITE)),
                ),
        );
    }

    let status_bar = row()
        .w_full()
        .px(12.0)
        .py(8.0)
        .radius(8.0)
        .fill(c::card())
        .border(1.0, c::line())
        .gap(10.0)
        .items_center()
        .child(
            row()
                .gap(5.0)
                .items_center()
                .child(glyph("check", 13.0, c::green()))
                .child(label("当前首页组件：", 11.5, 600.0, c::fg())),
        )
        .child(active_tags)
        .child(button("reset_layout", "重置默认", false, |s| {
            s.reset_widgets_to_defaults()
        }));

    let mut content_col = col().gap(18.0)
        .child(label("所有组件均支持 5x5 大卡与 2x2 方卡双尺寸，并支持 3D黑胶胆机、3D玻璃、纯粹曜黑、克莱因蓝、甜心马卡龙、莫兰迪灰、纯色极简、炫彩霓虹 8 大主题皮肤。点击组件卡片可一键添加/移除或更换皮肤。", 12.0, 400.0, c::muted()).max_lines(2))
        .child(cat_bar)
        .child(status_bar);

    for w in &WIDGET_DEFS {
        if filter != "all" && filter != w.category {
            continue;
        }

        let is_active = s.active_widgets.contains(&w.id.to_string());
        let cur_style = s
            .widget_styles
            .get(w.id)
            .map(String::as_str)
            .unwrap_or("classic");

        let row1_5x5 = row()
            .gap(10.0)
            .items_center()
            .child(card_5x5(
                &format!("{}/vinyl", w.id),
                w,
                &THEMES[0],
                is_active,
                cur_style == "vinyl",
            ))
            .child(card_5x5(
                &format!("{}/classic", w.id),
                w,
                &THEMES[1],
                is_active,
                cur_style == "classic",
            ))
            .child(card_5x5(
                &format!("{}/dark", w.id),
                w,
                &THEMES[2],
                is_active,
                cur_style == "dark",
            ))
            .child(card_5x5(
                &format!("{}/klein", w.id),
                w,
                &THEMES[3],
                is_active,
                cur_style == "klein",
            ));

        let row2_5x5 = row()
            .gap(10.0)
            .items_center()
            .child(card_5x5(
                &format!("{}/macaron", w.id),
                w,
                &THEMES[4],
                is_active,
                cur_style == "macaron",
            ))
            .child(card_5x5(
                &format!("{}/muted", w.id),
                w,
                &THEMES[5],
                is_active,
                cur_style == "muted",
            ))
            .child(card_5x5(
                &format!("{}/solid", w.id),
                w,
                &THEMES[6],
                is_active,
                cur_style == "solid",
            ))
            .child(card_5x5(
                &format!("{}/vibrant", w.id),
                w,
                &THEMES[7],
                is_active,
                cur_style == "vibrant",
            ));

        let mut row_2x2 = row().gap(8.0).items_center();
        for t in &THEMES {
            row_2x2 = row_2x2.child(card_2x2(
                &format!("{}/2x2/{}", w.id, t.id),
                w,
                t,
                is_active,
                cur_style == t.id,
            ));
        }

        let section = col()
            .gap(10.0)
            .child(
                row()
                    .gap(8.0)
                    .items_center()
                    .child(glyph(w.icon, 16.0, c::accent()))
                    .child(label(w.title, 15.0, 600.0, c::fg()))
                    .child(label(w.desc, 12.0, 400.0, c::muted())),
            )
            .child(
                label(
                    format!("{} · 5x5 大卡系列 (多功能大卡)", w.title),
                    11.5,
                    500.0,
                    c::muted(),
                )
                .ml(4.0),
            )
            .child(row1_5x5)
            .child(row2_5x5)
            .child(
                label(
                    format!("{} · 2x2 方卡系列 (方形/独立组件)", w.title),
                    11.5,
                    500.0,
                    c::muted(),
                )
                .ml(4.0)
                .mt(4.0),
            )
            .child(row_2x2);

        content_col = content_col.child(section);
    }

    content_col
}

// Subpages ------------------------------------------------------------------------------

const LOCATION: [(&str, &str); 2] = [("live", "这台 Mac 的位置"), ("set", "自选地点")];
const HOVER: [(&str, &str); 3] = [
    ("peek", "悬停预览"),
    ("open", "悬停展开"),
    ("none", "仅点击展开"),
];
const DISPLAY: [(&str, &str); 3] = [
    ("notch", "MacBook 刘海屏"),
    ("main", "主显示器"),
    ("pointer", "鼠标所在的显示器"),
];
const BACKGROUND: [(&str, &str); 4] = [
    ("black", "纯黑色"),
    ("frosted", "毛玻璃"),
    ("liquid", "液态玻璃"),
    ("faux", "仿液态"),
];

fn set_hover_delay(s: &mut Settings, value: f32, _: bool) {
    s.set_island(
        "hoverDelayMs",
        Value::from((100.0 + value * 900.0).round() as u64),
    );
}

fn set_content_padding(s: &mut Settings, value: f32, _: bool) {
    s.set_island("contentPadding", json!((8.0 + value * 32.0).round()));
}

fn set_background(s: &mut Settings, value: &str) {
    let value = value.to_owned();
    s.edit_layout(move |layout| {
        if !layout["island"].is_object() {
            layout["island"] = json!({});
        }
        layout["island"]["backgroundStyle"] = Value::from(value.as_str());
        layout["island"]["glass"] = Value::from(matches!(value.as_str(), "frosted" | "liquid"));
    });
}

fn pick_background(s: &mut Settings, value: &str) {
    set_background(s, value);
}

fn general(s: &Settings) -> El {
    let hover = s
        .island("hover")
        .and_then(Value::as_str)
        .unwrap_or("peek")
        .to_owned();
    let hover_delay = s
        .island("hoverDelayMs")
        .and_then(Value::as_f64)
        .unwrap_or(120.0) as f32;
    let display = s
        .island("display")
        .and_then(Value::as_str)
        .unwrap_or("notch")
        .to_owned();
    let hide_full = s
        .island("hideInFullScreen")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let haptics = s.island("haptics").and_then(Value::as_bool).unwrap_or(true);
    let dolls = s.island("dolls").and_then(Value::as_bool).unwrap_or(true);
    let dolls_active_only = s
        .island("dollsActiveOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let dolls_avoid_cursor = s
        .island("dollsAvoidCursor")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    col()
        .gap(18.0)
        .child(card(vec![
            choice(
                "hover",
                "mouse",
                "灵动岛展开方式",
                &HOVER,
                &hover,
                |s, id| s.set_island("hover", Value::from(id)),
            ),
            setting(
                "hover-delay",
                "timer",
                "悬停触发延迟",
                Some(&format!("{:.2} 秒", hover_delay / 1000.0)),
                slider(
                    "hover-delay",
                    (hover_delay - 100.0) / 900.0,
                    set_hover_delay,
                ),
            ),
            choice(
                "display",
                "monitor",
                "显示器",
                &DISPLAY,
                &display,
                |s, id| s.set_island("display", Value::from(id)),
            ),
            toggle(
                "fullscreen",
                "eye",
                "全屏时隐藏",
                Some("应用在该显示器上全屏显示时，灵动岛会暂时隐藏"),
                hide_full,
                move |s| s.set_island("hideInFullScreen", Value::from(!hide_full)),
            ),
            toggle(
                "haptics",
                "activity",
                "触感反馈",
                Some("双指下滑展开灵动岛时，触控板会轻点反馈"),
                haptics,
                move |s| s.set_island("haptics", Value::from(!haptics)),
            ),
            toggle(
                "dolls",
                "bot",
                "刘海吊绳玩偶",
                Some("在刘海下方挂出 Codex 与 MAP 模型小助手"),
                dolls,
                move |s| s.set_island("dolls", Value::from(!dolls)),
            ),
            toggle(
                "dolls-active-only",
                "activity",
                "平时完全藏进刘海",
                Some("开启后空闲时完全隐藏，关闭则露出小脑袋和眼睛偷看"),
                dolls_active_only,
                move |s| s.set_island("dollsActiveOnly", Value::from(!dolls_active_only)),
            ),
            toggle(
                "dolls-avoid-cursor",
                "mouse",
                "靠近玩偶自动让路",
                Some("鼠标移到玩偶上时自动淡出并缩短挂绳"),
                dolls_avoid_cursor,
                move |s| s.set_island("dollsAvoidCursor", Value::from(!dolls_avoid_cursor)),
            ),
            toggle("login", "laptop", "登录时启动", None, s.login, |s| {
                let want = !s.login;
                match nook_core::login_item::set_enabled(want) {
                    Ok(()) => s.login = want,
                    Err(e) => s.note = Some((format!("设置登录时启动失败：{e}"), false)),
                }
            }),
        ]))
        .child(location(s))
}

fn appearance_page(s: &Settings) -> El {
    let background = s
        .island("backgroundStyle")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if s.island("glass").and_then(Value::as_bool).unwrap_or(true) {
                "liquid"
            } else {
                "faux"
            }
            .to_owned()
        });
    let tint = s.island("glassTint").and_then(Value::as_f64).unwrap_or(0.5) as f32;
    let width = s.island("width").and_then(Value::as_f64).unwrap_or(560.0) as f32;
    let content_padding = s
        .island("contentPadding")
        .and_then(Value::as_f64)
        .unwrap_or(16.0) as f32;
    col().gap(18.0).child(card(vec![
            theme_choice(s),
        choice(
            "background-style",
            "layers",
            "灵动岛背景",
            &BACKGROUND,
            &background,
            pick_background,
        ),
        if background == "liquid" {
            setting(
                "tint",
                "droplets",
                "顶部黑色渐变",
                Some("调整黑色区域向下覆盖玻璃的范围"),
                slider("tint", tint, |s, v, _| {
                    s.set_island("glassTint", json!((v * 100.0).round() / 100.0))
                }),
            )
        } else {
            el()
        },
        setting(
            "width",
            "grip-vertical",
            "展开宽度",
            Some(&format!("{} 点", width.round())),
            slider("width", (width - 460.0) / 300.0, |s, v, _| {
                s.set_island("width", json!((460.0 + v * 300.0).round()))
            }),
        ),
        setting(
            "content-padding",
            "grip-vertical",
            "展开内容留白",
            Some(&format!("{} 点", content_padding.round())),
            slider(
                "content-padding",
                (content_padding - 8.0) / 32.0,
                set_content_padding,
            ),
        ),
        ]))
}

fn system_stats_page(_: &Settings) -> El {
    col().gap(18.0).child(card(vec![
        setting(
            "stat/cpu",
            "activity",
            "CPU 与内存监控",
            Some("实时显示处理器占用率与系统内存使用比"),
            el(),
        ),
        setting(
            "stat/disk",
            "gauge",
            "磁盘剩余容量",
            Some("监控主分区可用存储空间与占用比例"),
            el(),
        ),
        setting(
            "stat/battery",
            "battery-charging",
            "电池与充电状态",
            Some("显示当前电池电量百分比、充电指示与健康状态"),
            el(),
        ),
    ]))
}

fn notes_page(_: &Settings) -> El {
    col().gap(18.0).child(card(vec![
        setting(
            "note/memo",
            "notebook",
            "便签备忘录",
            Some("在灵动岛常驻显示临时备忘与待办清单"),
            el(),
        ),
        setting(
            "note/clip",
            "files",
            "剪贴板历史记录",
            Some("保存最近复制的文本与代码片段，支持快捷点击回填"),
            el(),
        ),
    ]))
}

fn launchers_page(_: &Settings) -> El {
    col().gap(18.0).child(card(vec![
        setting(
            "launch/dock",
            "layout-grid",
            "常用应用启动抽屉",
            Some("在灵动岛底部快捷启动终端、代码助手与音乐播放器"),
            el(),
        ),
        setting(
            "launch/order",
            "grip-vertical",
            "应用图标顺序",
            Some("支持在 dashboard 模块中自由排列与自定义"),
            el(),
        ),
    ]))
}

fn theme_choice(s: &Settings) -> El {
    let dir = home::app_dir();
    let current = crate::theme::current_name(s.layout.get("theme").unwrap_or(&Value::Null));
    let options: Vec<(String, String)> = crate::theme::names(&dir)
        .into_iter()
        .map(|n| {
        let title = match n.as_str() {
            "default" => "默认".to_owned(),
            "graphite" => "石墨".to_owned(),
            "midnight" => "午夜".to_owned(),
            "ember" => "余烬".to_owned(),
            "forest" => "森林".to_owned(),
            "rose" => "玫瑰".to_owned(),
            _ => n.replace(['-', '_'], " "),
        };
        (n, title)
        })
        .collect();
    let shown = options
        .iter()
        .find(|(id, _)| *id == current)
        .map_or_else(|| current.clone(), |(_, t)| t.clone());
    let end = row()
        .key("theme/value")
        .gap(6.0)
        .items_center()
        .child(label(shown, 14.5, 600.0, c::fg()))
        .child(glyph("chevron-right", 14.0, c::muted()));
    setting(
        "theme",
        "palette",
        "主题",
        Some("灵动岛和所有模块使用的全局配色"),
        end,
    )
    .hover(c::hover())
        .on_press_tx(move |_: &mut Settings, tx: &mut Tx| {
            let menu = PopupMenu::picker(options.clone(), &current);
            tx.popup_menu(menu, "theme/value", |s: &mut Settings, picked, _| {
                if let Some(name) = picked {
                    let name = name.to_owned();
                s.edit_layout(move |l| {
                    l["theme"] = crate::theme::set(&l["theme"], Some(&name), None)
                });
                }
            });
        })
}

fn location(s: &Settings) -> El {
    let current = s.layout.get("location");
    let mode = current
        .and_then(|l| l.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("live")
        .to_owned();
    let name = current
        .and_then(|l| l.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut rows = vec![choice(
        "location",
        "map-pin",
        "位置",
        &LOCATION,
        &mode,
        |s, id| s.set_location_mode(id),
    )];
    if mode == "set" {
        let detail = match &name {
            Some(n) => format!("当前：{n}"),
            None => "输入城市名称，然后点击“设置”".into(),
        };
        let field = text_input(&s.place, font(14.0, 400.0))
            .key("place")
            .on_edit(|s: &mut Settings| &mut s.place)
            .on_submit(|s: &mut Settings, _: &mut Tx| s.find_place())
            .placeholder("城市名称", c::faint())
            .color(c::fg())
            .w(180.0)
            .h(32.0)
            .padding(8.0)
            .radius(8.0)
            .fill(c::card())
            .border(1.0, c::line());
        rows.push(setting(
            "place",
            "search",
            "地点",
            Some(&detail),
            row().gap(8.0).items_center().child(field).child(button(
                "place/set",
                "设置",
                true,
                |s| s.find_place(),
            )),
        ));
    }
    let mut out = col().gap(8.0).child(card(rows));
    if let Some((msg, ok)) = &s.place_note {
        out = out.child(label(msg, 12.5, 500.0, if *ok { c::muted() } else { c::red() }).ml(4.0));
    }
    out
}

fn permission(
    key: &str,
    glyph_name: &str,
    title: &str,
    why: &str,
    access: Option<Access>,
    ask: Option<fn()>,
    pane: &'static str,
) -> El {
    let (state, color) = match access {
        Some(Access::Allowed) => ("已允许", c::green()),
        Some(Access::Denied) => ("未允许", c::red()),
        Some(Access::NotAsked) => ("尚未请求", c::muted()),
        None => ("", c::muted()),
    };
    let action = match (access, ask) {
        (Some(Access::NotAsked), Some(ask)) => {
            button(&format!("{key}/ask"), "允许…", true, move |_| ask())
        }
        _ => button(
            &format!("{key}/open"),
            "打开系统设置",
            false,
            move |_| {
                let _ = open::that_detached(pane);
            },
        ),
    };
    setting(
        key,
        glyph_name,
        title,
        Some(why),
        row()
            .gap(12.0)
            .items_center()
            .child(label(state, 13.0, 500.0, color))
            .child(action),
    )
}

fn permissions(s: &Settings) -> El {
    let dev = s.asked_for.is_some();
    let rows = vec![
        permission(
            "perm/audio",
            "volume-2",
            "系统音频录制",
            "根据正在播放的声音显示音量，不会录制或保存音频内容。",
            Some(s.audio_access),
            (!dev).then_some(nook_core::permissions::request_audio_capture as fn()),
            nook_core::permissions::AUDIO_CAPTURE_PANE,
        ),
        permission(
            "perm/location",
            "map-pin",
            "定位服务",
            "天气、世界时钟和国际空间站地图会使用你的位置；也可以在“通用”中手动指定地点。",
            Some(s.location_access),
            (!dev).then_some(ask_location as fn()),
            nook_core::permissions::LOCATION_PANE,
        ),
        permission(
            "perm/automation",
            "zap",
            "自动化",
            "当媒体按键无法控制音乐应用时，用于控制 Apple Music 和 Spotify。",
            None,
            None,
            nook_core::permissions::AUTOMATION_PANE,
        ),
    ];
    let mut page = col().gap(14.0).child(card(rows));
    if let Some(app) = &s.asked_for {
        page = page.child(label(
            format!("这是由 {app} 启动的开发版本，因此 macOS 会使用 {app} 的权限。若要使用音频音量或定位功能，请在系统设置中允许 {app}（音频权限位于“屏幕与系统录音”）。从 openNook.app 启动时会单独请求权限。"),
            12.5, 400.0, c::muted()).max_lines(4).ml(4.0));
    }
    page
}

fn ask_location() {
    drop(nook_core::location::begin_request());
}

fn mods(s: &Settings) -> El {
    let rows: Vec<El> = s
        .mods
        .iter()
        .map(|m| {
        let name = m.name.clone();
        let shown = !s.hidden(&m.name);
            let detail = if m.error {
                Some(format!("运行异常：mods/{}.error", m.name))
            } else {
                m.description.clone()
            };
            let mut words =
                col()
                    .gap(2.0)
                    .grow(1.0)
                    .shrink()
                    .child(label(&m.title, 14.5, 500.0, c::fg()));
        if let Some(d) = detail {
                words = words.child(
                    label(d, 12.5, 400.0, if m.error { c::red() } else { c::muted() }).truncate(),
                );
        }
            row()
                .key(format!("mod/{}", m.name))
                .min_h(ROW_H)
                .w_full()
                .px(16.0)
                .py(10.0)
                .gap(12.0)
                .items_center()
            .hover(c::hover())
            .on_press(move |s: &mut Settings| s.toggle_hidden(&name))
            .child(row().w(22.0).justify_center().child(mod_icon(m)))
            .child(words)
            .child(switch(&format!("mod/{}", m.name), shown))
        })
        .collect();
    let mut page = col().gap(18.0);
    page = if rows.is_empty() {
        page.child(label("还没有模块。", 14.0, 400.0, c::muted()))
    } else {
        page.child(card(rows))
    };
    page.child(
        row()
            .gap(10.0)
            .child(button(
                "mods/folder",
                "打开模块文件夹",
                false,
                |_| {
                    let _ = open::that_detached(home::app_dir().join("mods"));
                },
            ))
            .child(button(
                "mods/guide",
                "打开 AGENTS.md 指南",
                false,
                |_| {
                    let _ = open::that_detached(home::app_dir().join("AGENTS.md"));
                },
            )),
    )
}

fn yours(s: &Settings) -> El {
    let agent_name = s
        .agents
        .iter()
        .find(|(c, _)| *c == s.agent)
        .map_or("你的编程助手", |a| a.1);
    let mut col_ = col()
        .gap(16.0)
        .child(
            label(
                "openNook 由小型脚本模块组成。描述你希望在刘海屏显示什么，然后交给编程助手来创建。",
                14.0,
                400.0,
                c::muted(),
            )
            .max_lines(3),
        )
        .child(
            text_input(&s.wish, font(14.0, 400.0))
                .key("wish")
                .on_edit(|s: &mut Settings| &mut s.wish)
            .placeholder("例如：显示我未合并的 GitHub PR 和番茄钟", c::faint())
                .color(c::fg())
                .w_full()
                .h(96.0)
                .padding(12.0)
                .radius(10.0)
                .fill(c::card())
                .border(1.0, c::line()),
        );
    let mut actions = row().gap(10.0).items_center();
    if s.herdr && !s.agents.is_empty() {
        actions = actions.child(button(
            "start",
            &format!("在 Herdr 中用 {agent_name} 启动"),
            true,
            |s| s.start_in_herdr(),
        ));
    }
    actions = actions.child(button("copy", "复制任务说明", !s.herdr, |s| {
        let prompt = s.prompt();
        let copied = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
            use std::io::Write;
            c.stdin.take().unwrap().write_all(prompt.as_bytes())?;
            c.wait().map(|_| ())
        });
        s.note = Some(match copied {
            Ok(()) => ("已复制，可粘贴到任意编程助手中。".into(), true),
            Err(e) => (format!("复制失败：{e}"), false),
        });
    }));
    if s.agents.len() > 1 {
        let options: Vec<(&'static str, &'static str)> = s.agents.clone();
        let current = s.agent;
        actions = actions.child(
            row()
                .key("agent")
                .gap(4.0)
                .px(10.0)
                .h(32.0)
                .radius(8.0)
                .items_center()
                .hover(c::hover())
            .child(label(agent_name, 13.5, 500.0, c::muted()))
            .child(glyph("chevron-right", 12.0, c::muted()))
            .on_press_tx(move |_: &mut Settings, tx: &mut Tx| {
                let menu = PopupMenu::picker(options.clone(), current);
                tx.popup_menu(menu, "agent", |s: &mut Settings, picked, _| {
                        if let Some(a) =
                            picked.and_then(|id| s.agents.iter().find(|(c, _)| *c == id))
                        {
                        s.agent = a.0;
                    }
                });
                }),
        );
    }
    col_ = col_.child(actions);
    if let Some((msg, ok)) = &s.note {
        col_ = col_.child(label(
            msg,
            13.0,
            500.0,
            if *ok { c::green() } else { c::red() },
        ));
    }
    col_.child(card(vec![setting(
        "setup",
        "terminal",
        "配置编程助手",
        Some("为 Claude Code 安装 openNook 技能，并将 `opennook` 添加到 PATH"),
        button("setup/go", "开始配置", false, |s| {
            s.note = Some(match set_up_agents() {
                Ok(m) => (m, true),
                Err(e) => (format!("配置失败：{e}"), false),
            });
        }),
    )]))
}

fn about(_: &Settings) -> El {
    col().gap(18.0).child(card(vec![
        setting(
            "version",
            "info",
            "openNook",
            Some(&format!("版本 {}", env!("CARGO_PKG_VERSION"))),
            el(),
        ),
        setting(
            "repo",
            "github",
            "源代码",
            Some("github.com/prodBirdy/openNook"),
            button("repo/open", "打开", false, |_| {
                let _ = open::that_detached("https://github.com/prodBirdy/openNook");
            }),
        ),
        setting(
            "home",
            "folder",
            "openNook 数据目录",
            Some("~/.config/opennook"),
            button("home/open", "显示", false, |_| {
                let _ = open::that_detached(home::app_dir());
            }),
        ),
    ]))
}

fn sidebar(s: &Settings) -> El {
    let mut bar = col()
        .key("sidebar")
        .w(SIDEBAR)
        .h_full()
        .fill(c::sidebar())
        .pt(48.0)
        .px(10.0)
        .gap(2.0);
    for (page, title, glyph_name) in PAGES {
        let selected = s.page == page;
        let mut item = row()
            .key(format!("nav/{title}"))
            .h(36.0)
            .w_full()
            .px(10.0)
            .gap(10.0)
            .radius(8.0)
            .items_center()
            .hover(c::hover())
            .on_press(move |s: &mut Settings| s.page = page)
            .child(glyph(
                glyph_name,
                16.0,
                if selected { c::fg() } else { c::muted() },
            ))
            .child(label(
                title,
                14.5,
                500.0,
                if selected { c::fg() } else { c::muted() },
            ));
        if selected {
            item = item.fill(c::selected());
        }
        bar = bar.child(item);
    }
    bar.child(el().grow(1.0)).child(
        label(
            format!("版本 {}", env!("CARGO_PKG_VERSION")),
            12.0,
            400.0,
            c::faint(),
        )
        .px(10.0)
        .pb(16.0),
    )
}

fn content(s: &Settings, title: &str, body: El) -> El {
    col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .h_full()
        .padding(8.0)
        .child(
            col()
                .key("panel")
                .w_full()
                .min_w(0.0)
                .h_full()
                .radius(14.0)
                .fill(c::panel())
                .border(1.0, c::line())
                .px(24.0)
                .pt(16.0)
                .pb(20.0)
                .gap(14.0)
            .child(notch_preview(s))
            .child(label(title, 17.0, 600.0, c::fg()))
                .child(
                    col()
                        .key("scroll")
                        .w_full()
                        .grow(1.0)
                        .min_h(0.0)
                        .scroll(rune::ScrollSpec {
                            stick_bottom: false,
                        })
                        .scroll_edge_fade(18.0)
                        .child(
                            body.key(format!("page/{title}"))
                                .pb(8.0)
                                .transition(rune::Transition::opacity()),
                        ),
                ),
    )
}

fn onboarding(s: &Settings, step: u8) -> El {
    let body = if step == 0 {
        col()
            .gap(14.0)
            .items_center()
            .w(460.0)
            .child(label("欢迎使用 openNook", 26.0, 700.0, c::fg()))
            .child(
                label(
                    "用模块打造你的刘海屏。悬停可预览，点击或双指下滑可展开，左右滑动可切换页面。",
                    15.0,
                    400.0,
                    c::muted(),
                )
                .max_lines(4)
                .align(rune::Align::Center),
            )
            .child(el().h(12.0))
            .child(button("next", "继续", true, |s| s.onboarding = Some(1)))
    } else {
        col()
            .gap(14.0)
            .w(560.0)
            .child(label("自定义你的灵动岛", 26.0, 700.0, c::fg()))
            .child(yours(s))
            .child(row().w_full().justify_end().pt(8.0).child(button(
                "done",
                "完成",
                true,
                |s| s.finish_onboarding(),
            )))
    };
    col()
        .key("onboarding")
        .w_full()
        .h_full()
        .fill(c::panel())
        .items_center()
        .justify_center()
        .child(
            body.key(format!("step/{step}"))
                .transition(rune::Transition::opacity()),
        )
}

pub fn view(s: &Settings, _: &mut Cx<Settings>) -> El {
    if let Some(step) = s.onboarding {
        return onboarding(s, step);
    }
    let (title, body) = match s.page {
        Page::Widgets => ("可用小组件", widgets_page(s)),
        Page::General => ("通用", general(s)),
        Page::Appearance => ("外观", appearance_page(s)),
        Page::SystemStats => ("系统状态", system_stats_page(s)),
        Page::Notes => ("笔记与剪贴板", notes_page(s)),
        Page::Launchers => ("常用应用", launchers_page(s)),
        Page::Permissions => ("权限", permissions(s)),
        Page::Mods => ("模块管理", mods(s)),
        Page::Yours => ("自定义", yours(s)),
        Page::About => ("关于", about(s)),
    };
    row()
        .key("settings")
        .w_full()
        .h_full()
        .fill(c::sidebar())
        .child(sidebar(s))
        .child(content(s, title, body))
}
