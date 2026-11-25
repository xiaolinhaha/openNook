use crate::island::Island;
use rune::tok::TextStyle;
use rune::{
    Color, Cursor, DragItem, DragOutcome, El, PopupItem, PopupMenu, Tx, col, el, icon, row, text,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
    #[serde(default)]
    added_at: u64,
    #[serde(default)]
    pinned: bool,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Sort {
    #[default]
    Recent,
    Name,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Document {
    entries: Vec<Entry>,
    sort: Sort,
    remove_after_drag: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Stored {
    Current(Document),
    Legacy(Vec<Entry>),
}

#[derive(Default)]
pub struct Shelf {
    document: Document,
    selected: BTreeSet<PathBuf>,
    multi_select: bool,
    dragging: Vec<PathBuf>,
    hovered: Option<PathBuf>,
    menu_anchor: (f32, f32),
    error: Option<String>,
}

impl Shelf {
    pub fn add(&mut self, paths: &[PathBuf]) {
        self.reload();
        if self.error.is_some() {
            return;
        }
        for path in paths {
            if path.exists()
                && !self
                    .document
                    .entries
                    .iter()
                    .any(|entry| entry.path == *path)
            {
                self.document.entries.push(Entry {
                    path: path.clone(),
                    added_at: now(),
                    pinned: false,
                });
            }
        }
        self.selected = paths.iter().filter(|path| path.exists()).cloned().collect();
        persist(self);
    }

    pub fn reload(&mut self) {
        let fresh = load();
        if fresh.error.is_none() {
            self.document = fresh.document;
            self.selected.retain(|path| {
                self.document
                    .entries
                    .iter()
                    .any(|entry| entry.path == *path)
            });
        }
        self.error = fresh.error;
    }

    fn remove(&mut self, paths: &[PathBuf]) {
        self.reload();
        if self.error.is_some() {
            return;
        }
        self.document
            .entries
            .retain(|entry| !paths.contains(&entry.path));
        self.selected.retain(|path| !paths.contains(path));
        persist(self);
    }

    fn select(&mut self, path: &Path) {
        if self.multi_select {
            if !self.selected.remove(path) {
                self.selected.insert(path.to_path_buf());
            }
        } else {
            self.selected.clear();
            self.selected.insert(path.to_path_buf());
        }
    }

    fn ordered(&self) -> Vec<&Entry> {
        let mut entries: Vec<_> = self.document.entries.iter().collect();
        entries.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| match self.document.sort {
                    Sort::Recent => b.added_at.cmp(&a.added_at),
                    Sort::Name => name(&a.path)
                        .to_lowercase()
                        .cmp(&name(&b.path).to_lowercase()),
                })
        });
        entries
    }

    fn drag_paths(&self, origin: Option<&Path>) -> Vec<PathBuf> {
        self.document
            .entries
            .iter()
            .filter(|entry| {
                entry.path.exists()
                    && match origin {
                        Some(path) if self.selected.contains(path) => {
                            self.selected.contains(&entry.path)
                        }
                        Some(path) => entry.path == path,
                        None => true,
                    }
            })
            .map(|entry| entry.path.clone())
            .collect()
    }

    fn finish_drag(&mut self, outcome: DragOutcome) {
        let paths = std::mem::take(&mut self.dragging);
        if self.document.remove_after_drag && outcome != DragOutcome::Cancelled {
            self.reload();
            if self.error.is_some() {
                return;
            }
            let removable: Vec<_> = paths
                .into_iter()
                .filter(|path| {
                    self.document
                        .entries
                        .iter()
                        .any(|entry| entry.path == *path && !entry.pinned)
                })
                .collect();
            self.remove(&removable);
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn storage() -> PathBuf {
    crate::home::app_dir().join("file-shelf.json")
}

fn persist(shelf: &mut Shelf) {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec_pretty(&shelf.document)?;
        let path = storage();
        std::fs::create_dir_all(crate::home::app_dir())?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    })();
    shelf.error = result.err().map(|e| format!("保存失败，{e}"));
}

pub fn load() -> Shelf {
    let result = std::fs::read(storage());
    match result {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(stored) => {
                let document = match stored {
                    Stored::Current(document) => document,
                    Stored::Legacy(entries) => Document {
                        entries,
                        ..Default::default()
                    },
                };
                Shelf {
                    document,
                    ..Default::default()
                }
            }
            Err(e) => Shelf {
                error: Some(format!("读取失败，{e}")),
                ..Default::default()
            },
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Shelf::default(),
        Err(e) => Shelf {
            error: Some(format!("读取失败，{e}")),
            ..Default::default()
        },
    }
}

fn glyph(name: &str, size: f32, color: Color) -> El {
    match crate::icons::get(name, Path::new("")) {
        Some(shape) => icon(shape, size).color(color),
        None => el().size(size, size),
    }
}

pub fn tab_icon(island: &Island, selected: bool) -> El {
    let source = format!("app-file:{}", crate::home::app_dir().display());
    match island
        .images
        .as_ref()
        .and_then(|images| images.get(&source, Path::new(""), 16.0))
    {
        Some(image) => el()
            .size(16.0, 16.0)
            .pixels_fit(image.pixels(), rune::SurfaceFit::Fit),
        None => glyph(
            "folder",
            16.0,
            island
                .theme
                .color(if selected { "accent" } else { "muted" }),
        ),
    }
}

fn drag_item(paths: Vec<PathBuf>) -> Option<DragItem> {
    if paths.is_empty() {
        return None;
    }
    Some(
        paths
            .into_iter()
            .fold(DragItem::new(), |item, path| item.file(path)),
    )
}

fn tile(island: &Island, entry: &Entry, width: f32) -> El {
    let shelf = &island.file_shelf;
    let path = entry.path.clone();
    let selected = shelf.selected.contains(&path);
    let exists = path.exists();
    let fg = island.theme.color("fg");
    let muted = island.theme.color("muted");
    let accent = island.theme.color("accent");
    let image_source = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => path.to_string_lossy().into_owned(),
        _ => format!("app-file:{}", path.display()),
    };
    let picture = island
        .images
        .as_ref()
        .and_then(|images| images.get(&image_source, Path::new(""), 52.0));
    let picture = match picture {
        Some(image) => el()
            .size(52.0, 52.0)
            .radius(8.0)
            .pixels_fit(image.pixels(), rune::SurfaceFit::Fit),
        None => glyph(if path.is_dir() { "folder" } else { "files" }, 42.0, muted),
    };
    let selection = path.clone();
    let opened = path.clone();
    let context = path.clone();
    let source = path.clone();
    let drag_source = path.clone();
    let hovered = path.clone();
    let mut card = col()
        .key(format!("shelf/tile/{}", path.display()))
        .w(width)
        .h(106.0)
        .padding(8.0)
        .label(name(&path))
        .gap(7.0)
        .items_center()
        .radius(13.0)
        .fill(if selected {
            accent.alpha(0.14)
        } else {
            Color::BLACK.alpha(0.0)
        })
        .border(
            1.0,
            if selected {
                accent.alpha(0.55)
            } else {
                Color::BLACK.alpha(0.0)
            },
        )
        .hover(island.theme.color("raised"))
        .cursor(Cursor::Grab)
        .opacity(if !exists {
            0.45
        } else if shelf.dragging.contains(&path) {
            0.35
        } else {
            1.0
        })
        .child(el().h(54.0).w_full().center().child(picture))
        .child(
            text(name(&path))
                .font(TextStyle::new(11.0, 500.0))
                .color(fg)
                .max_lines(2)
                .help_when_truncated(name(&path)),
        )
        .on_press(move |s: &mut Island| s.file_shelf.select(&selection))
        .on_double_press_tx(move |s: &mut Island, tx| {
            act(s, "open", std::slice::from_ref(&opened), tx)
        })
        .on_hover_change(move |s: &mut Island, entered, _| {
            if entered {
                s.file_shelf.hovered = Some(hovered.clone());
            } else if s.file_shelf.hovered.as_ref() == Some(&hovered) {
                s.file_shelf.hovered = None;
            }
        })
        .on_secondary_press(move |s: &mut Island, at, tx| {
            if !s.file_shelf.selected.contains(&context) {
                s.file_shelf.selected.clear();
                s.file_shelf.selected.insert(context.clone());
            }
            menu(s, (at.x, at.y), tx);
        })
        .drag_out(move |s: &Island| drag_item(s.file_shelf.drag_paths(Some(&source))))
        .on_drag_start(move |s: &mut Island, _| {
            s.file_shelf.dragging = s.file_shelf.drag_paths(Some(&drag_source));
            s.mod_dragging = true;
        })
        .on_drag_end(|s: &mut Island, outcome, _| {
            s.mod_dragging = false;
            s.file_shelf.finish_drag(outcome);
        });
    if entry.pinned {
        card = card.child(glyph("pin", 11.0, accent).pin(None, Some(5.0), Some(5.0), None));
    }
    card
}

