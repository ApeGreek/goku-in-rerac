//! The Gadgetron vendor, game mode 5: `OpenVendorMenu` 0x2ae1a0, `VendorBuildItemList` 0x2adef0,
//! `VendorModeUpdate` 0x2b03b8, the buy flow 0x2af7e8 (Lombyte `DrawSpriteHelper_C`), `VendorExit` 0x2ae660 and the
//! screen draws 0x2b1a48.. (level01). Spec: docs/plan/menus.md §4 and docs/plan/interaction.md §4.
//!
//! [`Vendor::frame`] is one main-loop frame of mode 5 (the game runs `VendorModeUpdate` once per frame). Tables come
//! from the level ([`VendorTables::load`]): the price records 0x1c4530 (`interact::ShopTable`), the item
//! definitions 0x179f40 (+0 name, +0x38 icon), the description ids (hologram table 0x1c2588, +0x10), the 24 ticker
//! lines 0x1ca538 and the ticker's LED font (glyph cells 0x1ca598, advances 0x1ca698, HUD icon 0xe935).
//!
//! **Substates** (0x1ca940, timer 0x1ca944):
//! * **0** fly-in: the screen fades from black (0x15f3fc = 1.0, −0.34 per frame; the open ran `FadeToBlack(4)`
//!   and cut the camera to 3.8 in front of the vendor, 1.5 up, facing it: [`vendor_camera`]); 40 frames, then
//!   substate 1, vendor seq 3 (blend 8, speed 1.0), class sound 4.
//! * **1** menu: Left / Right (0x13cb04 edges not seen last frame) move the selection (≤ 7 items: no wrap, cell
//!   x = 12 + 56·i; ≥ 8: a wrapping carousel whose offset 0x1ca98c moves ±56 and eases back 4 px per frame), each
//!   with sound 1, the entry's description on the ticker and the HUD's ammo slot for an ammo entry. ✕ on an
//!   unlocked entry → the buy flow (sound 0). △ → 8 frames, then substate 2 (sound 5).
//! * **Buy flow** (0x1ca99c): 1 opens the popup (class 0x471; here a panel): an ammo entry that is not full and
//!   whose unit price the bolts cover asks "How many?" (quantity 1.., Left / Right with auto-repeat after 16 held
//!   frames every 8th, after 48 every frame, capped at min(max − ammo, bolts / unit)); anything else asks
//!   "Purchase?" or shows why not (sound 2 when it cannot be bought). 2: ✕ confirms, △ cancels; 3: the popup
//!   closes; 4: the purchase — ammo: `AddAmmo` (overflow returned), bolts −= unit·(qty − overflow), stats
//!   0x13dd70 += bought; a weapon: `GiveItem(item, 1)` (owned, quick-select slot, vendor stock byte |= 0x40),
//!   bolts −= price; either → ticker 20319 "THANK YOU", sound 7, list rebuilt; short of bolts → ticker 20320,
//!   sound 0. A cancel plays sound 0.
//! * **2** leave: 40 frames (vendor seq 1), then `VendorExit`: mode 0, the HUD slots released, the vendor's
//!   state 1, sound 6. A weapon bought with a demo scene (0x1ca4a0) would play it first (substate 3): not ported.
//!
//! **Not ported**: the world update during substates 0 / 2 (the port freezes the world for those 80 frames), the
//! salesman video and its voice streams (class 0xc), the hologram and frame mobys, the item model on the screen
//! (the item's HUD icon is drawn in its place), the weapon demo scenes, the PDA's remote vendor. The screens are
//! drawn as 2D panels at fixed places ([`layout`]) instead of being mapped onto the vendor's monitor bones
//! (0x2b3130): docs/plan/interaction.md §4.4.

use crate::game_state::{GameState, SessionState};
use crate::hud::Draw;
use crate::menus::{sprite, text, MenuAssets, MenuDraw, MenuInput, Overlay};
use crate::moby_update::interact::ShopTable;
use crate::pad::button;
use crate::rng::Rng;
use rc_formats::font::Font;
use rc_formats::save_game::ItemTables;

/// Items with an entry in the tables.
pub const ITEMS: usize = 37;
/// Text ids.
pub mod msg {
    pub const AMMO: i32 = 20317;
    pub const THANK_YOU: i32 = 20319;
    pub const CANT_AFFORD_TICKER: i32 = 20320;
    pub const EXIT: i32 = 20192;
    pub const NO: i32 = 21067;
    pub const YES: i32 = 21070;
    pub const BUY: i32 = 21044;
    pub const BACK: i32 = 21043;
    pub const MAXED: i32 = 21046;
    pub const HOW_MANY: i32 = 21047;
    pub const CANT_AFFORD: i32 = 21048;
    pub const PURCHASE: i32 = 21049;
}
/// The ticker's LED font texture (`GetIconFrame(0xe935, 0)`).
pub const LED_ICON: u16 = 0xe935;
/// The ticker format 0x20a510: 18 spaces, then the message.
const TICKER_PAD: usize = 18;

