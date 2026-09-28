//! The in-game HUD: the 13-slot element system, health / bolts / weapon elements, the banner, the help box and
//! the game's word-wrap layout (`FontPrintWindow`). Spec: docs/plan/hud_text.md §2–§4; addresses are level01.elf.
//!
//! [`HudState::tick`] runs one 60 Hz game tick (the HUD update loop 0x24f880, `Help_Update` 0x225bd0) and returns
//! the frame's draw calls in the game's order: slots 0..12 (`HudDraw` 0x24fb50), the banner, the help box
//! (0x2266c0). A [`Draw`] is one call of the game's 2D layer (`HudSprite` family, `FontPrint`,
//! `FontPrintWindow`, `DrawUIFrame`, `DrawTexturedQuad`); the engine turns them into GS primitives.
//!
//! **Slots** (13 × 0x90 at 0x17e0d0). `queue_animation_update` (0x24ad98) stores an element request as
//! *pending* when any argument differs from the last request (and then zeroes the slot's timer and ramps); the
//! loop applies it (`apply_pending_animation` 0x24aea8: copy, call the element's init) once the slot's
//! visibility counter (+0x6c) has come down to −6. Every tick the loop: bumps the timer (+0x7c) to at least
//! `ScaleTicks(10)` for a persistent element (flag 0x10); decrements it; counts +0x6c up to 30 while it is
//! ≥ 1 after the decrement, else down to −6; applies a pending element; runs the element's update. Updates ramp
//! **slide** (+0x70) 0→steps, then **alpha** (+0x71) 0→steps, one step per tick while `timer ≥ ScaleTicks(5)`,
//! and back (alpha first, then slide, then +0x6c = −6) below it. Draws use `s = slide/steps`, `f = alpha/steps`.
//!
//! **Constants** are the overlay's small-data values (gp = 0x166c00) and the boot ELF's defaults: rate factor
//! 0x15ed68 = 1.0, PAL flag 0x15ed80 = 0, max HP 0x15eda0 = 4, help text / voice options 0x15ee1d / 0x15ee1c = 1.

use rc_formats::font::{measure_text_width, Font, GlyphTable};
use rc_formats::hud::{Hud, IconEntry};
use rc_formats::strings::{self, Message};

/// Frame buffer size (NTSC, `0x13e500` / `0x13e504` at run time).
pub const SCREEN_W: i32 = 512;
pub const SCREEN_H: i32 = 416;
/// Element slots.
pub const SLOTS: usize = 13;

/// `ScaleTicks` (0x220e30): `(int)(n · rate + 0.5)`, rate = 1.0 on NTSC.
pub fn scale_ticks(n: i32) -> i32 { (n as f32 * 1.0 + 0.5) as i32 }

/// `FastTweenColor` (0x2221a8): per byte `a·(1−t) + b·t` on VU0 floats, truncated.
pub fn tween_color(t: f32, a: u32, b: u32) -> u32 {
    let mut out = 0u32;
    for k in 0..4 {
        let (ca, cb) = (((a >> (8 * k)) & 0xff) as f32, ((b >> (8 * k)) & 0xff) as f32);
        out |= ((ca * (1.0 - t) + cb * t) as i32 as u32 & 0xff) << (8 * k);
    }
    out
}

/// Static slot anchors (+0x50 / +0x54, level01 .data 0x17e0d0; never written). Only x is read by the elements
/// ported here; each element has its own y.
pub const ANCHORS: [(i32, i32); SLOTS] = [
    (20, 15), (256, 32), (492, 15), (20, 208), (492, 208), (20, 376), (256, 376), (492, 376),
    (492, 208), (492, 208), (492, 208), (492, 208), (142, 50),
];

// Small-data constants (gp-relative, level01 overlay section 0).
/// gp−0x7410 0x15f7f0: health ramp steps.
const HEALTH_STEPS: i32 = 8;
/// 0x15f7f8 / 0x17e8e0 / 0x17e8f8: orb offsets for 4, 5 and 8 orbs.
const ORBS_4: [(i32, i32); 4] = [(-80, 0), (-48, 0), (-16, 0), (16, 0)];
const ORBS_5: [(i32, i32); 5] = [(-64, 0), (-32, 0), (0, 0), (-48, 32), (-16, 32)];
const ORBS_8: [(i32, i32); 8] = [(-80, 0), (-48, 0), (-16, 0), (16, 0), (-80, 32), (-48, 32), (-16, 32), (16, 32)];
/// 0x15f808 health y base (+18 NTSC, +10 PAL), 0x15f80c second bar row, 0x15f810 bar y offset,
/// 0x15f81c bar tile size, 0x15f820 health bar alpha.
const HEALTH_Y: i32 = 14;
const BAR_ROW_DY: i32 = 16;
const BAR_DY: i32 = -16;
const BAR_TILE: i32 = 32;
const HEALTH_BAR_ALPHA: i32 = 20;
/// 0x15f824 / 0x15f828 bolt ramp steps, 0x15f82c / 0x15f830 text colour tween, 0x15f834 / 0x15f838 text offset,
/// 0x15f83c.. separator offsets ("'" / "."), 0x15f84c digit width.
const BOLT_SLIDE_STEPS: i32 = 8;
const BOLT_ALPHA_STEPS: i32 = 8;
const TEXT_FROM: u32 = 0x00e0_8060;
const TEXT_TO: u32 = 0x80e0_8060;
const BOLT_TEXT_D: (i32, i32) = (-32, 8);
const SEP_APOSTROPHE_D: (i32, i32) = (-30, 21);
const SEP_DOT_D: (i32, i32) = (-29, 10);
const DIGIT_W: i32 = 13;
/// 0x15f91c / 0x15f920 weapon ramp steps, 0x15f924 / 0x15f928 bar length (max ammo < 100 / ≥ 100),
/// 0x15f934 / 0x15f938 empty-ammo colour tween, 0x15f93c / 0x15f940 text offset.
const WEAPON_STEPS: i32 = 8;
const WEAPON_LEN: [i32; 2] = [70, 95];
const EMPTY_FROM: u32 = 0x0020_2080;
const EMPTY_TO: u32 = 0x8020_2080;
const WEAPON_TEXT_D: (i32, i32) = (25, 8);
/// Element y on NTSC (the code picks 10 on PAL).
const ELEMENT_Y: i32 = 18;

/// Icon ids (hud_text.md §1.3).
pub mod icon {
    pub const HEALTH_SLOT: u16 = 30005;
    pub const ORB: u16 = 30006;
    pub const BOLT_SLOT: u16 = 30030;
    pub const BOLT: u16 = 30031;
    pub const BAR: u16 = 30080;
    pub const ITEM_BASE: u16 = 60000;
}

/// The icon and frame-size tables the HUD code reads (`GetIconFrame`, `HudFrame`), the glyph tables and the
/// level's messages.
#[derive(Clone, Debug)]
pub struct HudAssets {
    pub icons: Vec<IconEntry>,
    /// Texture size of every frame (power of two), (0, 0) when unknown.
    pub frame_sizes: Vec<(i32, i32)>,
    pub glyphs: [GlyphTable; 3],
    pub messages: Vec<Message>,
}

impl HudAssets {
    pub fn new(hud: &Hud, glyphs: [GlyphTable; 3], messages: Vec<Message>) -> Self {
        let frame_sizes = (0..hud.frames.len()).map(|i| hud.frame_size(i).map_or((0, 0), |(w, h)| (w as i32, h as i32))).collect();
        HudAssets { icons: hud.icons.clone(), frame_sizes, glyphs, messages }
    }

    /// `GetIconFrame__Fii` with every bank resident (rc_formats::hud::Hud::icon_frame).
    pub fn icon_frame(&self, id: u16, k: i32) -> usize {
        let i = self.icons.iter().position(|e| e.id == 0xffff || e.id == id).unwrap_or(self.icons.len().saturating_sub(1));
        let Some(e) = self.icons.get(i) else { return 0 };
        if e.id == 0xffff || k >= e.frame_count as i32 { return 0; }
        (e.first_frame as i32 + k).max(0) as usize
    }

    fn size(&self, frame: usize) -> (i32, i32) { self.frame_sizes.get(frame).copied().unwrap_or((0, 0)) }