fn action_button(island: &Island, key: &str, symbol: &str, label: &str) -> El {
    let fg = island.theme.color("fg");
    let mut button = row()
        .key(format!("shelf/action/{key}"))
        .h(28.0)
        .px(9.0)
        .gap(5.0)
        .items_center()
        .radius(9.0)
        .hover(island.theme.color("raised"))
        .cursor(Cursor::Pointer)
        .child(glyph(symbol, 13.0, fg));
    if !label.is_empty() {
        button = button.child(text(label).font(TextStyle::new(11.0, 500.0)).color(fg));
    }
    let accessible_label = match key {
        "add" => "添加文件或文件夹",
        "menu" => "文件操作与整理",
        _ => label,
    };
    button = button.label(accessible_label).help(accessible_label);
    if key == "menu" {
        button = button.on_pointer_move(|s: &mut Island, at| {
            s.file_shelf.menu_anchor = (at.x, at.y);
        });
    }
    let action = key.to_owned();
    button.on_press_tx(move |s: &mut Island, tx| {
        let paths: Vec<_> = s.file_shelf.selected.iter().cloned().collect();
        act(s, &action, &paths, tx);
    })
}

fn footer(island: &Island) -> El {
    let shelf = &island.file_shelf;
    let paths = shelf.drag_paths(None);
    let mut info = col().grow(1.0).gap(3.0);
    let fg = island.theme.color("fg");
    let muted = island.theme.color("muted");
    if shelf.selected.len() == 1 {
        let path = shelf.selected.first().unwrap();
        let detail = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => "文件夹".to_owned(),
            Ok(metadata) => file_size(metadata.len()),
            Err(_) => "文件已移动或删除".to_owned(),
        };
        let added = shelf
            .document
            .entries
            .iter()
            .find(|entry| entry.path == *path)
            .map_or(0, |entry| entry.added_at);
        let detail = if added > 0 {
            format!("{detail} · {}", age(added))
        } else {
            detail
        };
        info = info
            .child(
                text(name(path))
                    .font(TextStyle::new(11.0, 500.0))
                    .color(fg)
                    .truncate(),
            )
            .child(text(detail).font(TextStyle::new(10.0, 400.0)).color(muted));
    } else {
        let label = if shelf.selected.is_empty() {
            format!("{} 个文件", shelf.document.entries.len())
        } else {
            format!("已选 {} 项", shelf.selected.len())
        };
        info = info.child(text(label).font(TextStyle::new(11.0, 500.0)).color(muted));
    }
    let mut bar = row().w_full().h(38.0).gap(5.0).items_center().child(info);
    if shelf.selected.len() == 1 && shelf.selected.first().is_some_and(|path| path.exists()) {
        bar = bar.child(action_button(island, "preview", "eye", "预览"));
    }
    if !shelf.selected.is_empty() {
        bar = bar.child(action_button(island, "remove", "xmark", "移出"));
    }
    let count = paths.len();
    let label = format!("拖走全部 {count}");
    if count > 0 {
        bar = bar.child(
            row()
                .key("shelf/drag-all")
                .h(28.0)
                .px(10.0)
                .gap(6.0)
                .items_center()
                .radius(9.0)
                .fill(island.theme.color("surface"))
                .hover(island.theme.color("raised"))
                .cursor(Cursor::Grab)
                .child(glyph("files", 13.0, fg))
                .child(text(label).font(TextStyle::new(11.0, 500.0)).color(fg))
                .drag_out(|s: &Island| drag_item(s.file_shelf.drag_paths(None)))
                .on_drag_start(|s: &mut Island, _| {
                    s.file_shelf.dragging = s.file_shelf.drag_paths(None);
                    s.mod_dragging = true;
                })
                .on_drag_end(|s: &mut Island, outcome, _| {
                    s.mod_dragging = false;
                    s.file_shelf.finish_drag(outcome);
                }),
        );
    }
    bar.child(action_button(
        island,
        "multi-select",
        "check",
        if shelf.multi_select {
            "完成"
        } else {
            "多选"
        },
    ))
    .child(action_button(island, "add", "plus", ""))
    .child(action_button(island, "menu", "ellipsis", ""))
}

