//! In-level menus: the mode system, the quick-select ring and the page-menu machinery of mode 3 (pause menu,
//! Options, map, ship planet select). Spec: `docs/plan/menus.md` §1–§3; addresses are level01.elf.
//!
//! Everything here is a pure state machine: one call per game frame with the pad ([`MenuInput`]) and the game
//! state, returning the frame's 2D draw calls ([`MenuDraw`]) in the game's order, plus sound / effect events.
//! The engine (crates/rc-engine/src/menu_render.rs) turns the draws into primitives of the HUD 2D pass.
//!
//! Disc data (page, widget and item records, neighbour tables, gp constants, item tables) is read at run time
//! from the level overlay through [`Overlay`]; nothing of it is compiled in.

pub mod mode;
pub mod pause;
pub mod quick_select;
pub mod vendor;

use crate::hud::{Draw, HudAssets, Rot};
use crate::pad::PadState;
use crate::ps2v::Pf;
use rc_formats::font::{measure_text_width, parse_overlay_sections, read_overlay, Font, OverlaySection};
use rc_formats::strings::{self, Message};

/// The level overlay as the EE sees it (loaded sections; `.bss` reads as absent), with the level-01 → this level
/// address map of the menu data (docs/plan/level_generalisation.md M1–M7).
///
/// The menu code names its records, tables and callbacks by their level-01 addresses (labels). On another level
/// they sit elsewhere: [`Overlay::relocated`] finds them (`rc_formats::level_overlay::Relocation`: the code that
/// forms each root address, `lui`/`%lo` or `$gp`-relative, matched in this level's overlay; then the page tree
/// walked in step with level 01's, record by record, which also pairs every widget callback with its label).
/// [`Overlay::at`] gives this level's address of a label (records and tables are then read at it), and
/// [`Overlay::label`] the label of one of this level's callbacks (widgets dispatch on labels). Level 01 itself,
/// and an overlay made by [`Overlay::parse`], map every address to itself.
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    sections: Vec<OverlaySection>,
    /// Label → this level's address (data).
    at: std::collections::HashMap<u32, u32>,
    /// This level's callback address → its label.
    label: std::collections::HashMap<u32, u32>,
}

impl Overlay {
    /// From the raw overlay lump (`LevelFiles::overlay`, `levels/NN/overlay.bin`), addresses as they are (level 01).
    pub fn parse(bytes: &[u8]) -> rc_formats::buf::Result<Overlay> { Ok(Overlay::from_sections(parse_overlay_sections(bytes)?)) }
    pub fn from_sections(sections: Vec<OverlaySection>) -> Overlay { Overlay { sections, ..Default::default() } }

    /// This level's overlay `bytes` with its address map against the level-01 overlay `reference` (type doc).
    pub fn relocated(bytes: &[u8], reference: &[u8]) -> rc_formats::buf::Result<Overlay> {
        use rc_formats::level_overlay::{LevelOverlay, Relocation};
        let mut ov = Overlay::parse(bytes)?;
        let (target, refo) = (LevelOverlay::parse(bytes)?, LevelOverlay::parse(reference)?);
        let rel = Relocation::new(&refo, &target);
        if rel.is_identity() { return Ok(ov); }
        let reference_ov = Overlay::parse(reference)?;
        for &a in DATA_LABELS.iter().chain(vendor::layout::DATA_LABELS) {
            if let Some(b) = rel.data(a) { ov.at.insert(a, b); }
        }
        let roots: Vec<(u32, u32)> = pause::page::ROOTS.iter().filter_map(|&a| Some((a, rel.data(a)?))).collect();
        let (records, callbacks) = pause::correlate(&reference_ov, &ov, &roots);
        ov.at.extend(records);
        ov.label = callbacks;
        Ok(ov)
    }

    /// This level's address of the level-01 address `label` (itself when unmapped).
    pub fn at(&self, label: u32) -> u32 { self.at.get(&label).copied().unwrap_or(label) }
    /// Whether `label` was mapped (always true on level 01).
    pub fn maps(&self, label: u32) -> bool { self.at.is_empty() || self.at.contains_key(&label) }
    /// The level-01 label of this level's callback `addr` (itself when unmapped).
    pub fn label(&self, addr: u32) -> u32 { self.label.get(&addr).copied().unwrap_or(addr) }