/// Vendor class sound indices (`PlayClassSound(n, 0, vendor)`).
pub mod sound {
    pub const SELECT: u8 = 0;
    pub const CURSOR: u8 = 1;
    pub const DENIED: u8 = 2;
    pub const OPEN: u8 = 3;
    pub const SCREENS_ON: u8 = 4;
    pub const EXIT: u8 = 5;
    pub const CLOSED: u8 = 6;
    pub const PURCHASE: u8 = 7;
}

/// What the vendor reads from the level.
#[derive(Clone, Debug, Default)]
pub struct VendorTables {
    pub shop: ShopTable,
    /// Item definition +0x00: name id; +0x38: icon id.
    pub name: Vec<i32>,
    pub icon: Vec<u16>,
    /// Hologram table +0x10: the description the ticker shows.
    pub desc: Vec<i32>,
    /// 0x1ca538: the idle ticker lines.
    pub ticker: Vec<i32>,
    /// 0x1ca598 / 0x1ca698: LED font cell (−1: none; `(v & 0xffff) >> 4` = u, `v >> 20` = v) and advance of the
    /// characters 0x20..0x5f.
    pub led_cell: Vec<i32>,
    pub led_adv: Vec<i32>,
}

fn find(ov: &Overlay, pat: &[u8]) -> Option<u32> {
    ov.sections().iter().filter(|s| s.kind != 8).find_map(|s| s.data.windows(pat.len()).position(|w| w == pat).map(|i| s.dest + i as u32))
}

fn i32s(v: &[i32]) -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() }

impl VendorTables {
    /// From the level overlay: `item_defs` = the item definition table (`ItemTables::item_defs_addr`); the other
    /// tables by their bytes (the description ids of the Bomb Glove / Devastator records, the first ticker line
    /// ids, the LED advances of ' ' '!' '"').
    pub fn load(ov: &Overlay, item_defs: u32, shop: ShopTable) -> Option<VendorTables> {
        let name = (0..ITEMS as u32).map(|i| ov.i16(item_defs + 0x4c * i).map(i32::from)).collect::<Option<Vec<_>>>()?;
        let icon = (0..ITEMS as u32).map(|i| ov.u16(item_defs + 0x4c * i + 0x38)).collect::<Option<Vec<_>>>()?;
        // Records 10 / 11 of the hologram table: desc 21083 at +0x10, 21084 at +0x24.
        let holo = ov
            .sections()
            .iter()
            .filter(|s| s.kind != 8)
            .flat_map(|s| s.data.windows(0x18).enumerate().filter(|(_, w)| w[..4] == 21083i32.to_le_bytes() && w[0x14..] == 21084i32.to_le_bytes()).map(move |(i, _)| s.dest + i as u32))
            .next()?
            .checked_sub(10 * 0x14 + 0x10)?;
        let desc = (0..ITEMS as u32).map(|i| ov.i32(holo + 0x14 * i + 0x10)).collect::<Option<Vec<_>>>()?;
        let t = find(ov, &i32s(&[20321, 20322, 20322, 20322, 20322, 20323]))?;
        let ticker = (0..24).map(|i| ov.i32(t + 4 * i)).collect::<Option<Vec<_>>>()?;
        let adv = find(ov, &i32s(&[9, 5, 10, 10, 10, 10, 10, 5]))?;
        let led_adv = (0..64).map(|i| ov.i32(adv + 4 * i)).collect::<Option<Vec<_>>>()?;
        let led_cell = (0..64).map(|i| ov.i32(adv - 0x100 + 4 * i)).collect::<Option<Vec<_>>>()?;
        Some(VendorTables { shop, name, icon, desc, ticker, led_cell, led_adv })
    }
}

/// One entry of the list 0x1caa10 (`{item, is_ammo, locked, 0, hologram}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub item: usize,
    pub ammo: bool,
    pub locked: bool,
}

/// `VendorBuildItemList(remote)` 0x2adef0: the stock 0x15edd0 in order (not owned → the weapon, unless remote; owned
/// (bit 0x40) → its ammo when it has an ammo price), then the ammo of every owned item with an ammo price that is
/// not listed yet.
pub fn build_list(gs: &GameState, shop: &ShopTable, remote: bool) -> Vec<Entry> {
    let g = &gs.global;
    let n = g.vendor.iter().position(|&b| b == 0xff).unwrap_or(g.vendor.len());
    let mut out: Vec<Entry> = Vec::new();
    for &b in &g.vendor[..n] {
        let item = (b & 0x3f) as usize;
        if b & 0x40 == 0 {
            if !remote { out.push(Entry { item, ammo: false, locked: false }); }
        } else if shop.ammo_price(item) != 0 {
            out.push(Entry { item, ammo: true, locked: false });
        }
    }
    for item in 0..ITEMS {
        if g.owned[item] != 0 && shop.ammo_price(item) != 0 && !out.iter().any(|e| e.item == item) {
            out.push(Entry { item, ammo: true, locked: false });
        }
    }
    out
}