pub fn island_view(island: &Island, width: f32) -> El {
    let shelf = &island.file_shelf;
    let entries = shelf.ordered();
    let width = width.max(250.0);
    let columns = ((width + 10.0) / 100.0).floor().max(1.0) as usize;
    let tile_width = (width - (columns - 1) as f32 * 10.0) / columns as f32;
    let mut grid = col().w_full().gap(8.0);
    for chunk in entries.chunks(columns) {
        let mut line = row().w_full().gap(10.0);
        for entry in chunk {
            line = line.child(tile(island, entry, tile_width));
        }
        grid = grid.child(line);
    }
    if entries.is_empty() {
        grid = grid.child(
            col()
                .w_full()
                .h(118.0)
                .center()
                .gap(9.0)
                .child(glyph(
                    "arrow-down-to-line",
                    26.0,
                    island.theme.color("muted"),
                ))
                .child(
                    text(if island.file_drop_hover {
                        "松手，放进暂存"
                    } else {
                        "拖入文件，随时取用"
                    })
                    .font(TextStyle::new(12.0, 400.0))
                    .color(island.theme.color("muted")),
                ),
        );
    }
    let rows = entries.len().div_ceil(columns).clamp(1, 2);
    let height = rows as f32 * 114.0 + 8.0;
    let mut page = col()
        .key("file-shelf/page")
        .w_full()
        .gap(6.0)
        .child(
            col()
                .key("file-shelf/list")
                .w_full()
                .h(height)
                .radius(15.0)
                .border(
                    1.0,
                    if island.file_drop_hover {
                        island.theme.color("accent").alpha(0.6)
                    } else {
                        Color::BLACK.alpha(0.0)
                    },
                )
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(grid.key("file-shelf/list#content")),
        )
        .child(footer(island));
    if let Some(error) = &shelf.error {
        page = page.child(
            text(error.clone())
                .font(TextStyle::new(11.0, 400.0))
                .color(island.theme.color("red")),
        );
    }
    page
}