    pub fn bytes(&self, addr: u32, n: usize) -> Option<&[u8]> { read_overlay(&self.sections, addr, n) }
    /// The loaded sections (code pattern searches).
    pub fn sections(&self) -> &[OverlaySection] { &self.sections }
    pub fn u8(&self, a: u32) -> Option<u8> { self.bytes(a, 1).map(|b| b[0]) }
    pub fn u16(&self, a: u32) -> Option<u16> { self.bytes(a, 2).map(|b| u16::from_le_bytes([b[0], b[1]])) }
    pub fn i16(&self, a: u32) -> Option<i16> { self.u16(a).map(|v| v as i16) }
    pub fn u32(&self, a: u32) -> Option<u32> { self.bytes(a, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
    pub fn i32(&self, a: u32) -> Option<i32> { self.u32(a).map(|v| v as i32) }
    /// A float constant as its PS2 bit pattern.
    pub fn pf(&self, a: u32) -> Option<Pf> { self.u32(a).map(Pf::b) }
}

/// The level-01 data addresses the menus read that code forms directly (the page records are found by the
/// page walk): the level / planet name table 0x1c22c0 and the galaxy points 0x1c23a8, the planet name offset
/// 0x15f650, the quick-select gp block 0x15f718 and d-pad defaults 0x17e098, the frame's corner lists 0x161fe0
/// and light 0x160280 / 0x160290, the menu constants 0x160270..0x160328, the Gadgets page's item preview table
/// 0x1c4988 and grid cell size 0x160350 / 0x160354.
pub const DATA_LABELS: &[u32] = &[
    0x1c22c0, pause::PLANET_POINTS_BASE, 0x15f650, quick_select::GP_BASE, quick_select::DPAD_DEFAULTS, pause::frame::CORNER_LISTS_ADDR,
    pause::frame::LIGHT_DIR_ADDR, pause::frame::LIGHT_COLOR_ADDR, 0x160270, 0x160274, 0x160278, 0x16027c, 0x160318, 0x160328,
    pause::gadgets::PREVIEW_TABLE, pause::gadgets::CELL_W, pause::gadgets::CELL_H,
];

/// The pad fields the menus read (the `PAD` record at 0x13c940 after this frame's `UpdatePad`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuInput {
    /// 0x13cae0 (+0x1a0) held, 0x13cae4 (+0x1a4) pressed (after the pad lock).
    pub held: u32,
    pub pressed: u32,
    /// 0x13caf4 (+0x1b4) pressed edges of the buttons only, 0x13caf8 (+0x1b8) released buttons.
    pub raw_pressed: u32,
    pub raw_released: u32,
    /// 0x13cb00 (+0x1c0) held / 0x13cb04 (+0x1c4) pressed with the mirror undone and before the lock.
    pub held_u: u32,
    pub pressed_u: u32,
    /// 0x13ca88 / 0x13ca8c (+0x148 / +0x14c): left stick before the lock.
    pub stick_x: Pf,
    pub stick_y: Pf,
    /// 0x13cb18 (+0x1d8): the left stick is off centre.
    pub stick_active: bool,
    /// `0x13cadc != 0`: a pad is connected and readable.
    pub connected: bool,
}

impl MenuInput {
    /// The fields of `pad` (updated this frame); `connected` = the data block was read.
    pub fn from_pad(pad: &PadState, connected: bool) -> MenuInput {
        MenuInput {
            held: pad.held,
            pressed: pad.pressed,
            raw_pressed: pad.raw_pressed,
            raw_released: pad.raw_released,
            held_u: pad.held_unmirrored,
            pressed_u: pad.pressed_unmirrored,
            stick_x: pad.analog_copy[2],
            stick_y: pad.analog_copy[3],
            stick_active: pad.stick_active,
            connected,
        }
    }
}

/// One 2D draw call of the menus, in the game's order.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuDraw {
    /// A call of the HUD's 2D layer (`HudSprite`, `FontPrint`, `FontPrintWindow`, ...).
    Hud(Draw),
    /// `fun_00200e08(x0, y0, x1, y1, rgba, 0)` (boot): untextured SPRITE, corners at pixel x−1: covers pixels
    /// `x0−1 .. x1−2` × `y0−1 .. y1−2`.
    Rect { x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32 },
    /// `fun_00200c80(x0, y0, x1, y1, rgba, 0)`: a LINE with the same corner offset.
    Line { x0: i32, y0: i32, x1: i32, y1: i32, rgba: u32 },
    /// `fun_00200258` / `fun_00200958`: a HUD frame over (x0, y0)..(x1, y1) (1/16 pixel) with UVs in 1/16 texel;
    /// `repeat_u` = drawn with CLAMP_1 = 0 (REPEAT).
    SpriteUv { frame: usize, x0: i32, y0: i32, x1: i32, y1: i32, u0: i32, v0: i32, u1: i32, v1: i32, alpha: i32, repeat_u: bool },
    /// Full-screen: the frame-buffer snapshot taken when the menu opened (`FUN_002b4d38`).
    Snapshot,
    /// Full-screen black at `alpha` (`emit_rgba_draw_packet(0, 0, 0, a)`).
    Darken { alpha: i32 },
    /// A widget panel: the following draws are in panel-local pixels of a render target cleared to `clear`
    /// and copied 1:1 onto (x, y, w, h) (`PageMenuDraw` widget pass, blit mode 2). Ends at [`MenuDraw::PanelEnd`].
    PanelBegin { x: i32, y: i32, w: i32, h: i32, clear: u32 },
    PanelEnd,
    /// A content stub (streamed image, 3D globe / moby): nothing drawn; named for the log.
    Stub(&'static str),
}

/// Sound requests (`fun_0022da68(n, 0x11, moby)`: class-0x472 sound n; the ring plays none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuSound {
    Confirm = 0,
    Cursor = 1,
    Denied = 2,
    Open = 3,
    PageChange = 4,
}