/// The vendor camera (`OpenVendorMenu`: 0x1ca9a0 = the vendor's rows · (3.8, 0, 1.5) (gp−0x5ba0) + its position,
/// yaw = the vendor's + π, pitch 0): (eye, forward, left, up).
pub fn vendor_camera(pos: [f32; 3], rows: [[f32; 4]; 4], yaw: f32) -> ([f32; 3], [[f32; 3]; 3]) {
    let off = [3.8f32, 0.0, 1.5];
    let eye = std::array::from_fn(|k| pos[k] + off[0] * rows[0][k] + off[1] * rows[1][k] + off[2] * rows[2][k]);
    let cy = crate::moby_update::interact::add_rot(yaw, std::f32::consts::PI);
    let (s, c) = cy.sin_cos();
    (eye, [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]])
}

/// What a frame did besides the state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorOut {
    /// Vendor class sounds, in order.
    pub sounds: Vec<u8>,
    /// Vendor animation requests (seq, blend ticks, speed).
    pub anim: Vec<(u8, i32, f32)>,
    /// `VendorExit` ran: back to mode 0.
    pub exit: bool,
    /// A purchase: (item, ammo entry, bolts spent, units bought).
    pub purchase: Option<(usize, bool, i32, i32)>,
}

/// The vendor's state (0x1ca940.., the popup, the ticker).
#[derive(Clone, Debug)]
pub struct Vendor {
    pub tables: VendorTables,
    /// 0x1ca940 / 0x1ca944.
    pub sub: u8,
    pub t: i32,
    /// 0x1ca980: the PDA's remote vendor (ammo only, PDA prices).
    pub remote: bool,
    /// 0x1caa10.. / 0x1cab50.
    pub items: Vec<Entry>,
    /// 0x1ca998: selected entry.
    pub sel: usize,
    /// 0x1ca98c: carousel offset (≥ 8 entries).
    pub scroll: i32,
    /// 0x1ca99c: buy flow 0..4.
    pub buy: u8,
    /// 0x1622b4: quantity; 0x162290: the popup asks a quantity; 0x16228c: confirmed.
    pub qty: i32,
    pub qty_mode: bool,
    pub confirm: bool,
    /// Popup animation frames left (the class-0x471 open / close, 8 each).
    pub popup_t: i32,
    /// 0x162298 / 0x162294: △ pressed, frames until the leave.
    pub exit_req: bool,
    pub exit_t: i32,
    /// 0x1622a8 / 0x1622ac: Left / Right held frames (auto-repeat).
    pub held_l: i32,
    pub held_r: i32,
    /// gp−0x4950: 0x13cb04 of the last frame.
    pub prev_pressed: u32,
    /// 0x1ca988: the weapon bought last (−1).
    pub bought: i32,
    /// 0x1ca96c: ticker text; 0x1ca984: its scroll (2 px per frame).
    pub ticker: Vec<u8>,
    pub ticker_scroll: i32,
    /// 0x15f3fc: fade-to-black factor.
    pub fade: f32,
    /// Frames of the open's `FadeToBlack(4)` still to show (the camera cuts after them).
    pub pre_fade: i32,
}

impl Vendor {
    /// `OpenVendorMenu(vendor)` 0x2ae1a0 without the moby / camera / HUD parts (the engine's): the list, the
    /// selection in the middle, the carousel offset, sound 3.
    pub fn open(tables: VendorTables, gs: &GameState, remote: bool, out: &mut VendorOut) -> Vendor {
        let items = build_list(gs, &tables.shop, remote);
        let n = items.len();
        let sel = n / 2;
        let base = if n < 3 { 1 } else { sel as i32 };
        out.sounds.push(sound::OPEN);
        Vendor {
            tables,
            sub: 0,
            t: 0,
            remote,
            items,
            sel,
            scroll: (base - sel as i32) * 0x28,
            buy: 0,
            qty: 0,
            qty_mode: false,
            confirm: false,
            popup_t: 0,
            exit_req: false,
            exit_t: 0,
            held_l: 0,
            held_r: 0,
            prev_pressed: 0,
            bought: -1,
            ticker: Vec::new(),
            ticker_scroll: 0,
            fade: 1.0,
            pre_fade: if remote { 0 } else { 4 },
        }
    }

    /// The selected entry.
    pub fn current(&self) -> Option<Entry> { self.items.get(self.sel).copied() }

    fn ammo_unit(&self, item: usize) -> i32 {
        if self.remote { self.tables.shop.pda_ammo_price(item) as i32 } else { self.tables.shop.ammo_price(item) as i32 }
    }

    /// The weapon price (the discounted one while the flag 0x13d4e3 is set).
    fn price(&self, gs: &GameState, item: usize) -> i32 {
        if discount(gs) { self.tables.shop.discounted(item) } else { self.tables.shop.price(item) }
    }