fn file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} 字节")
    } else if bytes < 1024 * 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn age(added: u64) -> String {
    let elapsed = now().saturating_sub(added);
    match elapsed {
        0..60 => "刚刚加入".into(),
        60..3600 => format!("{} 分钟前加入", elapsed / 60),
        3600..86400 => format!("{} 小时前加入", elapsed / 3600),
        _ => format!("{} 天前加入", elapsed / 86400),
    }
}

pub fn context_menu(island: &mut Island, at: (f32, f32), tx: &mut Tx) {
    if let Some(path) = island.file_shelf.hovered.clone() {
        if !island.file_shelf.selected.contains(&path) {
            island.file_shelf.selected.clear();
            island.file_shelf.selected.insert(path);
        }
    }
    menu(island, at, tx);
}

fn menu(island: &mut Island, at: (f32, f32), tx: &mut Tx) {
    island.context_menu_open = true;
    let shelf = &island.file_shelf;
    let paths: Vec<_> = shelf.selected.iter().cloned().collect();
    let single = paths.len() == 1;
    let available = single && paths[0].exists();
    let pinned = single
        && shelf
            .document
            .entries
            .iter()
            .any(|entry| entry.path == paths[0] && entry.pinned);
    let menu = PopupMenu::new()
        .item(
            PopupItem::new("open", "打开")
                .enabled(available)
                .symbol("arrow.up.forward.square"),
        )
        .item(
            PopupItem::new("preview", "快速预览")
                .enabled(available)
                .symbol("eye"),
        )
        .item(
            PopupItem::new("reveal", "在 Finder 中显示")
                .enabled(available)
                .symbol("folder"),
        )
        .item(
            PopupItem::new("airdrop", "隔空投送")
                .enabled(paths.iter().all(|path| path.exists()) && !paths.is_empty())
                .symbol("airdrop"),
        )
        .item(
            PopupItem::new("share", "分享…")
                .enabled(paths.iter().all(|path| path.exists()) && !paths.is_empty())
                .symbol("square.and.arrow.up"),
        )
        .item(
            PopupItem::new("copy-files", "拷贝文件")
                .enabled(paths.iter().all(|path| path.exists()) && !paths.is_empty())
                .symbol("doc.on.doc"),
        )
        .item(
            PopupItem::new("compress", "压缩")
                .enabled(paths.iter().all(|path| path.exists()) && !paths.is_empty())
                .symbol("document.zip"),
        )
        .item(
            PopupItem::new("copy-path", "复制路径")
                .enabled(!paths.is_empty())
                .symbol("doc.on.doc"),
        )
        .separator()
        .item(
            PopupItem::new("pin", if pinned { "取消置顶" } else { "置顶" })
                .enabled(single)
                .checked(pinned)
                .symbol("pin"),
        )
        .item(
            PopupItem::new("remove", "移出暂存")
                .enabled(!paths.is_empty())
                .symbol("minus.circle"),
        )
        .separator()
        .item(PopupItem::new("select-all", "全选").enabled(!shelf.document.entries.is_empty()))
        .item(PopupItem::new("multi-select", "多选模式").checked(shelf.multi_select))
        .item(
            PopupItem::new("sort-recent", "按加入时间排序")
                .checked(shelf.document.sort == Sort::Recent),
        )
        .item(PopupItem::new("sort-name", "按名称排序").checked(shelf.document.sort == Sort::Name))
        .item(
            PopupItem::new("remove-after-drag", "拖出成功后移除未置顶项")
                .checked(shelf.document.remove_after_drag),
        )
        .separator()
        .item(PopupItem::new("clean-missing", "清理失效条目"))
        .item(PopupItem::new("clear", "清空未置顶项"));
    tx.popup_menu(menu, at, move |s: &mut Island, picked, tx| {
        s.context_menu_open = false;
        if let Some(action) = picked {
            s.keep_open_for(Duration::from_secs(3));
            act(s, action, &paths, tx);
        }
    });
}