/// What the menus read from the level: HUD icons / frame sizes, glyph tables, the level text, the overlay.
#[derive(Clone, Debug)]
pub struct MenuAssets {
    pub hud: HudAssets,
    pub overlay: Overlay,
}

impl MenuAssets {
    pub fn new(hud: HudAssets, overlay: Overlay) -> MenuAssets { MenuAssets { hud, overlay } }
    /// `msg_string__Fi(id)`; the port's own text ids ([`pause::port::text`], negative, never on the disc)
    /// resolve to the port's strings instead.
    pub fn msg(&self, id: i32) -> &[u8] { pause::port::text::get(id).unwrap_or_else(|| strings::lookup(&self.hud.messages, id)) }
    pub fn messages(&self) -> &[Message] { &self.hud.messages }
    /// `measure_text_width` in `font`.
    pub fn width(&self, font: Font, text: &[u8]) -> i32 { measure_text_width(text, -1, &self.hud.glyphs[font as usize]) }
    /// `GetIconFrame__Fii`.
    pub fn frame(&self, icon: u16, k: i32) -> usize { self.hud.icon_frame(icon, k) }
    pub fn frame_size(&self, frame: usize) -> (i32, i32) { self.hud.frame_sizes.get(frame).copied().unwrap_or((0, 0)) }
}

/// `ScaleTicks` (0x220e30) on NTSC: identity.
pub fn scale_ticks(n: i32) -> i32 { crate::hud::scale_ticks(n) }

/// `FastTweenColor`.
pub fn tween(t: f32, a: u32, b: u32) -> u32 { crate::hud::tween_color(t, a, b) }

/// `HudSprite(frame, x, y, w, h, alpha)`.
pub fn sprite(out: &mut Vec<MenuDraw>, frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32) {
    out.push(MenuDraw::Hud(Draw::Sprite { frame, x, y, w, h, alpha, rot: Rot::None }));
}

/// `FontPrint` at (x, y) (the right / centre wrappers already applied).
pub fn text(out: &mut Vec<MenuDraw>, font: Font, x: i32, y: i32, rgba: u32, t: &[u8]) {
    out.push(MenuDraw::Hud(Draw::Text { font, x, y, rgba, text: t.to_vec() }));
}

/// Text with the colour-code switch 0x15f45c off (`FUN_0021cc38` … `FUN_0021cc28`): the codes 0x08..0x0f are
/// dropped from the string, which is what `FontPrint` does with the switch off (no glyph, no advance).
pub fn text_plain(out: &mut Vec<MenuDraw>, font: Font, x: i32, y: i32, rgba: u32, t: &[u8]) {
    let s: Vec<u8> = t.iter().copied().filter(|c| c.wrapping_sub(8) >= 8).collect();
    text(out, font, x, y, rgba, &s);
}