    fn full(&self, gs: &GameState, item: usize) -> bool { self.tables.shop.max_ammo(item) as i32 <= gs.global.ammo[item] }

    /// `set_scrolling_status_message(msg)` 0x2aede0: 18 spaces and the text, scroll 0.
    pub fn set_ticker(&mut self, text: &[u8]) {
        self.ticker = vec![b' '; TICKER_PAD];
        self.ticker.extend_from_slice(text);
        self.ticker_scroll = 0;
    }

    /// The HUD's ammo slot after a move: the selected ammo entry (`queue_animation_update(0x30, 60000 + item, …)`)
    /// or none.
    pub fn hud_ammo(&self, gs: &GameState) -> Option<(u16, i32, i32)> {
        let e = self.current().filter(|e| e.ammo)?;
        Some((e.item as u16, gs.global.ammo[e.item], self.tables.shop.max_ammo(e.item) as i32))
    }

    /// One frame of `VendorModeUpdate` (0x2b03b8). `rng` is the game's stream (the ticker picks its idle line with
    /// `randi(24)` in its draw).
    pub fn frame(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, rng: &mut Rng) -> VendorOut {
        let mut out = VendorOut::default();
        if self.pre_fade > 0 {
            self.pre_fade -= 1;
            return out;
        }
        match self.sub {
            0 => {
                self.fade = (self.fade - 0.34).max(0.0);
                self.t += 1;
                if self.t >= 40 || self.remote {
                    self.fade = 0.0;
                    self.sub = 1;
                    self.t = 0;
                    out.anim.push((3, 8, 1.0));
                    out.sounds.push(sound::SCREENS_ON);
                }
            }
            1 => self.menu(inp, gs, items, session, assets, &mut out),
            2 => {
                self.t += 1;
                if self.t >= 40 || self.remote {
                    out.anim.push((1, 8, 0.0));
                    // A bought weapon with a demo scene (0x1ca4a0 ≥ 0) would run `VendorStartWeaponDemo`: not ported.
                    out.sounds.push(sound::CLOSED);
                    out.exit = true;
                }
            }
            _ => out.exit = true,
        }
        // The ticker scrolls in its draw (every frame the screens are drawn: substates ≥ 1 after 36 frames).
        if self.sub >= 1 { self.ticker_step(rng, assets); }
        out
    }

    /// Substate 1 (and the buy flow).
    fn menu(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, out: &mut VendorOut) {
        let prev = self.prev_pressed;
        let edge = |b: u32| inp.pressed_u & b != 0 && prev & b == 0;
        let n = self.items.len();
        if self.buy == 0 && !self.exit_req && n > 0 {
            let mut moved = false;
            if n < 8 {
                if edge(button::RIGHT) && self.sel + 1 < n {
                    self.sel += 1;
                    moved = true;
                }
                if edge(button::LEFT) && self.sel > 0 {
                    self.sel -= 1;
                    moved = true;
                }
            } else {
                if edge(button::RIGHT) && self.scroll < 0x39 {
                    self.scroll += 0x38;
                    self.sel = if self.sel + 1 >= n { 0 } else { self.sel + 1 };
                    moved = true;
                }
                if edge(button::LEFT) && self.scroll > -0x39 {
                    self.scroll -= 0x38;
                    self.sel = if self.sel == 0 { n - 1 } else { self.sel - 1 };
                    moved = true;
                }
            }
            if moved {
                out.sounds.push(sound::CURSOR);
                let d = self.current().map_or(0, |e| self.tables.desc.get(e.item).copied().unwrap_or(0));
                let t = assets.msg(d).to_vec();
                self.set_ticker(&t);
            }
            if edge(button::CROSS) && self.current().is_some_and(|e| !e.locked) {
                self.buy = 1;
                out.sounds.push(sound::SELECT);
            }
            if inp.pressed_u & button::TRIANGLE != 0 {
                self.exit_req = true;
                self.exit_t = 8;
                out.sounds.push(sound::EXIT);
            }
        }
        self.buy_flow(inp, gs, items, session, assets, out);
        self.prev_pressed = inp.pressed_u;
        if self.exit_req {
            self.exit_t -= 1;
            if self.exit_t == 0 {
                out.anim.push((4, 8, -0.5));
                self.sub = 2;
                self.t = 0;
            }
        }
    }