fn act(island: &mut Island, action: &str, paths: &[PathBuf], tx: &mut Tx) {
    let first = paths
        .first()
        .map(|path| path.to_string_lossy().into_owned());
    let result = match action {
        "open" => first.map(nook_core::files::open_file),
        "preview" => first.map(|path| nook_core::files::quick_look(&path)),
        "reveal" => first.map(|path| nook_core::files::reveal(&path)),
        "airdrop" => Some(native_file_action("airdrop", paths)),
        "share" => Some(native_file_action("share", paths)),
        "copy-files" => Some(native_file_action("copy", paths)),
        "compress" => Some(compress(paths)),
        "copy-path" => {
            tx.copy(
                paths
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            None
        }
        "menu" => {
            menu(island, island.file_shelf.menu_anchor, tx);
            None
        }
        "add" => {
            if let Some(add) = &island.add_shelf_files {
                add();
            }
            None
        }
        "remove" => {
            island.file_shelf.remove(paths);
            None
        }
        "select-all" => {
            island.file_shelf.selected = island
                .file_shelf
                .document
                .entries
                .iter()
                .map(|entry| entry.path.clone())
                .collect();
            island.file_shelf.multi_select = true;
            None
        }
        "multi-select" => {
            island.file_shelf.multi_select = !island.file_shelf.multi_select;
            None
        }
        "pin" => {
            island.file_shelf.reload();
            if island.file_shelf.error.is_some() {
                return;
            }
            if let Some(entry) = island
                .file_shelf
                .document
                .entries
                .iter_mut()
                .find(|entry| paths.contains(&entry.path))
            {
                entry.pinned = !entry.pinned;
                persist(&mut island.file_shelf);
            }
            None
        }
        "sort-name" | "sort-recent" | "remove-after-drag" => {
            island.file_shelf.reload();
            if island.file_shelf.error.is_none() {
                match action {
                    "sort-name" => island.file_shelf.document.sort = Sort::Name,
                    "sort-recent" => island.file_shelf.document.sort = Sort::Recent,
                    _ => {
                        island.file_shelf.document.remove_after_drag =
                            !island.file_shelf.document.remove_after_drag
                    }
                }
                persist(&mut island.file_shelf);
            }
            None
        }
        "clean-missing" | "clear" => {
            island.file_shelf.reload();
            let removed: Vec<_> = island
                .file_shelf
                .document
                .entries
                .iter()
                .filter(|entry| {
                    if action == "clear" {
                        !entry.pinned
                    } else {
                        !entry.path.exists()
                    }
                })
                .map(|entry| entry.path.clone())
                .collect();
            island.file_shelf.remove(&removed);
            None
        }
        _ => None,
    };
    if let Some(Err(error)) = result {
        island.file_shelf.error = Some(error);
    }
}

fn compress(paths: &[PathBuf]) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let first = paths.first().ok_or("没有选中文件")?;
    let parent = first.parent().ok_or("无法确定文件所在目录")?;
    let stem = if paths.len() == 1 {
        first
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    } else {
        "Archive".into()
    };
    let mut output = parent.join(format!("{stem}.zip"));
    let mut suffix = 2;
    while output.exists() {
        output = parent.join(format!("{stem} {suffix}.zip"));
        suffix += 1;
    }
    let mut command = Command::new("/usr/bin/ditto");
    command.args(["-c", "-k", "--sequesterRsrc", "--keepParent"]);
    for path in paths {
        command.arg(path);
    }
    let result = command
        .arg(&output)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| error.to_string())?;
    if result.status.success() {
        Ok(())
    } else {
        let _ = std::fs::remove_file(&output);
        Err(String::from_utf8_lossy(&result.stderr).trim().to_owned())
    }
}

