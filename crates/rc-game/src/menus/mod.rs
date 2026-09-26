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

use crate::hud::{Draw, HudAssets, Rot};
use crate::pad::PadState;
use crate::ps2v::Pf;
use rc_formats::font::{measure_text_width, parse_overlay_sections, read_overlay, Font, OverlaySection};
use rc_formats::strings::{self, Message};

/// The level overlay as the EE sees it (loaded sections; `.bss` reads as absent).
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    sections: Vec<OverlaySection>,
}

impl Overlay {
    /// From the raw overlay lump (`LevelFiles::overlay`, `levels/NN/overlay.bin`).
    pub fn parse(bytes: &[u8]) -> rc_formats::buf::Result<Overlay> { Ok(Overlay { sections: parse_overlay_sections(bytes)? }) }
    pub fn from_sections(sections: Vec<OverlaySection>) -> Overlay { Overlay { sections } }
    pub fn bytes(&self, addr: u32, n: usize) -> Option<&[u8]> { read_overlay(&self.sections, addr, n) }
    pub fn u8(&self, a: u32) -> Option<u8> { self.bytes(a, 1).map(|b| b[0]) }
    pub fn u16(&self, a: u32) -> Option<u16> { self.bytes(a, 2).map(|b| u16::from_le_bytes([b[0], b[1]])) }
    pub fn i16(&self, a: u32) -> Option<i16> { self.u16(a).map(|v| v as i16) }
    pub fn u32(&self, a: u32) -> Option<u32> { self.bytes(a, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) }
    pub fn i32(&self, a: u32) -> Option<i32> { self.u32(a).map(|v| v as i32) }
    /// A float constant as its PS2 bit pattern.
    pub fn pf(&self, a: u32) -> Option<Pf> { self.u32(a).map(Pf::b) }
}

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