    /// The buy flow 0x2af7e8.
    fn buy_flow(&mut self, inp: &MenuInput, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, out: &mut VendorOut) {
        let Some(e) = self.current() else {
            self.buy = 0;
            return;
        };
        let bolts = gs.global.bolts;
        let prev = self.prev_pressed;
        let edge = |b: u32| inp.pressed_u & b != 0 && prev & b == 0;
        match self.buy {
            1 => {
                self.qty = 1;
                self.qty_mode = false;
                self.popup_t = 8;
                let full = e.ammo && self.full(gs, e.item);
                let ok = if e.ammo { !full && self.ammo_unit(e.item) <= bolts } else { self.price(gs, e.item) <= bolts };
                if e.ammo && ok { self.qty_mode = true; }
                if !ok { out.sounds.push(sound::DENIED); }
                self.buy = 2;
            }
            2 => {
                if self.popup_t > 0 { self.popup_t -= 1; }
                if self.qty_mode {
                    self.held_l = if inp.held_u & button::LEFT != 0 { self.held_l + 1 } else { 0 };
                    self.held_r = if inp.held_u & button::RIGHT != 0 { self.held_r + 1 } else { 0 };
                    let rep = |h: i32| (h > 15 && h & 7 == 0) || h > 47;
                    if (inp.pressed_u & button::LEFT != 0 || rep(self.held_l)) && self.qty > 1 {
                        self.qty -= 1;
                        if self.held_l & 3 == 0 || self.held_l < 16 { out.sounds.push(sound::CURSOR); }
                    }
                    if inp.pressed_u & button::RIGHT != 0 || rep(self.held_r) {
                        let unit = self.ammo_unit(e.item);
                        if unit != 0 {
                            let room = self.tables.shop.max_ammo(e.item) as i32 - gs.global.ammo[e.item];
                            let max = room.min(bolts / unit);
                            if self.qty < max {
                                self.qty += 1;
                                if self.held_r & 3 == 0 || self.held_r < 16 { out.sounds.push(sound::CURSOR); }
                            }
                        }
                    }
                }
                if edge(button::TRIANGLE) {
                    self.buy = 3;
                    self.confirm = false;
                    self.popup_t = 1;
                } else if edge(button::CROSS) {
                    self.buy = 3;
                    self.confirm = true;
                    self.popup_t = 8;
                }
            }
            3 => {
                self.popup_t -= 1;
                if self.popup_t <= 0 {
                    self.buy = 4;
                    self.popup_t = 8;
                }
            }
            4 => {
                self.popup_t -= 1;
                if self.popup_t > 0 { return; }
                let s = if self.confirm { self.purchase(e, gs, items, session, assets, out) } else { sound::SELECT };
                out.sounds.push(s);
                self.buy = 0;
            }
            _ => {}
        }
    }

    /// State 4 of the buy flow with ✕: the purchase; returns the class sound.
    fn purchase(&mut self, e: Entry, gs: &mut GameState, items: &ItemTables, session: &mut SessionState, assets: &MenuAssets, out: &mut VendorOut) -> u8 {
        let bolts = gs.global.bolts;
        let max = self.tables.shop.max_ammo(e.item) as i32;
        let cost = if e.ammo { self.ammo_unit(e.item) * self.qty } else { self.price(gs, e.item) };
        if bolts < cost && max <= gs.global.ammo[e.item] { return sound::SELECT; }
        if e.ammo {
            if gs.global.ammo[e.item] >= max { return sound::SELECT; }
            let unit = self.ammo_unit(e.item);
            if bolts < unit * self.qty {
                self.set_ticker(assets.msg(msg::CANT_AFFORD_TICKER));
                return sound::SELECT;
            }
            let a = &mut gs.global.ammo[e.item];
            if *a > max { *a = max; }
            // AddAmmo 0x2494d8: the overflow above the max comes back.
            *a += self.qty;
            let over = if max != 0 && *a > max { let o = *a - max; *a = max; o } else { 0 };
            let bought = self.qty - over;
            gs.global.ammo_bought[e.item] += bought;
            gs.global.bolts -= unit * bought;
            out.purchase = Some((e.item, true, unit * bought, bought));
        } else {
            let price = self.price(gs, e.item);
            if bolts < price {
                self.set_ticker(assets.msg(msg::CANT_AFFORD_TICKER));
                return sound::SELECT;
            }
            gs.give_item(e.item, true, items, session);
            self.bought = e.item as i32;
            gs.global.bolts -= price;
            out.purchase = Some((e.item, false, price, 1));
        }
        self.items = build_list(gs, &self.tables.shop, self.remote);
        if self.sel + 1 > self.items.len() { self.sel = self.items.len().saturating_sub(1); }
        self.set_ticker(assets.msg(msg::THANK_YOU));
        sound::PURCHASE
    }

    /// The ticker's scroll (0x2b1a48): +2 per frame; when the text has scrolled out (`len − scroll/20 < 1`) a random
    /// idle line (`randi(24)` of 0x1ca538).
    fn ticker_step(&mut self, rng: &mut Rng, assets: &MenuAssets) {
        let len = self.ticker.len() as i32;
        if len - self.ticker_scroll / 0x14 < 1 {
            let k = rng.randi(24) as usize;
            let id = self.tables.ticker.get(k).copied().unwrap_or(0);
            let t = assets.msg(id).to_vec();
            self.set_ticker(&t);
        } else {
            self.ticker_scroll += 2;
        }
    }
}