#[cfg(target_os = "macos")]
fn native_file_action(action: &str, paths: &[PathBuf]) -> Result<(), String> {
    mac_file_actions::perform(action, paths)
}

#[cfg(not(target_os = "macos"))]
fn native_file_action(_: &str, _: &[PathBuf]) -> Result<(), String> {
    Err("此操作仅支持 macOS".into())
}

#[cfg(target_os = "macos")]
mod mac_file_actions {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use std::ffi::CString;
    use std::path::PathBuf;

    pub(super) fn perform(action: &str, paths: &[PathBuf]) -> Result<(), String> {
        unsafe {
            let pool: *mut AnyObject = msg_send![class!(NSAutoreleasePool), new];
            let urls: Vec<*mut AnyObject> = paths
                .iter()
                .map(|path| {
                    let path = CString::new(path.to_string_lossy().as_bytes()).ok()?;
                    let string: *mut AnyObject =
                        msg_send![class!(NSString), stringWithUTF8String: path.as_ptr()];
                    if string.is_null() {
                        return None;
                    }
                    let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: string];
                    (!url.is_null()).then_some(url)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| "无法读取所选文件路径".to_owned())?;
            let items: *mut AnyObject =
                msg_send![class!(NSArray), arrayWithObjects: urls.as_ptr(), count: urls.len()];
            let result = match action {
                "copy" => {
                    let pasteboard: *mut AnyObject =
                        msg_send![class!(NSPasteboard), generalPasteboard];
                    let _: isize = msg_send![pasteboard, clearContents];
                    let copied: bool = msg_send![pasteboard, writeObjects: items];
                    if copied {
                        Ok(())
                    } else {
                        Err("无法将文件放入剪贴板".into())
                    }
                }
                "airdrop" => {
                    let service_name = CString::new("com.apple.share.AirDrop.send").unwrap();
                    let name: *mut AnyObject =
                        msg_send![class!(NSString), stringWithUTF8String: service_name.as_ptr()];
                    let service: *mut AnyObject =
                        msg_send![class!(NSSharingService), sharingServiceNamed: name];
                    if service.is_null() {
                        Err("隔空投送服务不可用".into())
                    } else {
                        let _: () = msg_send![service, performWithItems: items];
                        Ok(())
                    }
                }
                "share" => {
                    let picker: *mut AnyObject = msg_send![class!(NSSharingServicePicker), alloc];
                    let picker: *mut AnyObject = msg_send![picker, initWithItems: items];
                    let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
                    let window: *mut AnyObject = msg_send![app, keyWindow];
                    if picker.is_null() || window.is_null() {
                        Err("无法打开系统分享菜单".into())
                    } else {
                        let view: *mut AnyObject = msg_send![window, contentView];
                        if view.is_null() {
                            Err("无法打开系统分享菜单".into())
                        } else {
                            #[repr(C)]
                            struct Point {
                                x: f64,
                                y: f64,
                            }
                            #[repr(C)]
                            struct Size {
                                width: f64,
                                height: f64,
                            }
                            #[repr(C)]
                            struct Rect {
                                origin: Point,
                                size: Size,
                            }
                            unsafe impl objc2::Encode for Point {
                                const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
                                    "CGPoint",
                                    &[f64::ENCODING, f64::ENCODING],
                                );
                            }
                            unsafe impl objc2::Encode for Size {
                                const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
                                    "CGSize",
                                    &[f64::ENCODING, f64::ENCODING],
                                );
                            }
                            unsafe impl objc2::Encode for Rect {
                                const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
                                    "CGRect",
                                    &[Point::ENCODING, Size::ENCODING],
                                );
                            }
                            let bounds: Rect = msg_send![view, bounds];
                            let _: () = msg_send![picker, showRelativeTo: bounds, ofView: view, preferredEdge: 3_isize];
                            let _: *mut AnyObject = msg_send![picker, autorelease];
                            Ok(())
                        }
                    }
                }
                _ => Err("不支持的文件操作".into()),
            };
            let _: () = msg_send![pool, drain];
            result
        }
    }
}

