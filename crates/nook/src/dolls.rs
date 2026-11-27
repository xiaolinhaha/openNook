use rune::{
    Color, Cursor, Cycle, Drag, Easing, El, LoopSpec, Phase, PopupItem, PopupMenu, Spring, Tx,
    col, el, text,
};
use rune::tok::TextStyle;
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const CORD_PEEK: f32 = 0.0;
const CORD_HOVER_PEEK: f32 = 4.0;
const CORD_WAKE: f32 = 20.0;
const CORD_SUCCEEDED: f32 = 28.0;
const CORD_FAILED: f32 = 20.0;
const CORD_AVOID: f32 = 0.0;
const MAX_PULL_DY: f32 = 52.0;

const SLOT_W: f32 = 32.0;
const PEEK_H: f32 = 18.0;
const FULL_H: f32 = 28.0;
const SLOT_GAP: f32 = 56.0;
const CLUSTER_W: f32 = 300.0;

const BLINK_STOPS: &[(f32, f32)] = &[
    (0.0, 1.0),
    (0.86, 1.0),
    (0.90, 0.0),
    (0.94, 0.0),
    (0.98, 1.0),
    (1.0, 1.0),
];
const BREATHE_EASE: Easing = Easing::Piecewise(&[[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]]);
const LOOK_AROUND_EASE: Easing = Easing::Piecewise(&[
    [0.0, 0.5],
    [0.24, 0.5],
    [0.32, 0.04],
    [0.54, 0.04],
    [0.62, 0.96],
    [0.84, 0.96],
    [0.92, 0.5],
    [1.0, 0.5],
]);

const SUCCESS_TTL: Duration = Duration::from_secs(12);
const WAKE_TTL: Duration = Duration::from_secs(4);
const POKE_TTL: Duration = Duration::from_secs(3);
const BOUNCE_TTL: Duration = Duration::from_millis(95);

const CORD_SPRING: Spring = Spring::new(0.42, 0.50);

const THINKING_PHRASES: [&str; 4] = [
    "想一想...",
    "翻代码中...",
    "在忙呢别闹~",
    "马上就好啦",
];