/// Flag 0x13d4e3 (the discount; identity [L]): `GameState::global.owned[35]`'s neighbour byte in the owned table
/// 0x13d4c0 + 0x23.
fn discount(gs: &GameState) -> bool { gs.global.owned.get(0x23).is_some_and(|&b| b != 0) }

/// The screen panels' places on the 512×416 screen (port layout: the game maps its 512×128 render targets onto the
/// vendor's monitor bones).
pub mod layout {
    /// Ticker: target region x 4..226, y 0..40.
    pub const TICKER: (i32, i32, i32, i32) = (145, 36, 222, 40);
    /// Item panel (128×128).
    pub const ITEM: (i32, i32, i32, i32) = (40, 112, 128, 128);
    /// Icon strip (height 64; width by entry count).
    pub const STRIP_Y: i32 = 316;
    /// Prompt (80×40, text centred at (40, 20)).
    pub const PROMPT: (i32, i32, i32, i32) = (216, 262, 80, 40);
    /// Button window.
    pub const BUTTONS: (i32, i32, i32, i32) = (352, 262, 120, 40);
    /// Popup (the class-0x471 moby's text panel).
    pub const POPUP: (i32, i32, i32, i32) = (320, 112, 160, 96);
}

/// Panel clear colour (the render targets are cleared to black; on the monitors they read as dark glass: the port
/// draws them black at the help box's frame alpha 0x60).
const PANEL_CLEAR: u32 = 0x6000_0000;
const WHITE: u32 = 0x80f0_f0f0;

fn small_width(a: &MenuAssets, t: &[u8]) -> i32 { a.width(Font::Small, t) }

/// `format_scaled_display_value` 0x2b1bb0: "%d" below 1000, else "%d,%03d".
pub fn price_text(v: i32) -> Vec<u8> {
    if v < 1000 { format!("{v}").into_bytes() } else { format!("{},{:03}", v / 1000, v % 1000).into_bytes() }
}

impl Vendor {
    /// The frame's draws: the fade and, from substate 1 on (the screens), the panels.
    pub fn draw(&self, a: &MenuAssets, gs: &GameState, vsync: u32, out: &mut Vec<MenuDraw>) {
        if self.pre_fade > 0 {
            // FadeToBlack(4): black 0x80 − i·0x80/4 over the still frame.
            let k = 4 - self.pre_fade;
            out.push(MenuDraw::Darken { alpha: 0x80 - (3 - k) * 0x80 / 4 });
            return;
        }
        if self.fade > 0.0 { out.push(MenuDraw::Darken { alpha: (self.fade * 128.0) as i32 }); }
        if self.sub == 0 && self.t < 36 { return; }
        if self.sub >= 2 { return; }
        self.draw_ticker(a, vsync, out);
        self.draw_item(a, gs, out);
        self.draw_strip(a, out);
        self.draw_prompt(a, gs, out);
        self.draw_buttons(a, gs, out);
        if self.buy != 0 { self.draw_popup(a, gs, out); }
    }

    fn panel(out: &mut Vec<MenuDraw>, r: (i32, i32, i32, i32)) { out.push(MenuDraw::PanelBegin { x: r.0, y: r.1, w: r.2, h: r.3, clear: PANEL_CLEAR }); }

    /// `VendorDrawTicker` 0x2b1a48 with the LED text `fun_00238310(2.0, text, −scroll, 8)`: characters 0x20..0x5a,
    /// cells 9×9 drawn 18×18, 'b' makes the next glyph blink (hidden 10 of every 40 frames), glyphs drawn while
    /// −9 < x < 256; black bars at x 0..4 and 226..230.
    fn draw_ticker(&self, a: &MenuAssets, vsync: u32, out: &mut Vec<MenuDraw>) {
        Self::panel(out, layout::TICKER);
        let frame = a.frame(LED_ICON, 0);
        let mut x = -self.ticker_scroll - 4;
        let mut blink = false;
        for &c in &self.ticker {
            let k = c as i32 - 0x20;
            if k == 0x42 { blink = true; }
            if !(0..0x3b).contains(&k) { continue; }
            let cell = self.tables.led_cell.get(k as usize).copied().unwrap_or(-1);
            if cell == -1 && k != 0 { continue; }
            let hidden = std::mem::take(&mut blink) && vsync % 40 < 10;
            if k != 0 && !hidden && x > -9 && x < 0x100 {
                let (u, v) = (((cell & 0xffff) >> 4), ((cell as u32) >> 20) as i32);
                out.push(MenuDraw::SpriteUv { frame, x0: x * 16, y0: 8 * 16, x1: (x + 18) * 16, y1: (8 + 18) * 16, u0: u * 16, v0: v * 16, u1: (u + 9) * 16, v1: (v + 9) * 16, alpha: 0x80, repeat_u: false });
            }
            x += self.tables.led_adv.get(k as usize).copied().unwrap_or(10) * 2;
        }
        out.push(MenuDraw::PanelEnd);
    }