    fn glyphs(&self, font: Font) -> &GlyphTable { &self.glyphs[font as usize] }

    /// `FUN_0021cc90`-style width of `text` (to the NUL) in `font`.
    pub fn text_width(&self, font: Font, text: &[u8]) -> i32 { measure_text_width(text, -1, self.glyphs(font)) }
}

/// Which `HudSprite` variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rot {
    /// 0x2500e0: SPRITE, UV (0,0)..(tw,th).
    None,
    /// 0x250468: TRISTRIP with the texture rotated 180° (right bar caps).
    R180,
    /// 0x2506c8: TRISTRIP with the texture rotated 90°.
    R90,
}

/// One call of the game's 2D layer, in draw order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Draw {
    /// `HudSprite` (frame, x, y, w, h, alpha): the whole frame texture stretched over `w×h`, RGBAQ =
    /// `alpha << 24 | 0x7f7f7f` (alpha may exceed 0x80).
    Sprite { frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot },
    /// `FontPrint(x, y, rgba, text, -1)` in `font` (right / centre wrappers already applied to `x`).
    Text { font: Font, x: i32, y: i32, rgba: u32, text: Vec<u8> },
    /// `FontPrintWindow(window, rgba, text, -1)` in `font`.
    TextWindow { font: Font, window: text::Window, rgba: u32, text: Vec<u8> },
    /// `DrawUIFrame(top, bottom, left, right, alpha)` (0x21c958).
    UiFrame { top: i32, bottom: i32, left: i32, right: i32, alpha: i32 },
    /// `DrawTexturedQuad(x, y, w, h, u, v, tw, th, rgba, GetEffectTex(fx))` (0x21be90).
    FxQuad { fx: usize, x: i32, y: i32, w: i32, h: i32, u: i32, v: i32, tw: i32, th: i32, rgba: u32 },
}

/// What a slot shows (the init / update / draw callbacks + data pointer of `queue_animation_update`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Element {
    #[default]
    Empty,
    /// 0x24e1b8 / 0x24e238 / 0x24e418, data 0x1415f8 (HP), max 8.
    Health,
    /// 0x24e8d0 / 0x24e908 / 0x24ea00, data 0x15ed98 (bolts), max 9999999.
    Bolts,
    /// 0x24f368 / 0x2519c0 / 0x24f3b0, data 0x13d428 + 4·item (ammo), max = item table +6.
    Weapon { item: u16 },
    /// The context prompt, slot 12: init 0x24c828, update 0x24c878 (the generic ramp 0x24b538), draw 0x24c898
    /// (Lombyte `HudRaceTimerDraw`), no data (docs/plan/interaction.md §2).
    Prompt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Request {
    flags: u32,
    icon: u16,
    element: Element,
    max: i32,
}

/// One HUD slot (the fields of the 0x90-byte record the ported elements use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// +0x04 (0x10 = persistent).
    pub flags: u32,
    /// +0x00: the element's icon id.
    pub icon: u16,
    /// +0x08.
    pub max: i32,
    pub element: Element,
    /// +0x20..+0x38: the last request, +0x68: not applied yet.
    last: Request,
    pending: bool,
    /// +0x64: request handle.
    pub handle: u32,
    /// +0x48 / +0x4a: `HudFrame` offset.
    pub offset: (i32, i32),
    /// +0x6c: −6 = fully hidden (a pending element may be applied), up to 30 while shown.
    pub counter: i32,
    /// +0x70 / +0x71.
    pub slide: i32,
    pub alpha: i32,
    /// +0x74 shown value, +0x78 current value.
    pub shown: i32,
    pub value: i32,
    /// +0x7c.
    pub timer: i32,
}

const EMPTY_REQUEST: Request = Request { flags: 0, icon: 0xffff, element: Element::Empty, max: 1 };

impl Default for Slot {
    /// After the level-start reset `FUN_0024af10`: the empty element applied, timer 0, +0x6c = −6.
    fn default() -> Self {
        Slot {
            flags: 0, icon: 0xffff, max: 1, element: Element::Empty, last: EMPTY_REQUEST, pending: false, handle: 0,
            offset: (0, 0), counter: -6, slide: 0, alpha: 0, shown: 0, value: 0, timer: 0,
        }
    }
}

/// Banner (`ShowBanner` 0x2789e0, drawn by 0x24fb50 through 0x251b88).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Banner {
    /// 0x15f640.
    pub countdown: i32,
    /// 0x15f644, 0..0x80.
    pub alpha: i32,
    /// 0x15f648.
    pub y: i32,
    /// 0x179598.
    pub text: Vec<u8>,
}

/// Help / Infobot box (`Help_Update` 0x225bd0, draw 0x2266c0); fields at 0x179890.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Help {
    /// 0x179890: 0 idle, 1 open, 2 logo prompt, 3 grow, 4 text fade-in, 5 hold, 6 text fade-out, 7 shrink.
    pub state: u8,
    /// 0x179894: ticks in the state (incremented before the state's rule runs).
    pub t: i32,
    /// 0x1798b4: requested message id, −1 none.
    pub request: i32,
    /// 0x1798b0: index of the shown message.
    pub index: Option<usize>,
    /// 0x179898 / 0x17989c: full half-size; 0x1798a0 / 0x1798a4: centre; 0x1798a8 / 0x1798ac: current half-size.
    pub half_w: i32,
    pub half_h: i32,
    pub cx: i32,
    pub cy: i32,
    pub cur_w: i32,
    pub cur_h: i32,
    /// 0x1798c0: help allowed. The game opens it `ScaleTicks(120)` ticks after the first d-pad input of the
    /// level (0x1798c4 counter, pad bits 0xf000); the port starts it open.
    pub enabled: bool,
}

impl Default for Help {
    fn default() -> Self { Help { state: 0, t: 0, request: -1, index: None, half_w: 0, half_h: 0, cx: 0, cy: 0, cur_w: 0, cur_h: 0, enabled: true } }
}

/// Help box text: small font, window x 44..468, y 240..480, anchor 256, line height 16.
const HELP_COLOUR: u32 = 0x00ff_a888;
fn help_window(y: i32, flags: u16) -> text::Window { text::Window::new(0xf0, 0x1e0, 0x2c, 0x1d4, 0x100, y as i16, 0x10, flags) }

/// Per-tick game values the HUD elements read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inputs {
    /// 0x1415f8.
    pub hp: i32,
    /// 0x15eda0 (4 at start, up to 8).
    pub max_hp: i32,
    /// 0x15ed98.
    pub bolts: i32,
    /// Held item with an ammo HUD, its ammo and max ammo; `None` when the held item has none. The game's rule
    /// (0x24f9c0): shown iff item table 0x1c4538[item] (stride 0x18) has a non-zero s16 at +0 and the player
    /// state 0x1413f4 is 0; the wrench (item 8) has 0 there, so holding it shows nothing.
    pub weapon: Option<(u16, i32, i32)>,
    /// 0x15ed88 (rc_formats::strings::lang).
    pub lang: u32,
}

/// The HUD.
#[derive(Clone, Debug)]
pub struct HudState {
    pub assets: HudAssets,
    pub slots: [Slot; SLOTS],
    /// 0x17e958: next request handle.
    next_handle: u32,
    /// 0x17e95c: show every slot (bumps timers like flag 0x10).
    pub show_all: bool,
    /// 0x15fab0: health animation shorts {spin tick, glow, glow velocity, glow acceleration}.
    pub health_anim: [i16; 4],
    /// 0x15fab8: bolt spin tick.
    pub bolt_anim: i16,
    /// 0x15f970 / 0x15f96c: the weapon slot's request handle and item.
    weapon_handle: Option<u32>,
    /// The context prompt this tick (`PromptTick` 0x278eb8 / `NpcTalkUpdate`: an owner holds a message) and the
    /// text buffer 0x17e9b0 the draw prints ([`HudState::set_prompt`]).
    pub prompt_show: bool,
    pub prompt_text: Vec<u8>,
    /// The bolt counter kept up (`queue_animation_update(0x12, …)`: flag 0x10) while the vendor is open.
    pub bolts_pinned: bool,
    bolts_pin_handle: Option<u32>,
    pub banner: Banner,
    pub help: Help,
    /// △ pressed this tick (pad 0x13cae4 bit 0x10): skips the help box.
    pub triangle: bool,
    /// `PlayLevelSoundAtMoby(index, flags, 0)` calls of the HUD code since the owner last drained them (the help box's
    /// opening sound: crate::audio::class_sounds::level_sound).
    pub level_sounds: Vec<(i32, u32)>,
    inputs: Inputs,
    last: Option<(i32, i32)>,
}