pub fn choose_files(accept: impl Fn(Vec<PathBuf>) + 'static) {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};
        let panel: *mut AnyObject = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel, setAllowsMultipleSelection: true];
        let _: () = msg_send![panel, setCanChooseDirectories: true];
        let _: () = msg_send![panel, setCanChooseFiles: true];
        let callback = block2::RcBlock::new(move |response: isize| {
            if response != 1 {
                return;
            }
            let urls: *mut AnyObject = msg_send![panel, URLs];
            let count: usize = msg_send![urls, count];
            let mut paths = Vec::new();
            for index in 0..count {
                let url: *mut AnyObject = msg_send![urls, objectAtIndex: index];
                let path: *mut AnyObject = msg_send![url, path];
                if path.is_null() {
                    continue;
                }
                let utf8: *const std::ffi::c_char = msg_send![path, UTF8String];
                if !utf8.is_null() {
                    paths.push(PathBuf::from(
                        std::ffi::CStr::from_ptr(utf8)
                            .to_string_lossy()
                            .into_owned(),
                    ));
                }
            }
            accept(paths);
        });
        let _: () = msg_send![panel, beginWithCompletionHandler: &*callback];
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = accept;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(path: PathBuf, pinned: bool, added_at: u64) -> Entry {
        Entry {
            path,
            pinned,
            added_at,
        }
    }

    #[test]
    fn legacy_shelf_records_gain_defaults_without_losing_paths() {
        let stored: Stored = serde_json::from_str(r#"[{"path":"/tmp/报告.xlsx"}]"#).unwrap();
        let Stored::Legacy(entries) = stored else {
            panic!("expected legacy records");
        };
        assert_eq!(entries[0].path, PathBuf::from("/tmp/报告.xlsx"));
        assert!(!entries[0].pinned);
        assert_eq!(entries[0].added_at, 0);
        let document = Document {
            entries,
            remove_after_drag: true,
            sort: Sort::Name,
        };
        let bytes = serde_json::to_vec(&document).unwrap();
        let Stored::Current(restored) = serde_json::from_slice(&bytes).unwrap() else {
            panic!("expected current document");
        };
        assert_eq!(restored.entries.len(), 1);
        assert!(restored.remove_after_drag);
        assert!(restored.sort == Sort::Name);
    }

    #[test]
    fn pinned_entries_stay_first_when_sorting_by_name_or_time() {
        let mut shelf = Shelf::default();
        shelf.document.entries = vec![
            sample_entry("b.txt".into(), false, 10),
            sample_entry("z.txt".into(), true, 1),
            sample_entry("a.txt".into(), false, 20),
        ];
        let paths = |s: &Shelf| {
            s.ordered()
                .into_iter()
                .map(|entry| name(&entry.path))
                .collect::<Vec<_>>()
        };
        assert_eq!(paths(&shelf), ["z.txt", "a.txt", "b.txt"]);
        shelf.document.sort = Sort::Name;
        assert_eq!(paths(&shelf), ["z.txt", "a.txt", "b.txt"]);
    }

    #[test]
    fn group_drag_respects_selection_and_cancellation_keeps_entries() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let first = root.join("Cargo.toml");
        let second = root.join("src/main.rs");
        let mut shelf = Shelf::default();
        shelf.document.entries = vec![
            sample_entry(first.clone(), false, 1),
            sample_entry(second.clone(), false, 2),
        ];
        assert_eq!(shelf.drag_paths(None).len(), 2);
        shelf.select(&first);
        assert_eq!(shelf.drag_paths(None).len(), 2);
        assert_eq!(shelf.drag_paths(Some(&second)), [second.clone()]);
        shelf.multi_select = true;
        shelf.select(&second);
        assert_eq!(shelf.drag_paths(Some(&first)).len(), 2);
        shelf.dragging = shelf.drag_paths(None);
        shelf.document.remove_after_drag = true;
        shelf.finish_drag(DragOutcome::Cancelled);
        assert_eq!(shelf.document.entries.len(), 2);
        assert!(shelf.dragging.is_empty());
    }
    #[test]
    fn grid_click_selects_a_file_without_opening_a_separate_window() {
        use crate::home::Layout;
        use crate::island::{Notch, view};
        use rune::{Env, Runtime};
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let notch = Notch {
            w: 200.0,
            h: 32.0,
            has: true,
            screen: (1512.0, 982.0),
        };
        let mut island = Island::new(notch, Layout::default());
        island.file_shelf.document.entries = vec![sample_entry(path.clone(), false, 1)];
        island.expanded = true;
        island.selected = Some("file-shelf".into());
        let size = (island.canvas_w(), crate::island::CANVAS_H);
        let mut runtime = Runtime::new(island, view, size, Env::default());
        for _ in 0..30 {
            runtime.frame(1.0 / 60.0);
        }
        let (x, y) = runtime
            .find(&format!("shelf/tile/{}", path.display()))
            .unwrap();
        runtime.pointer_move(x, y);
        runtime.pointer_down(x, y);
        runtime.pointer_up(x, y);
        runtime.frame(1.0 / 60.0);
        assert!(runtime.state.file_shelf.selected.contains(&path));
        assert!(runtime.find("shelf/drag-all").is_some());
        assert!(runtime.find("shelf/action/preview").is_some());
        assert!(runtime.find("shelf/action/remove").is_some());
    }

    #[test]
    fn right_clicking_a_file_opens_its_context_menu_once() {
        use crate::home::Layout;
        use crate::island::{Notch, view};
        use rune::{Env, Runtime};
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let notch = Notch {
            w: 200.0,
            h: 32.0,
            has: true,
            screen: (1512.0, 982.0),
        };
        let mut island = Island::new(notch, Layout::default());
        island.file_shelf.document.entries = vec![sample_entry(path.clone(), false, 1)];
        island.expanded = true;
        island.selected = Some("file-shelf".into());
        let size = (island.canvas_w(), crate::island::CANVAS_H);
        let mut runtime = Runtime::new(island, view, size, Env::default());
        for _ in 0..30 {
            runtime.frame(1.0 / 60.0);
        }
        let (x, y) = runtime
            .find(&format!("shelf/tile/{}", path.display()))
            .unwrap();
        runtime.pointer_move(x, y);
        runtime.secondary_down(x, y);
        runtime.secondary_up(x, y);
        runtime.frame(1.0 / 60.0);
        assert!(runtime.popup_menu().is_some());
        assert!(runtime.state.context_menu_open);
        runtime.frame(1.0 / 60.0);
        assert!(
            runtime.popup_menu().is_some(),
            "the opening click must not dismiss the menu"
        );
        runtime.resolve_popup(None).unwrap();
        assert!(!runtime.state.context_menu_open);
    }
}