const HAPPY_PHRASES: [&str; 3] = [
    "嘿嘿~",
    "搞定啦！",
    "今天真顺利~",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DollId {
    Codex,
    MapGpt,
    MapGemini,
}

impl DollId {
    fn key(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::MapGpt => "map-gpt",
            Self::MapGemini => "map-gemini",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::MapGpt => "GPT",
            Self::MapGemini => "Gemini",
        }
    }

    fn palette(self) -> (Color, Color, Color) {
        match self {
            Self::Codex => (
                rgb(0x38, 0xbd, 0xf8),
                rgb(0x02, 0x84, 0xc7),
                rgb(0x0f, 0x17, 0x2a),
            ),
            Self::MapGpt => (
                rgb(0xf5, 0x9e, 0x0b),
                rgb(0xd9, 0x77, 0x06),
                rgb(0x18, 0x11, 0x02),
            ),
            Self::MapGemini => (
                rgb(0x81, 0x8c, 0xf8),
                rgb(0x63, 0x66, 0xf1),
                rgb(0x11, 0x18, 0x27),
            ),
        }
    }

    fn text_color(self, phase: DollPhase) -> Color {
        match phase {
            DollPhase::Failed => rgb(0xfc, 0xa5, 0xa5),
            DollPhase::Succeeded => rgb(0xfe, 0xf0, 0x8a),
            DollPhase::Thinking => match self {
                Self::Codex => rgb(0xe0, 0xf2, 0xfe),
                Self::MapGpt => rgb(0xfe, 0xf3, 0xc7),
                Self::MapGemini => rgb(0xe0, 0xe7, 0xff),
            },
            DollPhase::Idle => rgb(0xe0, 0xe7, 0xff),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DollPhase {
    Idle,
    Thinking,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug)]
pub struct DollState {
    pub id: DollId,
    pub phase: DollPhase,
    backend_phase: DollPhase,
    pub model_name: String,
    pub pid: Option<u32>,
    pub completion_msg: Option<String>,
    seen_completions: HashSet<String>,
    completions_primed: bool,
    success_until: Option<Instant>,
    woken_until: Option<Instant>,
    poke_msg: Option<(String, Instant)>,
    poke_index: usize,
    pull_dy: f32,
    dragging: bool,
    bounce_until: Option<Instant>,
    hovered: bool,
}

impl DollState {
    fn new(id: DollId) -> Self {
        Self {
            id,
            phase: DollPhase::Idle,
            backend_phase: DollPhase::Idle,
            model_name: id.default_model().to_owned(),
            pid: None,
            completion_msg: None,
            seen_completions: HashSet::new(),
            completions_primed: false,
            success_until: None,
            woken_until: None,
            poke_msg: None,
            poke_index: 0,
            pull_dy: 0.0,
            dragging: false,
            bounce_until: None,
            hovered: false,
        }
    }

    fn is_woken(&self, now: Instant) -> bool {
        self.woken_until.is_some_and(|until| now < until)
    }

    fn is_dropped(&self, now: Instant) -> bool {
        matches!(self.phase, DollPhase::Succeeded | DollPhase::Failed)
            || self.is_woken(now)
            || self.dragging
    }

    fn active_in_working_only(&self, now: Instant) -> bool {
        self.phase != DollPhase::Idle
            || self.is_woken(now)
            || self.poke_msg.as_ref().is_some_and(|(_, u)| now < *u)
            || self.dragging
    }

    fn cord_height(&self, now: Instant, avoid_cursor: bool) -> f32 {
        if avoid_cursor && self.hovered && !self.dragging {
            return CORD_AVOID;
        }
        let base = if !self.is_dropped(now) {
            if self.hovered {
                CORD_HOVER_PEEK
            } else {
                CORD_PEEK
            }
        } else {
            match self.phase {
                DollPhase::Succeeded => CORD_SUCCEEDED,
                DollPhase::Failed => CORD_FAILED,
                DollPhase::Idle | DollPhase::Thinking => CORD_WAKE,
            }
        };
        let bounce = if self.bounce_until.is_some_and(|until| now < until) {
            8.0
        } else {
            0.0
        };
        base + self.pull_dy + bounce
    }

    fn trigger_bounce(&mut self, now: Instant) {
        self.bounce_until = Some(now + BOUNCE_TTL);
    }

    fn next_wakeup(&self) -> Option<Instant> {
        [
            self.success_until,
            self.woken_until,
            self.poke_msg.as_ref().map(|(_, u)| *u),
            self.bounce_until,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    fn set_phase(&mut self, next: DollPhase, now: Instant, msg: Option<String>) -> bool {
        let prev_backend = self.backend_phase;
        self.backend_phase = next;
        let mut changed = false;
        match next {
            DollPhase::Succeeded => {
                if msg.is_some() {
                    self.completion_msg = msg;
                }
                if prev_backend != DollPhase::Succeeded {
                    self.phase = DollPhase::Succeeded;
                    self.success_until = Some(now + SUCCESS_TTL);
                    self.trigger_bounce(now);
                    changed = true;
                }
            }
            DollPhase::Thinking => {
                if self.phase != DollPhase::Thinking {
                    self.phase = DollPhase::Thinking;
                    self.success_until = None;
                    self.woken_until = None;
                    changed = true;
                }
            }
            DollPhase::Failed => {
                if self.phase != DollPhase::Failed {
                    self.phase = DollPhase::Failed;
                    self.success_until = None;
                    changed = true;
                }
            }
            DollPhase::Idle => {
                // Keep showing Succeeded until its local display timer expires.
                if self.phase == DollPhase::Succeeded && self.success_until.is_some_and(|u| now < u)
                {
                    return false;
                }
                if self.phase != DollPhase::Idle {
                    self.phase = DollPhase::Idle;
                    self.success_until = None;
                    changed = true;
                }
            }
        }
        changed
    }

    fn expire(&mut self, now: Instant) -> bool {
        let mut changed = false;
        if self.bounce_until.is_some_and(|until| now >= until) {
            self.bounce_until = None;
            changed = true;
        }
        if self.woken_until.is_some_and(|until| now >= until) {
            self.woken_until = None;
            changed = true;
        }
        if self
            .poke_msg
            .as_ref()
            .is_some_and(|(_, until)| now >= *until)
        {
            self.poke_msg = None;
            changed = true;
        }
        if self.success_until.is_some_and(|until| now >= until) {
            self.success_until = None;
            if self.phase == DollPhase::Succeeded {
                self.phase = DollPhase::Idle;
            }
            changed = true;
        }
        changed
    }

    fn display_text(&self, now: Instant) -> Option<String> {
        if let Some((msg, until)) = &self.poke_msg
            && now < *until
        {
            return Some(msg.clone());
        }
        match self.phase {
            DollPhase::Idle if self.is_woken(now) => Some("唔...被戳醒了!".into()),
            DollPhase::Idle => None,
            DollPhase::Thinking => Some(if self.hovered {
                format!("{} · 想一想...", short_model(&self.model_name))
            } else {
                "想一想...".into()
            }),
            DollPhase::Succeeded => {
                if self.hovered
                    && let Some(msg) = &self.completion_msg
                {
                    return Some(format!("搞定 · {}", truncate_chars(msg, 18)));
                }
                if let Some(msg) = &self.completion_msg
                    && !msg.is_empty()
                {
                    return Some(format!("搞定 · {}", truncate_chars(msg, 10)));
                }
                Some("搞定啦！".into())
            }
            DollPhase::Failed => Some("咦？停了".into()),
        }
    }
}

pub struct Dolls {
    pub items: [DollState; 3],
}

impl Default for Dolls {
    fn default() -> Self {
        Self::new()
    }
}

impl Dolls {
    pub fn new() -> Self {
        Self {
            items: [
                DollState::new(DollId::Codex),
                DollState::new(DollId::MapGpt),
                DollState::new(DollId::MapGemini),
            ],
        }
    }

    fn slot(&self, id: DollId) -> usize {
        match id {
            DollId::Codex => 0,
            DollId::MapGpt => 1,
            DollId::MapGemini => 2,
        }
    }

    pub fn get(&self, id: DollId) -> &DollState {
        &self.items[self.slot(id)]
    }

    pub fn get_mut(&mut self, id: DollId) -> &mut DollState {
        &mut self.items[self.slot(id)]
    }

    pub fn apply_codex(&mut self, codex: &Value, agents: &Value, tx: &mut Tx) {
        let now = Instant::now();
        self.apply_codex_at(codex, agents, now);
        self.schedule_wakeup(now, tx);
    }

    pub fn apply_map(&mut self, map: &Value, tx: &mut Tx) {
        let now = Instant::now();
        self.apply_map_at(map, now);
        self.schedule_wakeup(now, tx);
    }

    pub fn apply_codex_at(&mut self, codex: &Value, agents: &Value, now: Instant) {
        let mut codex_pid = None;
        let mut codex_model = None;
        if let Some(list) = agents.as_array() {
            for item in list {
                let kind = item["kind"].as_str().unwrap_or_default();
                if kind.eq_ignore_ascii_case("codex") {
                    if codex_pid.is_none() || item["status"] == "working" {
                        codex_pid = item["pid"].as_u64().map(|p| p as u32);
                    }
                    if let Some(m) = item["model"].as_str().filter(|m| !m.is_empty()) {
                        codex_model = Some(format_model(m));
                    }
                }
            }
        }

        let doll = self.get_mut(DollId::Codex);
        doll.pid = codex_pid;
        if let Some(m) = codex_model {
            doll.model_name = m;
        }

        let fresh_msg = take_fresh_completion(doll, codex.get("completions"), |c| {
            let id = c["id"].as_str()?.to_owned();
            let msg = c["message"]
                .as_str()
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(str::to_owned);
            Some((id, msg))
        });

        let status = codex["status"].as_str().unwrap_or("offline");
        let next = if fresh_msg.is_some() {
            DollPhase::Succeeded
        } else {
            parse_phase(status)
        };
        if next == DollPhase::Succeeded && fresh_msg.is_some() {
            doll.success_until = Some(now + SUCCESS_TTL);
            doll.trigger_bounce(now);
        }
        doll.set_phase(next, now, fresh_msg.flatten());
    }

    pub fn apply_map_at(&mut self, map: &Value, now: Instant) {
        let models = map.get("models").and_then(Value::as_array);
        let sessions = map.get("sessions").and_then(Value::as_array);

        for (id, family) in [(DollId::MapGpt, "GPT"), (DollId::MapGemini, "Gemini")] {
            let entry = models.and_then(|arr| {
                arr.iter()
                    .find(|m| m["family"].as_str() == Some(family))
            });
            let session_title = sessions.and_then(|arr| {
                arr.iter()
                    .find(|s| {
                        let m = s["model"].as_str().unwrap_or_default().to_ascii_lowercase();
                        let matches_family = if family == "GPT" {
                            m.starts_with("gpt-")
                        } else {
                            m.starts_with("gemini-")
                        };
                        matches_family && s["status"].as_str() == Some("completed")
                    })
                    .and_then(|s| s["title"].as_str())
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned)
            });

            let doll = self.get_mut(id);
            if let Some(entry) = entry {
                if let Some(slug) = entry["model"].as_str().filter(|s| !s.is_empty()) {
                    doll.model_name = format_model(slug);
                }
                let status = entry["status"].as_str().unwrap_or("idle");
                let fresh = take_fresh_completion(doll, map.get("completions"), |c| {
                    let cid = c["id"].as_str()?.to_owned();
                    let m = c["model"].as_str().unwrap_or_default().to_ascii_lowercase();
                    let matches = if family == "GPT" {
                        m.starts_with("gpt-")
                    } else {
                        m.starts_with("gemini-")
                    };
                    matches.then_some((cid, session_title.clone()))
                });
                let next = if fresh.is_some() {
                    DollPhase::Succeeded
                } else {
                    parse_phase(status)
                };
                if next == DollPhase::Succeeded && fresh.is_some() {
                    doll.success_until = Some(now + SUCCESS_TTL);
                    doll.trigger_bounce(now);
                }
                doll.set_phase(next, now, fresh.flatten().or(session_title));
            } else {
                doll.set_phase(DollPhase::Idle, now, None);
            }
        }
    }

    pub fn poll(&mut self, now: Instant, tx: &mut Tx) -> bool {
        let mut changed = false;
        for doll in &mut self.items {
            changed |= doll.expire(now);
        }
        self.schedule_wakeup(now, tx);
        changed
    }

    fn schedule_wakeup(&self, now: Instant, tx: &mut Tx) {
        if let Some(next) = self.items.iter().filter_map(DollState::next_wakeup).min() {
            let delay = next.saturating_duration_since(now);
            tx.wake_after(delay.max(Duration::from_millis(20)));
        }
    }

    pub fn poke(&mut self, id: DollId, tx: &mut Tx) {
        let now = Instant::now();
        self.poke_at(id, now);
        self.schedule_wakeup(now, tx);
    }

    pub fn poke_at(&mut self, id: DollId, now: Instant) {
        let idx = self.slot(id);
        let doll = &mut self.items[idx];
        doll.trigger_bounce(now);
        match doll.phase {
            DollPhase::Idle => {
                doll.woken_until = Some(now + WAKE_TTL);
                doll.poke_msg = Some(("唔...被戳醒了!".into(), now + WAKE_TTL));
            }
            DollPhase::Thinking => {
                let phrase = THINKING_PHRASES[doll.poke_index % THINKING_PHRASES.len()];
                doll.poke_index = doll.poke_index.wrapping_add(1);
                doll.poke_msg = Some((phrase.into(), now + POKE_TTL));
            }
            DollPhase::Succeeded => {
                let msg = doll
                    .completion_msg
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|s| truncate_chars(s, 18))
                    .unwrap_or_else(|| {
                        let p = HAPPY_PHRASES[doll.poke_index % HAPPY_PHRASES.len()];
                        doll.poke_index = doll.poke_index.wrapping_add(1);
                        p.to_owned()
                    });
                doll.poke_msg = Some((msg, now + POKE_TTL));
                // Neighbor chemistry: an idle buddy peeks over curiously.
                let neighbor = (idx + 1) % self.items.len();
                if self.items[neighbor].phase == DollPhase::Idle
                    && !self.items[neighbor].is_woken(now)
                {
                    self.items[neighbor].woken_until = Some(now + Duration::from_secs(3));
                    self.items[neighbor].poke_msg =
                        Some(("👀 这么快!".into(), now + Duration::from_secs(3)));
                }
            }
            DollPhase::Failed => {
                doll.poke_msg = Some(("要去看看吗?".into(), now + POKE_TTL));
            }
        }
    }

    pub fn open_session(&self, id: DollId) {
        match id {
            DollId::Codex => {
                if let Some(pid) = self.get(DollId::Codex).pid {
                    std::thread::spawn(move || nook_core::agents::focus(pid));
                }
            }
            DollId::MapGpt | DollId::MapGemini => {
                std::thread::spawn(|| {
                    let _ = std::process::Command::new("/usr/bin/open")
                        .args(["-b", crate::map_status::BUNDLE_ID])
                        .status();
                });
            }
        }
    }
}

fn take_fresh_completion(
    doll: &mut DollState,
    completions: Option<&Value>,
    extract: impl Fn(&Value) -> Option<(String, Option<String>)>,
) -> Option<Option<String>> {
    let list = completions?.as_array()?;
    if !doll.completions_primed {
        for item in list {
            if let Some((id, _)) = extract(item) {
                doll.seen_completions.insert(id);
            }
        }
        doll.completions_primed = true;
        return None;
    }
    let mut fresh = None;
    for item in list {
        if let Some((id, msg)) = extract(item)
            && doll.seen_completions.insert(id)
            && fresh.is_none()
        {
            fresh = Some(msg);
        }
    }
    fresh
}

fn parse_phase(status: &str) -> DollPhase {
    match status {
        "running" | "working" => DollPhase::Thinking,
        "succeeded" | "completed" => DollPhase::Succeeded,
        "failed" | "interrupted" => DollPhase::Failed,
        _ => DollPhase::Idle,
    }
}

fn format_model(slug: &str) -> String {
    let s = slug.trim();
    if s.is_empty() {
        return "Model".into();
    }
    s.split(['-', '_'])
        .map(|part| {
            if part.eq_ignore_ascii_case("gpt") {
                "GPT".to_owned()
            } else {
                let mut chars = part.chars();
                match chars.next() {
                    Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn short_model(name: &str) -> String {
    truncate_chars(name, 12)
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut chars = s.chars();
    let collected: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{collected}…")
    } else {
        collected
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub fn view(s: &crate::island::Island) -> El {
    let opts = &s.layout.island;
    if !opts.dolls || s.expanded || s.launcher_fullscreen.is_some() {
        return el().size(0.0, 0.0);
    }
    let now = Instant::now();
    let any_shown = s
        .dolls
        .items
        .iter()
        .any(|d| !opts.dolls_active_only || d.active_in_working_only(now));

    let mut cluster = el()
        .key("dolls/cluster")
        .size(CLUSTER_W, 118.0)
        .opacity(if any_shown { 1.0 } else { 0.0 });
    let step = SLOT_W + SLOT_GAP;
    let first_left = (CLUSTER_W - SLOT_W) * 0.5 - step;
    for (slot_idx, doll) in s.dolls.items.iter().enumerate() {
        let slot_left = first_left + slot_idx as f32 * step;
        let shown = !opts.dolls_active_only || doll.active_in_working_only(now);
        cluster = cluster.child(doll_column(
            doll,
            slot_left,
            now,
            shown,
            opts.dolls_avoid_cursor,
        ));
    }
    cluster
}

fn doll_column(doll: &DollState, left: f32, now: Instant, shown: bool, avoid_cursor: bool) -> El {
    let id = doll.id;
    let ghosted = avoid_cursor && doll.hovered && !doll.dragging;
    let dropped = doll.is_dropped(now) && !ghosted;
    let cord_h = if shown {
        doll.cord_height(now, avoid_cursor)
    } else {
        0.0
    };
    let show_cord = shown && cord_h > 0.5;
    let cord_color = rgb(0x52, 0x58, 0x66).alpha(if !show_cord {
        0.0
    } else if ghosted {
        0.35
    } else {
        0.88
    });
    let motion = if doll.dragging {
        Spring::NONE
    } else {
        CORD_SPRING
    };

    let mut sprite = if dropped {
        sprite_el(doll, now)
    } else {
        peek_sprite_el(doll, now)
    };
    if dropped && doll.phase == DollPhase::Succeeded && shown && !doll.dragging && !ghosted {
        sprite = sprite.looping(
            LoopSpec::slide(1.3, [0.0, 0.0], [0.0, 0.08]).easing(BREATHE_EASE),
        );
    }

    let mut body = sprite
        .key(format!("dolls/sprite/{}", id.key()))
        .opacity(if !shown {
            0.0
        } else if ghosted {
            0.24
        } else {
            1.0
        });
    if shown {
        body = body
            .hit()
            .cursor(Cursor::Pointer)
            .on_hover_change(move |s: &mut crate::island::Island, entered, _| {
                s.dolls.get_mut(id).hovered = entered;
            })
            .on_drag(move |s: &mut crate::island::Island, d: &mut Drag, tx| {
                handle_drag(s, id, d, tx);
            })
            .on_secondary_press(move |s: &mut crate::island::Island, press, tx| {
                doll_menu(s, id, (press.x, press.y), tx);
            });
    }

    if shown && let Some(msg) = floating_message(doll, now, dropped) {
        body = body.child(msg);
    }

    // Tuck peeking heads slightly behind the notch rim so only rounded cheeks + eyes poke out.
    let slide_y = if !shown {
        -26.0
    } else if dropped {
        0.0
    } else {
        -4.0
    };

    col()
        .key(format!("dolls/col/{}", id.key()))
        .absolute(left, 0.0)
        .w(SLOT_W)
        .items_center()
        .motion(motion)
        .child(
            el().key(format!("dolls/cord/{}", id.key()))
                .w(1.0)
                .h(cord_h)
                .fill(cord_color),
        )
        .child(el().offset(0.0, slide_y).child(body))
}

fn handle_drag(s: &mut crate::island::Island, id: DollId, d: &mut Drag, tx: &mut Tx) {
    let now = Instant::now();
    match d.phase {
        Phase::Down => {
            d.data = 0.0;
            let doll = s.dolls.get_mut(id);
            doll.dragging = true;
            doll.pull_dy = 0.0;
        }
        Phase::Move => {
            if d.dy.abs() > 3.0 {
                d.data = 1.0;
            }
            let pull = (d.dy.max(0.0) * 0.72).min(MAX_PULL_DY);
            let doll = s.dolls.get_mut(id);
            doll.pull_dy = pull;
            if pull > 22.0 && doll.poke_msg.is_none() {
                doll.poke_msg = Some(("哇！弹起来~".into(), now + POKE_TTL));
            }
            tx.instant();
        }
        Phase::Up => {
            let was_drag = d.data > 0.5;
            let cancelled = d.cancelled();
            let clicks = d.clicks;
            {
                let doll = s.dolls.get_mut(id);
                doll.dragging = false;
                doll.pull_dy = 0.0;
            }
            if !was_drag && !cancelled {
                if clicks >= 2 {
                    s.dolls.open_session(id);
                } else {
                    s.dolls.poke(id, tx);
                }
            } else {
                s.dolls.schedule_wakeup(now, tx);
            }
        }
    }
}

fn doll_menu(s: &mut crate::island::Island, id: DollId, at: (f32, f32), tx: &mut Tx) {
    let active_only = s.layout.island.dolls_active_only;
    let avoid = s.layout.island.dolls_avoid_cursor;
    let open_label = match id {
        DollId::Codex => "打开 Codex 会话",
        DollId::MapGpt | DollId::MapGemini => "打开 MAP",
    };
    let menu = PopupMenu::new()
        .item(PopupItem::new("open", open_label))
        .item(PopupItem::new("poke", "戳一戳小助手"))
        .separator()
        .item(PopupItem::new(
            "active-only",
            if active_only {
                "✓ 平时完全藏进刘海"
            } else {
                "平时完全藏进刘海"
            },
        ))
        .item(PopupItem::new(
            "avoid-cursor",
            if avoid {
                "✓ 靠近自动让路"
            } else {
                "靠近自动让路"
            },
        ))
        .item(PopupItem::new("hide", "隐藏吊绳玩偶"))
        .separator()
        .item(PopupItem::new("settings", "打开设置…"));
    s.context_menu_open = true;
    tx.popup_menu(menu, at, move |s: &mut crate::island::Island, picked, _| {
        s.context_menu_open = false;
        match picked {
            Some("open") => s.dolls.open_session(id),
            Some("poke") => {
                let now = Instant::now();
                s.dolls.poke_at(id, now);
            }
            Some("active-only") => {
                let next = !s.layout.island.dolls_active_only;
                s.layout.island.dolls_active_only = next;
                let _ = crate::home::patch_layout(&serde_json::json!({
                    "island": { "dollsActiveOnly": next }
                }));
            }
            Some("avoid-cursor") => {
                let next = !s.layout.island.dolls_avoid_cursor;
                s.layout.island.dolls_avoid_cursor = next;
                let _ = crate::home::patch_layout(&serde_json::json!({
                    "island": { "dollsAvoidCursor": next }
                }));
            }
            Some("hide") => {
                s.layout.island.dolls = false;
                let _ = crate::home::patch_layout(&serde_json::json!({
                    "island": { "dolls": false }
                }));
            }
            Some("settings") => {
                if let Some(open) = &s.open_settings {
                    open();
                }
            }
            _ => {}
        }
    });
}

fn floating_message(doll: &DollState, now: Instant, dropped: bool) -> Option<El> {
    let label = doll.display_text(now)?;
    let fg = doll.id.text_color(doll.phase);
    let rim = match doll.phase {
        DollPhase::Succeeded => rgb(0xf5, 0x9e, 0x0b).alpha(0.45),
        DollPhase::Failed => rgb(0xf8, 0x71, 0x71).alpha(0.45),
        _ => Color::WHITE.alpha(0.18),
    };
    let style = TextStyle::new(10.0, 600.0);
    let puff = rune::row()
        .absolute(5.0, 0.0)
        .px(5.5)
        .py(1.5)
        .radius(6.5)
        .fill(rgb(0x0b, 0x0e, 0x16).alpha(0.94))
        .border(0.8, rim)
        .items_center()
        .child(text(&label).font(style).color(fg));
    let dot_fill = rgb(0x0b, 0x0e, 0x16).alpha(0.94);
    let (anchor_y, d1_y, d2_y) = if dropped {
        (-6.0, 14.0, 11.2)
    } else {
        (3.5, 9.2, 7.0)
    };
    Some(
        el().absolute(SLOT_W - 2.0, anchor_y)
            .size(134.0, 22.0)
            .child(px(0.0, d1_y, 2.2, 2.2, dot_fill, 1.1).border(0.6, rim))
            .child(px(3.2, d2_y, 3.0, 3.0, dot_fill, 1.5).border(0.6, rim))
            .child(puff),
    )
}

fn rhythm(id: DollId) -> (f32, f32, f64, f64) {
    // Staggered (blink_period, blink_phase, look_period, breath_period) per buddy.
    match id {
        DollId::Codex => (4.1, 0.12, 5.4, 3.2),
        DollId::MapGpt => (4.7, 0.45, 4.6, 3.8),
        DollId::MapGemini => (3.7, 0.78, 6.1, 3.5),
    }
}

fn glossy_eye(x: f32, y: f32, doll: &DollState) -> El {
    let (blink_period, blink_phase, look_period, _) = rhythm(doll.id);
    let faster_look = if doll.phase == DollPhase::Thinking {
        look_period * 0.55
    } else {
        look_period
    };
    let iris = match doll.id {
        DollId::Codex => rgb(0x09, 0x15, 0x26),
        DollId::MapGpt => rgb(0x3b, 0x17, 0x04),
        DollId::MapGemini => rgb(0x1e, 0x1b, 0x4b),
    };
    let lid_color = match doll.id {
        DollId::Codex => rgb(0x7d, 0xd3, 0xfc),
        DollId::MapGpt => rgb(0x45, 0x1a, 0x03),
        DollId::MapGemini => rgb(0x31, 0x2e, 0x81),
    };

    // Moving dark pupil + white anime sparkle highlight inside each sclera socket.
    let pupil_wrap = el()
        .size(5.2, 5.4)
        .looping(
            LoopSpec::slide(faster_look, [-0.22, -0.05], [0.22, 0.08]).easing(LOOK_AROUND_EASE),
        )
        .child(px(1.1, 1.1, 3.0, 3.2, iris, 1.5))
        .child(px(1.4, 1.4, 1.2, 1.2, Color::WHITE, 0.6))
        .child(px(2.8, 2.9, 0.8, 0.8, Color::WHITE.alpha(0.75), 0.4));

    let open_eye = px(0.0, 0.0, 5.2, 5.4, Color::WHITE, 2.6)
        .child(pupil_wrap)
        .looping(LoopSpec::cycle(Cycle {
            period: blink_period,
            phase: blink_phase,
            stops: BLINK_STOPS,
            still: 1.0,
            fps: 24.0,
        }));

    el().absolute(x, y)
        .size(5.2, 5.4)
        .child(px(0.4, 2.6, 4.4, 1.3, lid_color, 0.65))
        .child(open_eye)
}

fn character_head_el(doll: &DollState, now: Instant) -> El {
    let (primary, shade, visor) = doll.id.palette();
    let (_, _, _, breath_period) = rhythm(doll.id);
    let blush = rgb(0xfb, 0x71, 0x85).alpha(0.58);

    let mut head = el().size(SLOT_W, 17.0);

    // Side ears / wings stay visible below the notch rim so each mascot has its own silhouette.
    head = match doll.id {
        DollId::Codex => head
            .child(
                px(1.0, 4.6, 4.2, 6.2, shade, 2.0)
                    .child(px(0.8, 1.0, 1.8, 2.0, rgb(0xba, 0xe6, 0xfd), 0.9)),
            )
            .child(
                px(26.8, 4.6, 4.2, 6.2, shade, 2.0)
                    .child(px(1.6, 1.0, 1.8, 2.0, rgb(0xba, 0xe6, 0xfd), 0.9)),
            ),
        DollId::MapGpt => {
            let ear_l = px(1.2, 4.0, 4.4, 8.6, shade, 2.2)
                .child(px(1.2, 1.4, 2.0, 5.0, rgb(0xfe, 0xf3, 0xc7), 1.0))
                .looping(
                    LoopSpec::slide(breath_period, [0.0, 0.0], [-0.08, 0.12]).easing(BREATHE_EASE),
                );
            let ear_r = px(26.4, 4.0, 4.4, 8.6, shade, 2.2)
                .child(px(1.2, 1.4, 2.0, 5.0, rgb(0xfe, 0xf3, 0xc7), 1.0))
                .looping(
                    LoopSpec::slide(breath_period, [0.0, 0.0], [0.08, 0.12]).easing(BREATHE_EASE),
                );
            head.child(ear_l).child(ear_r)
        }
        DollId::MapGemini => {
            let ear_l = px(1.0, 3.6, 4.2, 10.2, shade, 2.1)
                .child(px(1.1, 1.6, 1.8, 6.4, rgb(0xe0, 0xe7, 0xff), 0.9))
                .looping(
                    LoopSpec::slide(breath_period, [0.0, 0.0], [-0.06, 0.14]).easing(BREATHE_EASE),
                );
            let ear_r = px(26.8, 3.6, 4.2, 10.2, shade, 2.1)
                .child(px(1.3, 1.6, 1.8, 6.4, rgb(0xe0, 0xe7, 0xff), 0.9))
                .looping(
                    LoopSpec::slide(breath_period, [0.0, 0.0], [0.06, 0.14]).easing(BREATHE_EASE),
                );
            let star = px(27.6, 12.2, 2.4, 2.4, rgb(0xe0, 0xe7, 0xff), 1.2)
                .looping(LoopSpec::opacity(2.2, 0.2, 0.35, 1.0));
            head.child(ear_l).child(ear_r).child(star)
        }
    };

    // Chubby rounded head + distinct facial markings.
    head = head.child(px(5.0, 0.5, 22.0, 14.5, primary, 7.2));
    match doll.id {
        DollId::Codex => {
            head = head
                .child(px(6.8, 4.4, 18.4, 6.8, visor, 3.4))
                .child(px(14.9, 11.4, 2.2, 1.4, rgb(0xe0, 0xf2, 0xfe), 0.7));
        }
        DollId::MapGpt => {
            head = head
                .child(px(10.0, 9.2, 12.0, 5.2, rgb(0xfe, 0xf3, 0xc7), 2.6))
                .child(px(15.0, 10.2, 2.0, 1.4, rgb(0x45, 0x1a, 0x03), 0.7))
                .child(px(15.5, 11.4, 1.0, 1.2, rgb(0x45, 0x1a, 0x03), 0.5));
        }
        DollId::MapGemini => {
            head = head
                .child(px(8.2, 4.6, 15.6, 6.2, visor.alpha(0.35), 3.1))
                .child(px(14.8, 11.2, 2.4, 1.3, rgb(0xe0, 0xe7, 0xff), 0.65));
        }
    }

    // Rosy cheeks
    head = head
        .child(px(6.4, 10.4, 3.4, 2.0, blush, 1.0))
        .child(px(22.2, 10.4, 3.4, 2.0, blush, 1.0));

    // Expressive eyes
    match doll.phase {
        DollPhase::Succeeded => head
            .child(px(7.6, 7.6, 1.6, 1.6, Color::WHITE, 0.4))
            .child(px(9.0, 6.2, 2.0, 1.6, Color::WHITE, 0.4))
            .child(px(10.8, 7.6, 1.6, 1.6, Color::WHITE, 0.4))
            .child(px(19.6, 7.6, 1.6, 1.6, Color::WHITE, 0.4))
            .child(px(21.0, 6.2, 2.0, 1.6, Color::WHITE, 0.4))
            .child(px(22.8, 7.6, 1.6, 1.6, Color::WHITE, 0.4)),
        DollPhase::Failed => head
            .child(px(8.2, 6.4, 3.4, 3.4, rgb(0xfc, 0xa5, 0xa5), 1.7))
            .child(px(20.4, 6.4, 3.4, 3.4, rgb(0xfc, 0xa5, 0xa5), 1.7)),
        _ => {
            let _ = now;
            head.child(glossy_eye(8.0, 5.0, doll))
                .child(glossy_eye(18.8, 5.0, doll))
        }
    }
}

fn peek_sprite_el(doll: &DollState, now: Instant) -> El {
    let (primary, shade, _) = doll.id.palette();
    let (_, _, _, breath_period) = rhythm(doll.id);
    let (amp, period) = if doll.phase == DollPhase::Thinking {
        (0.16, 1.35)
    } else {
        (0.11, breath_period)
    };
    let animated_head = character_head_el(doll, now)
        .looping(LoopSpec::slide(period, [0.0, 0.0], [0.0, amp]).easing(BREATHE_EASE));

    // Tiny paws stay clutching onto the notch rim while the head breathes and looks around.
    el().size(SLOT_W, PEEK_H)
        .child(animated_head)
        .child(
            px(3.8, 2.4, 3.4, 3.2, primary, 1.6)
                .child(px(0.9, 1.2, 1.6, 1.2, shade.alpha(0.55), 0.6)),
        )
        .child(
            px(24.8, 2.4, 3.4, 3.2, primary, 1.6)
                .child(px(0.9, 1.2, 1.6, 1.2, shade.alpha(0.55), 0.6)),
        )
}

fn sprite_el(doll: &DollState, now: Instant) -> El {
    let (primary, shade, _) = doll.id.palette();
    let mut root = el()
        .size(SLOT_W, FULL_H)
        .child(character_head_el(doll, now));

    // Chubby rounded torso
    root = root.child(px(9.5, 14.5, 13.0, 6.8, shade, 3.4));

    // Arms (cheering upward when succeeded)
    let arm_y = if doll.phase == DollPhase::Succeeded {
        12.4
    } else {
        15.2
    };
    root = root
        .child(px(6.2, arm_y, 3.0, 4.0, primary, 1.5))
        .child(px(22.8, arm_y, 3.0, 4.0, primary, 1.5));

    // Stubby cute feet
    let mut leg_l = px(10.8, 21.0, 3.2, 4.6, primary, 1.6);
    let mut leg_r = px(18.0, 21.0, 3.2, 4.6, primary, 1.6);
    if matches!(doll.phase, DollPhase::Thinking | DollPhase::Succeeded) {
        leg_l = leg_l.looping(
            LoopSpec::slide(0.75, [0.0, 0.0], [0.0, -0.32]).easing(BREATHE_EASE),
        );
        leg_r = leg_r.looping(
            LoopSpec::slide(0.75, [0.0, -0.32], [0.0, 0.0]).easing(BREATHE_EASE),
        );
    }
    root.child(leg_l).child(leg_r)
}

fn px(x: f32, y: f32, w: f32, h: f32, color: Color, radius: f32) -> El {
    el().absolute(x, y).size(w, h).radius(radius).fill(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn codex_lifecycle_triggers_success_and_expires_to_idle() {
        let mut dolls = Dolls::new();
        let t0 = Instant::now();

        // Priming poll ignores stale completion from before launch.
        dolls.apply_codex_at(
            &json!({
                "status": "idle",
                "completions": [{ "id": "old-1", "at": 100, "message": "old task" }]
            }),
            &json!([]),
            t0,
        );
        assert_eq!(dolls.get(DollId::Codex).phase, DollPhase::Idle);
        assert_eq!(dolls.get(DollId::Codex).cord_height(t0, false), CORD_PEEK);

        // Running turn keeps head peeking under the notch and extracts model + pid.
        dolls.apply_codex_at(
            &json!({ "status": "running", "completions": [] }),
            &json!([{ "kind": "Codex", "pid": 42, "status": "working", "model": "gpt-6.2-sol" }]),
            t0 + Duration::from_secs(1),
        );
        let c = dolls.get(DollId::Codex);
        assert_eq!(c.phase, DollPhase::Thinking);
        assert_eq!(c.pid, Some(42));
        assert_eq!(c.model_name, "GPT 6.2 Sol");
        assert!(!c.is_dropped(t0 + Duration::from_secs(1)));
        assert_eq!(c.cord_height(t0 + Duration::from_secs(1), false), CORD_PEEK);

        // Fresh completion drops the full doll down with Succeeded message.
        let t2 = t0 + Duration::from_secs(2);
        dolls.apply_codex_at(
            &json!({
                "status": "succeeded",
                "completions": [{ "id": "new-2", "at": 200, "message": "已修复边界条件并通过全部单测" }]
            }),
            &json!([]),
            t2,
        );
        let c = dolls.get(DollId::Codex);
        assert_eq!(c.phase, DollPhase::Succeeded);
        assert!(c.is_dropped(t2));
        assert_eq!(c.cord_height(t2, false), CORD_SUCCEEDED + 8.0);
        assert!(c.display_text(t2).unwrap().contains("搞定"));

        // After SUCCESS_TTL expires, poll retracts Codex back to peeking Idle.
        let t_later = t2 + SUCCESS_TTL + Duration::from_secs(1);
        assert!(dolls.get_mut(DollId::Codex).expire(t_later));
        assert_eq!(dolls.get(DollId::Codex).phase, DollPhase::Idle);
        assert!(!dolls.get(DollId::Codex).is_dropped(t_later));
        assert_eq!(dolls.get(DollId::Codex).cord_height(t_later, false), CORD_PEEK);
    }

    #[test]
    fn map_models_track_gpt_and_gemini_independently() {
        let mut dolls = Dolls::new();
        let t0 = Instant::now();
        dolls.apply_map_at(
            &json!({
                "running": true,
                "models": [
                    { "family": "GPT", "model": "gpt-6.1-sol", "status": "running", "at": 10 },
                    { "family": "Gemini", "model": "gemini-4-hydrogen-high", "status": "idle", "at": 5 }
                ],
                "completions": [],
                "sessions": []
            }),
            t0,
        );
        assert_eq!(dolls.get(DollId::MapGpt).phase, DollPhase::Thinking);
        assert_eq!(dolls.get(DollId::MapGpt).model_name, "GPT 6.1 Sol");
        assert_eq!(dolls.get(DollId::MapGemini).phase, DollPhase::Idle);
        assert_eq!(
            dolls.get(DollId::MapGemini).model_name,
            "Gemini 4 Hydrogen High"
        );
    }

    #[test]
    fn poking_sleeping_doll_wakes_it_and_poking_succeeded_attracts_neighbor() {
        let mut dolls = Dolls::new();
        let t0 = Instant::now();
        dolls.poke_at(DollId::MapGemini, t0);
        assert!(dolls.get(DollId::MapGemini).is_woken(t0));
        assert_eq!(
            dolls.get(DollId::MapGemini).cord_height(t0, false),
            CORD_WAKE + 8.0
        );

        // Make Codex succeed, poke it, and verify idle MapGpt peeks over.
        dolls.get_mut(DollId::Codex).set_phase(DollPhase::Succeeded, t0, None);
        dolls.poke_at(DollId::Codex, t0);
        assert!(dolls.get(DollId::MapGpt).is_woken(t0));
        assert!(dolls.get(DollId::MapGpt).display_text(t0).is_some());
    }
}