impl HudState {
    pub fn new(assets: HudAssets) -> Self {
        HudState {
            assets,
            slots: [Slot::default(); SLOTS],
            next_handle: 0,
            show_all: false,
            health_anim: [0; 4],
            bolt_anim: 0,
            weapon_handle: None,
            prompt_show: false,
            prompt_text: Vec::new(),
            bolts_pinned: false,
            bolts_pin_handle: None,
            banner: Banner { y: 100, ..Default::default() },
            help: Help::default(),
            triangle: false,
            level_sounds: Vec::new(),
            inputs: Inputs { hp: 4, max_hp: 4, bolts: 0, weapon: None, lang: 0 },
            last: None,
        }
    }

    /// `queue_animation_update(slot | flags, icon, init, update, draw, data, max)` → the request handle.
    pub fn queue(&mut self, slot_flags: u32, icon: u16, element: Element, max: i32) -> u32 {
        let slot = (slot_flags & 0xf) as usize;
        let req = Request { flags: slot_flags & 0xfff0, icon, element, max };
        let s = &mut self.slots[slot];
        if s.last == req { return s.handle; }
        s.last = req;
        s.handle = self.next_handle;
        self.next_handle += 1;
        s.pending = true;
        s.timer = 0;
        s.slide = 0;
        s.alpha = 0;
        if req.flags & s.flags & 0x20 != 0 { self.apply(slot); }
        self.slots[slot].handle
    }