    /// `VendorDrawItemPanel` 0x2b1f08 (the item model replaced by its icon).
    fn draw_item(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        Self::panel(out, layout::ITEM);
        let icon = self.tables.icon.get(e.item).copied().unwrap_or(0);
        sprite(out, a.frame(icon, if e.ammo { 2 } else { 0 }), 40, 40, 48, 48, 0x80);
        let name = a.msg(self.tables.name.get(e.item).copied().unwrap_or(0)).to_vec();
        text(out, Font::Small, 6, 8, WHITE, &name);
        let (price, rgba) = if e.ammo {
            text(out, Font::Small, 0x18, 0x18, WHITE, a.msg(msg::AMMO));
            (self.ammo_unit(e.item), WHITE)
        } else {
            (self.tables.shop.price(e.item), if discount(gs) { 0x8080_8080 } else { WHITE })
        };
        let t = price_text(price);
        text(out, Font::Small, 118 - small_width(a, &t), 101, rgba, &t);
        if !e.ammo && discount(gs) {
            let t = price_text(self.tables.shop.discounted(e.item));
            text(out, Font::Small, 118 - small_width(a, &t), 85, WHITE, &t);
        }
        out.push(MenuDraw::PanelEnd);
    }

    /// `VendorDrawIconStrip` 0x2b1c10.
    fn draw_strip(&self, a: &MenuAssets, out: &mut Vec<MenuDraw>) {
        let n = self.items.len() as i32;
        let w = if n < 8 { 16 + 56 * n.max(1) } else { 400 };
        Self::panel(out, (256 - w / 2, layout::STRIP_Y, w, 64));
        let pulse = ((self.t & 15) * 4 - 32).abs() + 0x40;
        let sel_rgba = (pulse as u32).wrapping_mul(0x0001_0202) | 0x8000_0000;
        let icon = |e: &Entry| a.frame(self.tables.icon.get(e.item).copied().unwrap_or(0), if e.ammo { 2 } else { 0 });
        if n < 8 {
            let x = self.sel as i32 * 0x38;
            out.push(MenuDraw::Rect { x0: x + 9, y0: 3, x1: x + 0x41, y1: 0x3b, rgba: sel_rgba });
            out.push(MenuDraw::Rect { x0: x + 11, y0: 5, x1: x + 0x3f, y1: 0x39, rgba: 0x8000_0000 });
            for (i, e) in self.items.iter().enumerate() { sprite(out, icon(e), 12 + 0x38 * i as i32, 6, 0x30, 0x30, 0x80); }
        } else {
            if self.scroll == 0 {
                out.push(MenuDraw::Rect { x0: 0xb1, y0: 3, x1: 0xe9, y1: 0x3b, rgba: sel_rgba });
                out.push(MenuDraw::Rect { x0: 0xb3, y0: 5, x1: 0xe7, y1: 0x39, rgba: 0x8000_0000 });
            }
            let mut x = self.scroll - 100;
            for k in -2..9i32 {
                let i = ((2 * n + k + self.sel as i32 - 3) % n) as usize;
                sprite(out, icon(&self.items[i]), x, 6, 0x30, 0x30, 0x80);
                x += 0x38;
            }
        }
        out.push(MenuDraw::PanelEnd);
    }

    /// `VendorDrawPrompt` 0x2b2688: small, centred at (40, 20).
    fn draw_prompt(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let id = if self.buy == 0 {
            Some(msg::EXIT)
        } else if e.ammo {
            (!self.full(gs, e.item) && self.ammo_unit(e.item) <= gs.global.bolts).then_some(msg::EXIT)
        } else {
            (self.price(gs, e.item) <= gs.global.bolts).then_some(msg::NO)
        };
        Self::panel(out, layout::PROMPT);
        if let Some(id) = id {
            let t = a.msg(id);
            text(out, Font::Small, 0x28 - small_width(a, t) / 2, 0x14, WHITE, t);
        }
        out.push(MenuDraw::PanelEnd);
    }

    /// `VendorDrawButtonWindow` 0x2b2430 (the text's place is lost in the decompile [M]: centred here).
    fn draw_buttons(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let bolts = gs.global.bolts;
        let id = if self.buy == 0 {
            msg::BUY
        } else if e.ammo && self.full(gs, e.item) {
            msg::BACK
        } else if e.ammo {
            if self.ammo_unit(e.item) <= bolts { msg::BUY } else { msg::BACK }
        } else if self.price(gs, e.item) <= bolts {
            msg::YES
        } else {
            msg::BACK
        };
        let r = layout::BUTTONS;
        Self::panel(out, r);
        let t = a.msg(id);
        text(out, Font::Small, (r.2 - small_width(a, t)) / 2, r.3 / 2 - 7, WHITE, t);
        out.push(MenuDraw::PanelEnd);
    }