    /// `FUN_0024b090(handle, flags)`: replace the flags of the slot holding `handle`.
    pub fn set_flags(&mut self, handle: u32, flags: u32) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.handle == handle) {
            s.last.flags = flags;
            if !s.pending { s.flags = flags; }
        }
    }

    /// `apply_pending_animation` + the element's init.
    fn apply(&mut self, slot: usize) {
        let s = &mut self.slots[slot];
        let r = s.last;
        s.icon = r.icon;
        s.flags = r.flags;
        s.element = r.element;
        s.max = r.max;
        s.pending = false;
        match r.element {
            Element::Empty => {}
            Element::Health => {
                // 0x24e1b8.
                s.timer = scale_ticks(180) + 30;
                s.offset = (32, 0);
                self.init_value(slot);
                self.health_anim = [0, 0, 0, 1];
            }
            Element::Bolts => {
                // 0x24e8d0 → 0x24f368.
                s.timer = scale_ticks(120) + 30;
                self.init_value(slot);
                self.bolt_anim = 0;
            }
            Element::Weapon { .. } => {
                s.timer = scale_ticks(120) + 30;
                self.init_value(slot);
            }
            Element::Prompt => {
                // 0x24c828: offset 0, timer ScaleTicks(10) + 30, size 32×32.
                s.offset = (0, 0);
                s.timer = scale_ticks(10) + 30;
                self.init_value(slot);
            }
        }
    }

    /// `FUN_0024b4b0(handle, n)`: the slot holding `handle`, when its request is applied, keeps its timer at ≥ n;
    /// false when there is no such slot or it is still pending.
    pub fn keep_up(&mut self, handle: u32, n: i32) -> bool {
        // (The port's handles start at 0 like the empty slots' default: only a slot holding an element matches.)
        let Some(s) = self.slots.iter_mut().find(|s| s.handle == handle && !s.pending && s.element != Element::Empty) else { return false };
        s.timer = s.timer.max(n);
        true
    }

    /// The context prompt of this tick and its text (the engine: `moby_update::interact`).
    pub fn set_prompt(&mut self, show: bool, text: &[u8]) {
        self.prompt_show = show;
        if self.prompt_text != text { self.prompt_text = text.to_vec(); }
    }

    /// The element's data word (`*data`).
    fn data(&self, e: Element) -> i32 {
        match e {
            Element::Empty => 99999,
            Element::Health => self.inputs.hp,
            Element::Bolts => self.inputs.bolts,
            Element::Weapon { .. } => self.inputs.weapon.map_or(0, |w| w.1),
            Element::Prompt => 99999,
        }
    }

    /// `FUN_0024b318`: value = min(*data, max), shown = value (no data → 99999).
    fn init_value(&mut self, slot: usize) {
        let v = self.data(self.slots[slot].element);
        let s = &mut self.slots[slot];
        s.value = if s.element == Element::Empty { 99999 } else { v.min(s.max) };
        s.shown = s.value;
    }

    /// Asks for message `id` in the help box (`FUN_00225818`); false while a box is up or pending.
    pub fn help_request(&mut self, id: i32) -> bool {
        if self.help.state != 0 || self.help.request != -1 { return false; }
        self.help.request = id;
        true
    }

    /// `ShowBanner(msg, ticks)` with the text already formatted; `ticks` defaults to `ScaleTicks(180)`.
    pub fn show_banner(&mut self, text: &[u8], ticks: Option<i32>) {
        self.banner.text = text.to_vec();
        self.banner.countdown = ticks.unwrap_or_else(|| scale_ticks(180));
    }

    /// `ShowBanner(id, ticks)` 0x2789e0: the level message `id` (`msg_string`) for `ticks` (the gold bolt's "Gold Bolt
    /// Acquired", `ShowPlanetBanner`'s planet messages).
    pub fn show_banner_msg(&mut self, id: i32, ticks: i32) {
        let t = strings::lookup(&self.assets.messages, id).to_vec();
        self.show_banner(&t, Some(ticks));
    }

    /// `ShowBannerf(id, n, −1)` 0x278a50: the level message `id` with its `%d` replaced by `n` (`sprintf` into
    /// 0x179598), 180 ticks (the ammo pickups' "+n" banner).
    pub fn show_bannerf(&mut self, id: i32, n: i32) {
        let t = strings::lookup(&self.assets.messages, id).to_vec();
        let text = match t.windows(2).position(|w| w == b"%d") {
            Some(i) => [&t[..i], n.to_string().as_bytes(), &t[i + 2..]].concat(),
            None => t,
        };
        self.show_banner(&text, None);
    }

    /// One game tick: the callers that arm elements, `Help_Update`, the slot loop; then the frame's draws.
    pub fn tick(&mut self, inputs: Inputs) -> Vec<Draw> {
        self.inputs = inputs;
        // Callers: the damage path 0x226fa8 shows health (the port also on a gain [L]); the bolt pickups
        // (0x2aba68 and others) show the counter.
        if let Some((hp, bolts)) = self.last {
            if inputs.hp != hp { self.queue(1, icon::HEALTH_SLOT, Element::Health, 8); }
            if inputs.bolts != bolts { self.queue(2, icon::BOLT_SLOT, Element::Bolts, 9_999_999); }
        }
        self.last = Some((inputs.hp, inputs.bolts));
        // PromptTick 0x278eb8's slot part (and NpcTalkUpdate's): slot 12 requested, or kept up for 10 ticks.
        if self.prompt_show {
            let h = self.queue(12, 0, Element::Prompt, 0);
            self.keep_up(h, scale_ticks(10));
        }
        // The vendor's bolt counter (OpenVendorMenu: slot 2 | 0x10; VendorExit: flags 0).
        match (self.bolts_pinned, self.bolts_pin_handle) {
            (true, None) => self.bolts_pin_handle = Some(self.queue(0x12, icon::BOLT_SLOT, Element::Bolts, 9_999_999)),
            (false, Some(h)) => {
                self.set_flags(h, 0);
                self.bolts_pin_handle = None;
            }
            _ => {}
        }
        self.help_update();
        self.update_slots();
        self.triangle = false;
        let mut out = Vec::new();
        self.draw(&mut out);
        out
    }

    /// 0x24f880 (with 0x24f9c0 first).
    fn update_slots(&mut self) {
        self.update_weapon_request();
        for i in 0..SLOTS {
            let s = &mut self.slots[i];
            if s.flags & 0x10 != 0 || self.show_all { s.timer = s.timer.max(scale_ticks(10)); }
            let shown = s.timer >= 1 && {
                s.timer -= 1;
                s.timer >= 1
            };
            if shown {
                if s.counter < 30 { s.counter += 1; }
            } else if s.counter > -6 {
                s.counter -= 1;
            }
            if s.pending && s.counter == -6 { self.apply(i); }
            match self.slots[i].element {
                Element::Empty => {}
                Element::Health => self.update_health(i),
                Element::Bolts => self.update_bolts(i),
                Element::Weapon { .. } => self.update_weapon(i),
                Element::Prompt => {
                    // 0x24b538 with no data: the ramp only (steps ScaleTicks(8)).
                    let s = &mut self.slots[i];
                    let down = s.timer < scale_ticks(5);
                    Self::ramp(s, scale_ticks(8), scale_ticks(8), down);
                }
            }
        }
    }

    /// 0x24f9c0: keep the weapon element requested while the held item has an ammo HUD, release it otherwise.
    fn update_weapon_request(&mut self) {
        match self.inputs.weapon {
            Some((item, _, max)) => {
                self.weapon_handle = Some(self.queue(0x10, icon::ITEM_BASE.wrapping_add(item), Element::Weapon { item }, max));
            }
            None => {
                if let Some(h) = self.weapon_handle.take() { self.set_flags(h, 0); }
            }
        }
    }

    fn ramp(s: &mut Slot, slide_steps: i32, alpha_steps: i32, down: bool) {
        if down {
            s.counter = 1;
            if s.alpha != 0 {
                s.alpha -= 1;
            } else if s.slide != 0 {
                s.slide -= 1;
            } else {
                s.counter = -6;
            }
        } else if s.slide < slide_steps {
            s.slide += 1;
        } else if s.alpha < alpha_steps {
            s.alpha += 1;
        }
    }

    /// 0x24e238.
    fn update_health(&mut self, i: usize) {
        let hp = self.inputs.hp.max(0);
        let p = &mut self.health_anim;
        let acc = p[2].wrapping_add(p[3]);
        p[1] = p[1].wrapping_add(p[2]);
        p[2] = acc;
        p[0] = (p[0] + 1) % 60;
        p[2] = acc.clamp(-60, 60);
        if p[1] >= 0x800 && p[2] > 0 { p[3] = -1; }
        if p[1] >= 0x1000 { p[1] = 0xfff; }
        if p[1] < 0x7ff && p[2] < 0 { p[3] = 1; }
        if p[1] < 0 { p[1] = 0; }
        let s = &mut self.slots[i];
        s.value = hp.min(s.max);
        if s.shown != s.value || s.shown == 1 {
            s.timer = scale_ticks(120);
            s.shown = s.value;
        }
        let down = s.timer < scale_ticks(5);
        Self::ramp(s, HEALTH_STEPS, HEALTH_STEPS, down);
    }

    /// 0x24e908.
    fn update_bolts(&mut self, i: usize) {
        self.bolt_anim = (self.bolt_anim + 1) % 60;
        let bolts = self.inputs.bolts;
        let s = &mut self.slots[i];
        if s.shown != bolts {
            s.shown = bolts;
            s.timer = scale_ticks(90);
        }
        let down = s.timer < scale_ticks(5);
        Self::ramp(s, BOLT_SLIDE_STEPS, BOLT_ALPHA_STEPS, down);
    }

    /// 0x2519c0.
    fn update_weapon(&mut self, i: usize) {
        let ammo = self.data(self.slots[i].element);
        let s = &mut self.slots[i];
        s.shown = if s.max < ammo { s.max } else { ammo.max(0) };
        let down = s.timer < scale_ticks(5);
        if !down { s.timer = scale_ticks(5); }
        Self::ramp(s, WEAPON_STEPS, WEAPON_STEPS, down);
    }

    /// `Help_Update` (0x225bd0) without the voice system (no stream ever plays).
    fn help_update(&mut self) {
        let h = &mut self.help;
        if !h.enabled {
            h.state = 0;
            h.t = 0;
            h.request = -1;
            return;
        }
        h.t += 1;
        if h.state == 0 {
            if h.request >= 0 {
                let idx = strings::find_index(&self.assets.messages, h.request);
                self.help.request = -1;
                self.help.index = idx;
                if idx.is_some() { self.help_size(); }
            }
            return;
        }
        let tri = self.triangle;
        match h.state {
            1 => {
                if tri {
                    h.t = 8 - h.t;
                    h.state = 7;
                } else if h.t >= 6 {
                    h.state = 2;
                    h.t = 0;
                }
            }
            2 | 3 => {
                if tri {
                    h.state = 7;
                    h.t = 0;
                } else if h.state == 2 && h.t >= scale_ticks(24) {
                    // Stream status never 3 and no voice line playing: straight on.
                    h.state = 3;
                    h.t = 0;
                } else if h.state == 3 && h.t >= 8 {
                    h.state = 4;
                    h.t = 0;
                }
            }
            4 => {
                if tri {
                    h.state = 6;
                    h.t = 4 - h.t;
                } else if h.t >= 4 {
                    h.state = 5;
                    h.t = 0;
                }
            }
            5 => {
                if h.t >= scale_ticks(420) || tri {
                    h.state = 6;
                    h.t = 0;
                }
            }
            6 => {
                // 0x15ee1d (help text option) = 1.
                if h.t >= 4 || tri {
                    h.state = 7;
                    h.t = 0;
                }
            }
            7 if h.t > 7 => {
                h.state = 0;
                h.index = None;
                h.t = 0;
            }
            _ => {}
        }
    }

    fn help_text(&self) -> &[u8] { self.help.index.and_then(|i| self.assets.messages.get(i)).map_or(&[][..], |m| &m.text) }

    /// 0x225a98: the opening sound `PlayLevelSoundAtMoby(0, 1, 0)` (the help text or voice option 0x15ee1d / 0x15ee1c
    /// is on: the port has the text on), measure the text (small font, window flags 7) and size the box.
    fn help_size(&mut self) {
        self.level_sounds.push((crate::audio::class_sounds::level_sound::HELP_OPEN, 1));
        let mut win = help_window(0x168, 7);
        text::layout(&mut win, self.help_text(), -1, self.assets.glyphs(Font::Small), true);
        let h = &mut self.help;
        h.state = 1;
        h.t = 0;
        let hh = win.height >> 1;
        h.cy = SCREEN_H - 0x3c;
        h.half_h = hh as i32 + 5;
        h.half_w = (win.max_width >> 1) as i32 + 10;
        h.cx = 0x100;
        h.cur_w = 8;
        h.cur_h = 8;
        if SCREEN_H - 12 < h.cy + h.half_h { h.cy = SCREEN_H - (hh as i32 + 0x11); }
    }

    /// `HudDraw` 0x24fb50 (slots, banner) then the help box 0x2266c0.
    fn draw(&mut self, out: &mut Vec<Draw>) {
        for i in 0..SLOTS {
            match self.slots[i].element {
                Element::Empty => {}
                Element::Health => self.draw_health(i, out),
                Element::Bolts => self.draw_bolts(i, out),
                Element::Weapon { .. } => self.draw_weapon(i, out),
                Element::Prompt => self.draw_prompt(i, out),
            }
        }
        self.draw_banner(out);
        self.draw_help(out);
    }

    fn fractions(s: &Slot, slide_steps: i32, alpha_steps: i32) -> (f32, f32) {
        ((s.slide as f32 / slide_steps as f32).clamp(0.0, 1.0), (s.alpha as f32 / alpha_steps as f32).clamp(0.0, 1.0))
    }

    /// `HudFrame` 0x24fd28: slot offset, then flags 1 centre, 2 half size, 4 double size.
    #[allow(clippy::too_many_arguments)]
    fn hud_frame(&self, slot: usize, frame: usize, x: i32, y: i32, flags: u32, alpha: i32, out: &mut Vec<Draw>) {
        let (ox, oy) = self.slots[slot].offset;
        let (mut x, mut y) = (x + ox, y + oy);
        let (w0, h0) = self.assets.size(frame);
        let (hw, hh) = (w0 >> 1, h0 >> 1);
        let (mut w, mut h) = (w0, h0);
        if flags & 1 != 0 {
            x -= hw;
            y -= hh;
        }
        if flags & 2 != 0 {
            x += w0 >> 2;
            y += h0 >> 2;
            w = hw;
            h = hh;
        }
        if flags & 4 != 0 {
            x -= hw;
            y -= hh;
            w <<= 1;
            h <<= 1;
        }
        out.push(Draw::Sprite { frame, x, y, w, h, alpha, rot: Rot::None });
    }

    fn sprite(frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot) -> Draw { Draw::Sprite { frame, x, y, w, h, alpha, rot } }

    /// `font_print_right` (regular font): `x − width`.
    fn text_right(&self, x: i32, y: i32, rgba: u32, text: &[u8], out: &mut Vec<Draw>) {
        let w = self.assets.text_width(Font::Regular, text);
        out.push(Draw::Text { font: Font::Regular, x: x - w, y, rgba, text: text.to_vec() });
    }

    /// 0x24e418.
    fn draw_health(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if s.slide == 0 { return; }
        let x = ANCHORS[i].0;
        let y = HEALTH_Y + ELEMENT_Y;
        let (sf, f) = Self::fractions(s, HEALTH_STEPS, HEALTH_STEPS);
        let a_orb = (f * 128.0) as i32;
        let bar_a = (HEALTH_BAR_ALPHA as f32 * sf) as i32;
        let p = &self.health_anim;
        let mut fr = (p[0] as i32) >> 1;
        if s.value == 0 { fr = 30; }
        let max_hp = self.inputs.max_hp;
        let (orbs, hw, hw2): (&[(i32, i32)], i32, i32) = match max_hp {
            8 => (&ORBS_8, (sf * 48.0) as i32, (sf * 48.0) as i32),
            5 => (&ORBS_5, (sf * 32.0) as i32, (sf * 16.0) as i32),
            _ => (&ORBS_4, (sf * 48.0) as i32, 0),
        };
        let cap = self.assets.icon_frame(icon::BAR, 1);
        let mid = self.assets.icon_frame(icon::BAR, 0);
        let mut yb = y + BAR_DY;
        for (k, half) in [hw, hw2].into_iter().enumerate() {
            if k == 1 {
                if half == 0 { break; }
                yb += BAR_ROW_DY;
            }
            out.push(Self::sprite(cap, x + half, yb, BAR_TILE, BAR_TILE, bar_a, Rot::R180));
            out.push(Self::sprite(mid, x - half, yb, half << 1, BAR_TILE, bar_a, Rot::None));
            out.push(Self::sprite(cap, x - half - BAR_TILE, yb, BAR_TILE, BAR_TILE, bar_a, Rot::None));
        }
        let glow_a = ((p[1] >> 4) as f32 * f) as i32;
        for (n, &(ox, oy)) in orbs.iter().enumerate().take(max_hp.max(0) as usize) {
            self.hud_frame(i, self.assets.icon_frame(icon::ORB, fr), x + ox, y + oy, 1, a_orb, out);
            if fr != 30 {
                self.hud_frame(i, self.assets.icon_frame(icon::ORB, 30), x + ox, y + oy, 1, a_orb, out);
                out.push(Self::sprite(self.assets.icon_frame(icon::ORB, 31), x + ox + 14, y + oy - 17, 34, 34, glow_a, Rot::None));
                fr = if s.value == n as i32 + 1 { 30 } else { (fr + 6) % 30 };
            }
        }
    }

    /// 0x24ea00.
    fn draw_bolts(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if s.slide == 0 { return; }
        let y = ELEMENT_Y;
        let x = ANCHORS[i].0;
        let (sf, f) = Self::fractions(s, BOLT_SLIDE_STEPS, BOLT_ALPHA_STEPS);
        let bar_a = ((sf * 128.0) as i32 as f32 * 0.7) as i32;
        let bolts = self.inputs.bolts;
        let mut digits = 1;
        let mut v = bolts;
        while v > 9 {
            digits += 1;
            v /= 10;
        }
        let w = ((DIGIT_W * digits) as f32 * sf) as i32;
        let left = x - 0x1c - w;
        let (cap, mid) = (self.assets.icon_frame(icon::BAR, 1), self.assets.icon_frame(icon::BAR, 0));
        out.push(Self::sprite(cap, x - 12, y, 32, 32, bar_a, Rot::R180));
        out.push(Self::sprite(mid, left, y, w + 16, 32, bar_a, Rot::None));
        out.push(Self::sprite(cap, left - 32, y, 32, 32, bar_a, Rot::None));
        self.hud_frame(i, self.assets.icon_frame(icon::BOLT, (self.bolt_anim as i32) >> 1), x - 32, y, 0, 0x80, out);
        let colour = tween_color(f, TEXT_FROM, TEXT_TO);
        let shadow = tween_color(f, 0, 0x8000_0000);
        let number = bolts.to_string().into_bytes();
        let (tx, ty) = (x + BOLT_TEXT_D.0, y + BOLT_TEXT_D.1);
        self.text_right(tx + 1, ty + 1, shadow, &number, out);
        self.text_right(tx, ty, colour, &number, out);
        let german_italian = self.inputs.lang == strings::lang::GERMAN || self.inputs.lang == strings::lang::ITALIAN;
        let (sep, (dx, dy)): (&[u8], _) = if german_italian { (b".", SEP_DOT_D) } else { (b"'", SEP_APOSTROPHE_D) };
        let mut k = 3;
        while k < digits {
            self.text_right(x + dx - DIGIT_W * k + 2, y + dy + 2, shadow, sep, out);
            self.text_right(x + dx - DIGIT_W * k, y + dy, colour, sep, out);
            k += 3;
        }
    }

    /// 0x24c898 (Lombyte `HudRaceTimerDraw`): the prompt text 0x17e9b0 in a bar frame, centred on x 256, at
    /// y = 0x15f770 (32) + 18 (NTSC; 10 PAL). Alpha `trunc(128·slide/8)`, text colour `A << 24 | 0x40f040`
    /// (green), regular font. A byte 0x01 splits it into two lines (frame 54 tall, second line 19 lower). The
    /// Rilgar / Kalebo III best-time lines under it (levels 5 and 16, within 5 of the race start) are not ported.
    fn draw_prompt(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        let y = 32 + ELEMENT_Y;
        let f = (s.slide as f32 / scale_ticks(8) as f32).clamp(0.0, 1.0);
        let a = (f * 128.0) as i32;
        let colour = (a as u32) << 24 | 0x0040_f040;
        let text = &self.prompt_text;
        let (mid, cap) = (self.assets.icon_frame(icon::BAR, 0), self.assets.icon_frame(icon::BAR, 1));
        let width = |t: &[u8]| self.assets.text_width(Font::Regular, t);
        let centred = |t: &[u8], y: i32, out: &mut Vec<Draw>| out.push(Draw::Text { font: Font::Regular, x: 0x100 - (width(t) >> 1), y, rgba: colour, text: t.to_vec() });
        // The scan stops at the first byte ≤ 1 from index 1 on.
        let mut k = 0;
        if text.first().is_some_and(|&c| c > 1) {
            k = 1;
            while k < 0x80 && text.get(k).is_some_and(|&c| c > 1) { k += 1; }
        }
        if text.get(k) == Some(&1) {
            let (l1, l2) = (&text[..k], &text[k + 1..]);
            let (w1, w2) = (width(l1), width(l2));
            let (x1, x2) = (0xe0 - (w1 >> 1), 0xe0 - (w2 >> 1));
            let (x, w) = if x2 <= x1 { (x2, w2) } else { (x1, w1) };
            out.push(Self::sprite(mid, x + 0x20, y, w, 0x36, a, Rot::None));
            out.push(Self::sprite(cap, x, y, 0x20, 0x36, a, Rot::None));
            out.push(Self::sprite(cap, x + w + 0x20, y, 0x20, 0x36, a, Rot::R180));
            centred(l1, y + 8, out);
            centred(l2, y + 0x1b, out);
        } else {
            let w = width(text);
            let x = 0xe0 - (w >> 1);
            out.push(Self::sprite(cap, x, y, 0x20, 0x20, a, Rot::None));
            out.push(Self::sprite(mid, 0x100 - (w >> 1), y, w, 0x20, a, Rot::None));
            out.push(Self::sprite(cap, x + w + 0x20, y, 0x20, 0x20, a, Rot::R180));
            centred(text, y + 8, out);
        }
    }

    /// 0x24f3b0. The middle bar's width is `x + len` (x = the anchor, 20) and the right cap follows it, so the
    /// bar is `x` pixels longer than `len` would suggest; that is what the code draws.
    fn draw_weapon(&self, i: usize, out: &mut Vec<Draw>) {
        let s = &self.slots[i];
        if s.slide == 0 { return; }
        let y = ELEMENT_Y;
        let x = ANCHORS[i].0;
        let (sf, f) = Self::fractions(s, WEAPON_STEPS, WEAPON_STEPS);
        let a = (sf * 128.0) as i32;
        let bar_a = (a as f32 * 0.7) as i32;
        let len = WEAPON_LEN[(s.max > 99) as usize];
        let lw = (len as f32 * sf) as i32;
        let (cap, mid) = (self.assets.icon_frame(icon::BAR, 1), self.assets.icon_frame(icon::BAR, 0));
        out.push(Self::sprite(cap, x - 0x1c, y, 32, 32, bar_a, Rot::None));
        out.push(Self::sprite(mid, x + 4, y, x + lw, 32, bar_a, Rot::None));
        out.push(Self::sprite(cap, x + 4 + x + lw, y, 32, 32, bar_a, Rot::R180));
        self.hud_frame(i, self.assets.icon_frame(s.icon, 3), x, y, 0, a, out);
        let text = format!("{}/{}", s.shown, s.max).into_bytes();
        let colour = if s.shown != 0 { tween_color(f, TEXT_FROM, TEXT_TO) } else { tween_color(f, EMPTY_FROM, EMPTY_TO) };
        let shadow = tween_color(f, 0, 0x8000_0000);
        let (tx, ty) = (x + WEAPON_TEXT_D.0 + len, y + WEAPON_TEXT_D.1);
        self.text_right(tx + 1, ty + 1, shadow, &text, out);
        self.text_right(tx, ty, colour, &text, out);
    }

    /// The banner part of 0x24fb50 (ramp ±0x80/ScaleTicks(8) per tick) and its draw 0x251b88.
    fn draw_banner(&mut self, out: &mut Vec<Draw>) {
        let b = &mut self.banner;
        if b.countdown == 0 && b.alpha == 0 {
            b.y = 100;
            return;
        }
        let step = 0x80 / scale_ticks(8);
        if b.countdown == 0 {
            b.alpha -= step;
            if b.alpha < 0 { b.alpha = 0; }
        } else {
            b.alpha = (b.alpha + step).min(0x80);
        }
        let colour = (b.alpha as u32) << 24 | 0x00f0_f0f0;
        let (x, y, text) = (0x100, b.y, b.text.clone());
        if b.countdown != 0 { b.countdown -= 1; }
        // 0x251b88: shadow, bar frame behind the text (alpha ≤ 0x50), text.
        let a = ((colour as i32) >> 24).min(0x50);
        let w = self.assets.text_width(Font::Large, &text);
        let left = (x + 1) - (w >> 1);
        out.push(Draw::Text { font: Font::Large, x: left, y: y + 1, rgba: colour & 0xff00_0000, text: text.clone() });
        self.stretch_frame(left - 0x20, y - 8, (x - (left - 0x20)) * 2, 0x20, a, out);
        out.push(Draw::Text { font: Font::Large, x: x - (w >> 1), y, rgba: colour, text });
    }

    /// `draw_stretchable_ui_frame` 0x251ab0: cap, stretched middle, rotated cap.
    pub fn stretch_frame(&self, x: i32, y: i32, w: i32, h: i32, alpha: i32, out: &mut Vec<Draw>) {
        let (mid, cap) = (self.assets.icon_frame(icon::BAR, 0), self.assets.icon_frame(icon::BAR, 1));
        out.push(Self::sprite(cap, x, y, 0x20, h, alpha, Rot::None));
        out.push(Self::sprite(mid, x + 0x20, y, w - 0x40, h, alpha, Rot::None));
        out.push(Self::sprite(cap, x + w - 0x20, y, 0x20, h, alpha, Rot::R180));
    }

    /// 0x2266c0.
    fn draw_help(&mut self, out: &mut Vec<Draw>) {
        let text = self.help_text().to_vec();
        let h = &mut self.help;
        if h.state == 0 || !h.enabled { return; }
        let frame = |top, bottom, left, right, alpha| Draw::UiFrame { top, bottom, left, right, alpha };
        let logo = |cx: i32, cy: i32, a: i32| Draw::FxQuad { fx: 4, x: cx - 0x20, y: cy - 0x20, w: 0x40, h: 0x40, u: 0, v: 0, tw: 0x40, th: 0x40, rgba: (a as u32) << 24 | 0x0080_8080 };
        match h.state {
            1 => {
                h.cur_w = h.t * 4 + 8;
                h.cur_h = h.cur_w;
                out.push(frame(h.cy - h.cur_w, h.cy + h.cur_w, h.cx - h.cur_w, h.cx + h.cur_w, 0x60));
            }
            2 => {
                // Help_DrawPrompt 0x2265d8: logo alpha 0x7e in this state.
                h.cur_w = 0x20;
                h.cur_h = 0x20;
                out.push(frame(h.cy - 0x20, h.cy + 0x20, h.cx - 0x20, h.cx + 0x20, 0x60));
                out.push(logo(h.cx, h.cy, 0x7e));
            }
            3 => {
                h.cur_w = (h.half_w - 0x20) * h.t / 8 + 0x20;
                h.cur_h = (h.half_h - 0x20) * h.t / 8 + 0x20;
                out.push(frame(h.cy - h.cur_h, h.cy + h.cur_h, h.cx - h.cur_w, h.cx + h.cur_w, 0x60));
                out.push(logo(h.cx, h.cy, ((8 - h.t) << 4).max(0)));
            }
            4..=6 => {
                h.cur_w = h.half_w;
                h.cur_h = h.half_h;
                out.push(frame(h.cy - h.half_h, h.cy + h.half_h, h.cx - h.half_w, h.cx + h.half_w, 0x60));
                let rgba = match h.state {
                    4 => HELP_COLOUR | (h.t as u32) << 29,
                    6 => HELP_COLOUR | ((4 - h.t) as u32) << 29,
                    _ => 0x80ff_a888,
                };
                out.push(Draw::TextWindow { font: Font::Small, window: help_window(h.cy, 3), rgba, text });
            }
            7 => {
                let hh = h.cur_h - (h.cur_h - 8) * h.t / 8;
                let hw = h.cur_w - (h.cur_w - 8) * h.t / 8;
                out.push(frame(h.cy - hh, h.cy + hh, h.cx - hw, h.cx + hw, (8 - h.t) * 12));
            }
            _ => {}
        }
    }
}

/// `FontPrintWindow`'s layout (0x21db48): line breaking, balancing and placement. The engine draws the lines
/// with `FontPrint` inside the window's scissor; the help box uses it to measure.
pub mod text {
    use rc_formats::font::{measure_text_width, GlyphTable};

    /// Flags (window +0x12).
    pub const CENTRE_LINES: u16 = 1;
    pub const CENTRE_BLOCK: u16 = 2;
    pub const MEASURE_ONLY: u16 = 4;
    /// Sub-pixel float path (`0x21d0f0`); not used by the ported callers, drawn like the integer path.
    pub const FLOAT_POS: u16 = 8;

    /// The window record (shorts, `FontSetWindow` 0x21e120).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Window {
        pub y_min: i16,
        pub y_max: i16,
        pub x_min: i16,
        pub x_max: i16,
        pub x_anchor: i16,
        pub y_start: i16,
        /// Out: widest drawn line.
        pub max_width: i16,
        /// Out: lines × line height.
        pub height: i16,
        pub line_height: i16,
        pub flags: u16,
        /// 1/16-pixel offsets of the float path.
        pub sub_x: i16,
        pub sub_y: i16,
    }

    impl Window {
        /// `FontSetWindow(win, y_min, y_max, x_min, x_max, x_anchor, y_start, line_height, flags)`.
        #[allow(clippy::too_many_arguments)]
        pub fn new(y_min: i16, y_max: i16, x_min: i16, x_max: i16, x_anchor: i16, y_start: i16, line_height: i16, flags: u16) -> Self {
            Window { y_min, y_max, x_min, x_max, x_anchor, y_start, max_width: 0, height: 0, line_height, flags, sub_x: 0, sub_y: 0 }
        }
    }

    /// A line `FontPrintWindow` draws: `count` bytes from `start`, pen at (x, y), starting colour slot
    /// `colour` of the font colour table (0 = the caller's colour).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Line {
        pub start: usize,
        /// Bytes to print (`FontPrint`'s len: ≤ 0 prints to the NUL, like the game).
        pub count: i32,
        pub colour: u8,
        pub x: i32,
        pub y: i32,
        pub width: i32,
    }

    /// Wraps `text` (first `len` bytes, all when `len < 0`), fills `win.max_width` / `win.height` and returns
    /// the lines inside the window's y range (all of them, also with [`MEASURE_ONLY`]). `colour_codes` is the
    /// game's colour-code switch 0x15f45c (1 during play).
    pub fn layout(win: &mut Window, text: &[u8], len: i32, glyphs: &GlyphTable, colour_codes: bool) -> Vec<Line> {
        let at = |i: i32| -> u8 { if i < 0 { 0 } else { text.get(i as usize).copied().unwrap_or(0) } };
        let adv = |c: u8| -> i32 { glyphs.get(c as usize).map_or(0, |g| g.advance as i32) };
        let (x_min, x_max, anchor) = (win.x_min as i32, win.x_max as i32, win.x_anchor as i32);
        let w0 = if win.flags & CENTRE_LINES != 0 { 2 * (x_max - anchor).min(anchor - x_min) } else { x_max - anchor };
        let mut colour = 0i32;
        let (mut final_pass, mut n0, mut last_w, mut w) = (false, 0usize, 0i32, w0);
        // (start, end inclusive, colour slot at the line start)
        let mut lines: Vec<(i32, i32, i32)>;
        loop {
            lines = Vec::new();
            let mut pos = 0i32;
            if len != 0 && at(0) != 0 {
                loop {
                    let start = pos;
                    let line_colour = colour;
                    let mut brk = pos;
                    let mut width = 0i32;
                    if w > 0 {
                        loop {
                            let c = at(pos);
                            if c == 0x20 || c < 0x10 { brk = pos; }
                            if colour_codes && c.wrapping_sub(8) < 8 { colour = c as i32 - 8; }
                            if c < 2 { break; }
                            pos += 1;
                            width += adv(c);
                            if width >= w { break; }
                        }
                    }
                    let mut end = if brk as i16 == start as i16 { pos } else { brk };
                    let e = end;
                    let ce = at(e);
                    if ce == 0x20 || ce < 0x10 { end -= 1; }
                    lines.push((start, end, line_colour));
                    if ce == 0 {
                        last_w = width;
                        break;
                    }
                    pos = e + 1;
                    if pos == len || at(pos) == 0 { break; }
                }
            }
            let n = lines.len();
            if final_pass { break; }
            if n0 == 0 { n0 = n; }
            if n < 2 { break; }
            if n0 < n {
                final_pass = true;
                w = w0;
                continue;
            }
            let third = w / 3;
            w -= 16;
            if last_w >= third { break; }
        }

        let lh = win.line_height as i32;
        let total = lines.len() as i32 * lh;
        win.max_width = 0;
        win.height = total as i16;
        let mut y = win.y_start as i32;
        if win.flags & CENTRE_BLOCK != 0 { y -= total >> 1; }
        let mut out = Vec::new();
        for &(start, end, col) in &lines {
            if !(y + lh < win.y_min as i32 || (win.y_max as i32) < y) {
                let count = end - start + 1;
                let from = text.get(start.max(0) as usize..).unwrap_or(&[]);
                let width = measure_text_width(from, count, glyphs);
                if (win.max_width as i32) < width { win.max_width = width as i16; }
                let x = if win.flags & CENTRE_LINES != 0 { anchor - (width >> 1) } else { anchor };
                out.push(Line { start: start.max(0) as usize, count, colour: col as u8, x, y, width });
            }
            y += lh;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::font::{Glyph, GLYPHS};

    fn glyphs() -> GlyphTable {
        let mut t = [Glyph::default(); GLYPHS];
        for g in &mut t[0x21..0x7b] { g.advance = 10; }
        t[b' ' as usize].advance = 5;
        for d in b'0'..=b'9' { t[d as usize].advance = 13; }
        t[b'\'' as usize].advance = 4;
        t[b'.' as usize].advance = 6;
        t
    }

    fn assets() -> HudAssets {
        let mut icons = vec![IconEntry { id: 0, frame_count: 1, first_frame: 0, ..Default::default() }];
        let mut next = 1u16;
        for (id, n) in [(icon::ORB, 32u16), (icon::BOLT, 30), (icon::BAR, 2), (60010, 5)] {
            icons.push(IconEntry { id, frame_count: n, first_frame: next, ..Default::default() });
            next += n;
        }
        icons.push(IconEntry { id: 0xffff, ..Default::default() });
        let g = glyphs();
        let messages = vec![Message { id: 1000, text: b"Gadgetron \x0cInfobots\x08 give you coordinates for new planets.".to_vec(), help_audio: 4 }];
        HudAssets { icons, frame_sizes: vec![(32, 32); next as usize], glyphs: [g, g, g], messages }
    }

    fn inputs(hp: i32, bolts: i32) -> Inputs { Inputs { hp, max_hp: 4, bolts, weapon: None, lang: 0 } }

    /// The context prompt (slot 12, 0x24c898): requested while an owner holds it, slides in over 8 ticks (alpha
    /// 16·slide), one or two lines in a bar frame centred on x 256 at y 50, and fades out once no longer requested.
    #[test]
    fn prompt_slot_12_shows_while_requested() {
        let mut h = HudState::new(assets());
        h.set_prompt(true, b"\x12 Activate");
        let mut d = Vec::new();
        for _ in 0..60 { d = h.tick(inputs(4, 0)); }
        let s = &h.slots[12];
        assert_eq!((s.element, s.slide), (Element::Prompt, 8));
        let text: Vec<_> = d.iter().filter_map(|x| if let Draw::Text { y, rgba, text, .. } = x { Some((*y, *rgba, text.clone())) } else { None }).collect();
        assert_eq!(text, vec![(58, 0x8040_f040, b"\x12 Activate".to_vec())]);
        let sprites = d.iter().filter(|x| matches!(x, Draw::Sprite { y: 50, h: 32, alpha: 128, .. })).count();
        assert_eq!(sprites, 3, "caps and middle");
        // Two lines split at 0x01: frame 54 tall, the second line 19 lower.
        h.set_prompt(true, b"Buy\x01now");
        let d = h.tick(inputs(4, 0));
        let ys: Vec<i32> = d.iter().filter_map(|x| if let Draw::Text { y, .. } = x { Some(*y) } else { None }).collect();
        assert_eq!(ys, vec![58, 77]);
        assert!(d.iter().any(|x| matches!(x, Draw::Sprite { h: 0x36, .. })));
        // No longer requested: the timer (≥ 10) runs out, then alpha and slide ramp down.
        h.set_prompt(false, b"");
        for _ in 0..60 { h.tick(inputs(4, 0)); }
        assert_eq!(h.slots[12].slide, 0);
    }

    #[test]
    fn nothing_on_screen_until_something_changes() {
        let mut h = HudState::new(assets());
        for _ in 0..300 { assert!(h.tick(inputs(4, 0)).is_empty()); }
    }

    /// First pickup: the counter is armed (init timer ScaleTicks(120)+30 = 150), slides in over 8 ticks, fades
    /// in over 8, holds, and ramps out (alpha first) once the timer drops below 5.
    #[test]
    fn bolt_ramp_timings() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        let mut seen = Vec::new();
        for k in 0..200 {
            h.tick(inputs(4, if k == 0 { 0 } else { 5 }));
            let s = h.slots[2];
            seen.push((s.slide, s.alpha));
        }
        // Armed on tick 1 (the change): applied the same tick (slot hidden, counter −6), slide 1.
        assert_eq!(seen[1], (1, 0));
        assert_eq!(seen[8], (8, 0));
        assert_eq!(seen[9], (8, 1));
        assert_eq!(seen[16], (8, 8));
        // Timer 150 at arming, −1 per later tick: < 5 from 146 ticks after arming.
        assert_eq!(seen[1 + 145], (8, 8));
        assert_eq!(seen[1 + 146], (8, 7));
        assert_eq!(seen[1 + 153], (8, 0));
        assert_eq!(seen[1 + 154], (7, 0));
        assert_eq!(seen[1 + 161], (0, 0));
        assert_eq!(h.slots[2].counter, -6);
    }

    /// Later changes show the counter for 90 ticks (the update's ScaleTicks(90)).
    #[test]
    fn bolt_visible_90_ticks_after_a_change() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        for _ in 0..200 { h.tick(inputs(4, 1)); }
        assert_eq!((h.slots[2].slide, h.slots[2].alpha), (0, 0));
        let mut frames_with_draws = 0;
        let mut full = Vec::new();
        for k in 0..200 {
            let d = h.tick(inputs(4, 1234));
            if !d.is_empty() { frames_with_draws += 1; }
            if (h.slots[2].slide, h.slots[2].alpha) == (8, 8) { full.push(k); }
        }
        // Change seen at k = 0: timer 90, then 89, 88, …; below 5 from k = 86.
        assert_eq!((full[0], *full.last().unwrap()), (15, 85));
        // Alpha 7 at k = 86 … 0 at 93, slide 7 at 94 … 0 at 101: drawn (slide > 0) for k = 0 ..= 100.
        assert_eq!(frames_with_draws, 101);
    }

    #[test]
    fn health_shows_120_ticks_after_damage_and_always_at_one_hp() {
        let mut h = HudState::new(assets());
        h.tick(inputs(4, 0));
        h.tick(inputs(3, 0));
        // Init: timer ScaleTicks(180) + 30.
        assert_eq!((h.slots[1].element, h.slots[1].timer, h.slots[1].offset), (Element::Health, 210, (32, 0)));
        for _ in 0..400 { h.tick(inputs(3, 0)); }
        assert_eq!(h.slots[1].slide, 0);
        for _ in 0..400 { h.tick(inputs(1, 0)); }
        assert_eq!((h.slots[1].slide, h.slots[1].alpha), (8, 8), "1 HP keeps the timer at 120");
    }

    fn texts(d: &[Draw]) -> Vec<(i32, i32, Vec<u8>)> {
        d.iter().filter_map(|x| match x { Draw::Text { x, y, text, .. } => Some((*x, *y, text.clone())), _ => None }).collect()
    }

    #[test]
    fn bolt_separators_english_and_german() {
        for (lang, sep, (dx, dy)) in [(strings::lang::ENGLISH, b'\'', (-30, 21)), (strings::lang::GERMAN, b'.', (-29, 10))] {
            let mut h = HudState::new(assets());
            h.tick(Inputs { lang, ..inputs(4, 0) });
            let mut d = Vec::new();
            for _ in 0..20 { d = h.tick(Inputs { lang, ..inputs(4, 1_234_567) }); }
            let t = texts(&d);
            // Number: shadow + text, right-aligned at x − 32 (width 7 × 13).
            assert_eq!(t[1], (492 - 32 - 91, 18 + 8, b"1234567".to_vec()));
            assert_eq!(t[0], (492 - 32 - 91 + 1, 18 + 8 + 1, b"1234567".to_vec()));
            // Separators after the 3rd and 6th digit from the right: shadow at +2, +2.
            let w = if sep == b'.' { 6 } else { 4 };
            let seps: Vec<_> = t[2..].to_vec();
            assert_eq!(seps.len(), 4);
            for (j, k) in [3, 6].into_iter().enumerate() {
                assert_eq!(seps[2 * j], (492 + dx - 13 * k + 2 - w, 18 + dy + 2, vec![sep]));
                assert_eq!(seps[2 * j + 1], (492 + dx - 13 * k - w, 18 + dy, vec![sep]));
            }
        }
    }

    #[test]
    fn wrench_hides_the_weapon_slot_and_ammo_weapon_persists() {
        let mut h = HudState::new(assets());
        for _ in 0..50 { h.tick(Inputs { weapon: Some((10, 25, 40)), ..inputs(4, 0) }); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha, h.slots[0].shown), (8, 8, 25));
        for _ in 0..400 { h.tick(Inputs { weapon: Some((10, 25, 40)), ..inputs(4, 0) }); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha), (8, 8), "flag 0x10 keeps it up");
        for _ in 0..40 { h.tick(inputs(4, 0)); }
        assert_eq!((h.slots[0].slide, h.slots[0].alpha, h.slots[0].flags), (0, 0, 0));
    }

    #[test]
    fn help_box_states_and_timings() {
        let mut h = HudState::new(assets());
        assert!(h.help_request(1000));
        let mut states = Vec::new();
        for _ in 0..500 {
            h.tick(inputs(4, 0));
            states.push(h.help.state);
        }
        let first = |s: u8| states.iter().position(|&x| x == s).unwrap();
        assert_eq!([first(1), first(2), first(3), first(4), first(5), first(6), first(7)], [0, 6, 30, 38, 42, 462, 466]);
        assert_eq!(states[474], 0);
        // Box centred at (256, H − 60) with the measured half-size.
        assert_eq!((h.help.cx, h.help.cy), (256, 356));
    }

    #[test]
    fn window_wrap_balances_short_last_lines() {
        let g = glyphs();
        // Anchor 100, width 2·min(100, 100) = 200: "aaaa…" words of 5 letters (50 px) + space (5 px).
        let text = b"aaaaa aaaaa aaaaa aaaaa b";
        let mut w = text::Window::new(0, 400, 0, 200, 100, 50, 16, text::CENTRE_LINES);
        let lines = text::layout(&mut w, text, -1, &g, true);
        // Greedy at 200: 3 words + space (165 px), then "aaaa" overflows → break at the space: [3 words] (160 px),
        // [1 word + "b"] (65 px); 65 < 200/3 → retry at 184: same lines, 65 ≥ 184/3 → kept.
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].count, lines[1].start, lines[1].count), (17, 18, 7));
        assert_eq!(lines[0].x, 100 - (160 >> 1));
        assert_eq!((w.max_width, w.height), (160, 32));
        // Four words at W = 180: [3 words], [1 word] (50 < 60) → 164 (50 < 54) → 148: [2 words], [2 words].
        let text = b"aaaaa aaaaa aaaaa aaaaa";
        let mut w = text::Window::new(0, 400, 0, 180, 90, 50, 16, text::CENTRE_LINES);
        let lines = text::layout(&mut w, text, -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.width)).collect::<Vec<_>>(), [(0, 11, 105), (12, 11, 105)]);
        // Newline 0x01 forces a break. The colour slot is not reset between balancing passes (s2 is zeroed
        // before the pass loop only): the short last line makes the game re-wrap, and the second pass starts
        // line 0 with the 0x0c seen at the end of the first.
        let mut w = text::Window::new(0, 400, 0, 400, 0, 0, 16, 0);
        let lines = text::layout(&mut w, b"ab\x01\x0ccd", -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.colour)).collect::<Vec<_>>(), [(0, 2, 4), (3, 3, 4)]);
        let mut w = text::Window::new(0, 400, 0, 400, 0, 0, 16, 0);
        let lines = text::layout(&mut w, b"abcdefghijklmn\x01\x0cxy\x08z", -1, &g, true);
        assert_eq!(lines.iter().map(|l| (l.start, l.count, l.colour)).collect::<Vec<_>>(), [(0, 14, 0), (15, 5, 0)]);
    }
}