    /// The popup's text `FUN_002b2848`.
    fn draw_popup(&self, a: &MenuAssets, gs: &GameState, out: &mut Vec<MenuDraw>) {
        let Some(e) = self.current() else { return };
        let r = layout::POPUP;
        Self::panel(out, r);
        let centre = |out: &mut Vec<MenuDraw>, id: i32, y: i32| {
            let t = a.msg(id);
            text(out, Font::Small, (r.2 - small_width(a, t)) / 2, y, WHITE, t);
        };
        let bolts = gs.global.bolts;
        if e.ammo && self.full(gs, e.item) {
            centre(out, msg::MAXED, 24);
        } else if !e.ammo {
            if self.price(gs, e.item) <= bolts {
                text(out, Font::Small, 6, 6, WHITE, a.msg(msg::PURCHASE));
                text(out, Font::Small, 2, 0x20, WHITE, a.msg(msg::YES));
                text(out, Font::Small, 2, 0x3e, WHITE, a.msg(msg::NO));
            } else {
                centre(out, msg::CANT_AFFORD, 24);
            }
        } else if bolts < self.ammo_unit(e.item) {
            centre(out, msg::CANT_AFFORD, 24);
        } else {
            centre(out, msg::HOW_MANY, 6);
            let q = format!("{}", self.qty).into_bytes();
            let qx = if self.qty >= 100 { 23 } else if self.qty >= 10 { 30 } else { 37 };
            text(out, Font::Small, qx, 50, WHITE, &q);
            text(out, Font::Small, 12, 50, WHITE, b"<");
            text(out, Font::Small, 64, 50, WHITE, b">");
            let total = format!("{}", self.qty * self.ammo_unit(e.item)).into_bytes();
            text(out, Font::Small, 64 - small_width(a, &total), 76, WHITE, &total);
        }
        out.push(MenuDraw::PanelEnd);
    }
}

/// The HUD 2D call of a `text` draw, for tests.
#[allow(dead_code)]
fn is_text(d: &MenuDraw, s: &[u8]) -> bool { matches!(d, MenuDraw::Hud(Draw::Text { text, .. }) if text == s) }

#[cfg(test)]
mod tests {
    use super::*;

    fn shop() -> ShopTable {
        let mut records = vec![[0u8; 0x18]; crate::moby_update::interact::SHOP_RECORDS];
        let mut set = |i: usize, price: i32, unit: u16, max: u16| {
            records[i][0..4].copy_from_slice(&price.to_le_bytes());
            records[i][4..8].copy_from_slice(&(price / 2).to_le_bytes());
            records[i][8..10].copy_from_slice(&unit.to_le_bytes());
            records[i][0xa..0xc].copy_from_slice(&(unit * 5).to_le_bytes());
            records[i][0xe..0x10].copy_from_slice(&max.to_le_bytes());
        };
        set(10, 0, 5, 40);
        set(15, 2500, 1, 200);
        set(16, 2500, 1, 240);
        set(12, 1000, 0, 0);
        ShopTable { records }
    }

    #[test]
    fn list_is_stock_then_owned_ammo() {
        let mut gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
        gs.global.vendor = [0x4a, 16, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        gs.global.owned[10] = 1;
        gs.global.owned[15] = 1;
        gs.global.owned[12] = 1;
        let l = build_list(&gs, &shop(), false);
        assert_eq!(l, vec![
            Entry { item: 10, ammo: true, locked: false },
            Entry { item: 16, ammo: false, locked: false },
            Entry { item: 15, ammo: true, locked: false },
        ]);
        let r = build_list(&gs, &shop(), true);
        assert_eq!(r.iter().map(|e| (e.item, e.ammo)).collect::<Vec<_>>(), vec![(10, true), (15, true)], "remote: ammo only");
    }

    #[test]
    fn price_text_formats() {
        assert_eq!(price_text(500), b"500");
        assert_eq!(price_text(2500), b"2,500");
        assert_eq!(price_text(150000), b"150,000");
        assert_eq!(price_text(60050), b"60,050");
    }

    #[test]
    fn vendor_camera_faces_the_vendor() {
        let yaw = 0.3f32;
        let (s, c) = yaw.sin_cos();
        let rows = [[c, s, 0.0, 0.0], [-s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]];
        let (eye, r) = vendor_camera([10.0, 20.0, 5.0], rows, yaw);
        assert!((eye[0] - (10.0 + 3.8 * c)).abs() < 1e-5 && (eye[1] - (20.0 + 3.8 * s)).abs() < 1e-5 && (eye[2] - 6.5).abs() < 1e-5);
        // Forward points from the eye back to the vendor.
        let to = [10.0 - eye[0], 20.0 - eye[1]];
        let l = (to[0] * to[0] + to[1] * to[1]).sqrt();
        assert!((r[0][0] - to[0] / l).abs() < 1e-5 && (r[0][1] - to[1] / l).abs() < 1e-5);
    }
}
