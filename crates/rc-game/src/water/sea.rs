//! The sea and the other liquid surfaces the level code draws from a draw callback (docs/plan/world_animation.md §3.3):
//! the class updates that own them, ported per level and found by code identity (`LevelPorts`, `ClassUpdate::Sea(i)`,
//! the index into [`PORTS`]), their level data ([`SeaPortData`], read once per level through `Relocation` from each
//! port's reference level) and their state ([`SeaState`], in `Services::water`). rc-engine's `sea_render` draws the
//! registered callbacks. Three mechanisms:
//!
//! | ports | classes (level) | reference update | draws |
//! |---|---|---|---|
//! | 0–5 | 994 (03), 879 (05), 460 (07) and 317 (09, the same code), 1018 (07), 327 (08), 1260 (14) | per level (below) | the liquid grid module: level05 `0x2ce098` (init) … `0x2ce830` (draw), one [`rc_formats::sea::LiquidGrid`] each |
//! | 6 | 1111 (11, 16) | level11 `0x30b358` (the same code on 16) | the camera-following ocean `0x30a908` |
//! | 7 | 1901 (12) | level12 `0x30bff0` | the static liquid strips `0x30be68` |
//!
//! **The liquid grid classes** are each level's own wrapper around the shared module: init once (`0x2ce098(scale, state,
//! FIX)`: `state+0x3f = FIX`, the module's ST table · scale), then register the level's draw callback (which calls the
//! module's `0x2ce830(state)`) every tick. Their extras: 879 (05) writes the grid height every tick, `59.5 + 0.25·sin`
//! (61.5 while Ratchet is on the Hoverboard, state group 0x16), and its callback skips the draw while the camera is in
//! one of the moby's 8 cuboids (pvar words 0..7, −1 = none); 1018 (07) registers only while the camera is outside its
//! cuboid (pvar 0); 327 (08) inits once per level (a `$gp` flag, not the moby state) and registers every tick without
//! setting its update distance (its splash when Ratchet falls in, level 8's `0x2da0f0` second half, is not ported).
//! Which list: `RegisterDrawCallback` (list 1, after the mobys) on 03, 08, 14; the list drained after the ties and before
//! the shrubs (`0x16e100`, count `0x15f42c`) on 05, 07, 09.
//!
//! **The ocean 1111** (pvar: 0..2 colours of the far band and the wall, 3 / 4 the surface colour near / at two tiles,
//! 5 sea z, 6 tile size S, 7 FX base, 8 lowest camera z, 9 the wall's drop, 10 the band's reach, 11 the fade height):
//! state 0 keeps the camera and the sea z; state 1 scrolls each layer's ST by 2·scale/(5·S) of the camera's motion
//! (fraction only) and by its speed · dt, wraps each lane into [−1, 1], bobs the sea `z + 0.25·sin` and registers the
//! draw (list 1) while the camera is above pvar 8.
//!
//! **The Hoven liquid 1901**: its first init rescales the colours of both strip groups (`0x30bba0`: RGB = constant,
//! A = A/2; done here at load), state 1 scrolls the two layers and group 1 by their speeds · dt (wrapped) and registers
//! the draw on the after-ties list.
//!
//! Native `f32` throughout (the game's literal 3.14 and 0.017444445 in the bob). dt = 1/60 (`0x15ed7c`, NTSC).

use super::world::WaterWorld;
use crate::moby_runtime::MobyId;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::services::{fv, pvar, World};
use rc_formats::level_overlay::{LevelOverlay, Relocation};
use rc_formats::sea::{self as fs, LiquidGrid, LiquidGridModule, OceanTables, StripMesh};
use rc_formats::water::Overlay;

/// Which draw list a port's callback goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawList {
    /// `RegisterDrawCallback` 0x21afe0: after the mobys, before the particles.
    AfterMobys,
    /// The list `0x16e100` (count `0x15f42c`), drained by `DrawWorld` after the ties and before the shrubs.
    AfterTies,
}

/// A liquid grid class's own behaviour around the module (module doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridVariant {
    Plain,
    /// 879 (level05 `0x316110`, callback `0x316070`).
    Rilgar,
    /// 1018 (level07 `0x30d440`).
    Umbris,
    /// 327 (level08 `0x2da0f0`).
    Batalia,
}

/// A liquid grid port: its state record (a label of the reference level), the init's scale and FIX, its list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridPort {
    pub state: u32,
    pub scale: f32,
    pub fix: u8,
    pub list: DrawList,
    pub variant: GridVariant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeaKind {
    Grid(GridPort),
    Ocean,
    Hoven,
}

/// One sea class port.
#[derive(Clone, Copy, Debug)]
pub struct SeaPort {
    pub name: &'static str,
    /// The level whose overlay holds [`SeaPort::func`] (and the port's data labels).
    pub level: u32,
    pub func: u32,
    /// The classes the reference level's class table runs it for.
    pub classes: &'static [i16],
    pub kind: SeaKind,
}

/// `0x3f2aaaab`: the init scale 2/3 of 03, 07 (1018), 08, 14.
const TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);

const fn grid(state: u32, scale: f32, fix: u8, list: DrawList, variant: GridVariant) -> SeaKind {
    SeaKind::Grid(GridPort { state, scale, fix, list, variant })
}

pub const OCEAN: usize = 6;
pub const HOVEN: usize = 7;

pub const PORTS: [SeaPort; 8] = [
    // level03 0x2dc9b0: 0x291198(2/3, 0x1dc1a0, 0x20); callback 0x2dc990 via 0x1f2d48 (list 1).
    SeaPort { name: "994 liquid", level: 3, func: 0x2d_c9b0, classes: &[994], kind: grid(0x1d_c1a0, TWO_THIRDS, 0x20, DrawList::AfterMobys, GridVariant::Plain) },
    // level05 0x316110: 0x2ce110(1.0, 0x211b20) (FIX 0x80); callback 0x316070 via 0x228110 (after the ties).
    SeaPort { name: "879 sea", level: 5, func: 0x31_6110, classes: &[879], kind: grid(0x21_1b20, 1.0, 0x80, DrawList::AfterTies, GridVariant::Rilgar) },
    // level07 0x2f8058: 0x2cc608(1.0, 0x1d3400, 0x80); callback 0x2f8038 via 0x221290 (after the ties). The same code
    // (relinked) is level09's 0x2ef7f8 for 317, the lava: 0x2c11a0(1.0, 0x1fc740, 0x80), callback 0x2ef750 via 0x218810.
    SeaPort { name: "460 / 317 liquid", level: 7, func: 0x2f_8058, classes: &[460, 317], kind: grid(0x1d_3400, 1.0, 0x80, DrawList::AfterTies, GridVariant::Plain) },
    // level07 0x30d440: 0x2cc680(2/3, 0x208560) (FIX 0x80); callback 0x30d420 via 0x221290 (after the ties).
    SeaPort { name: "1018 sea", level: 7, func: 0x30_d440, classes: &[1018], kind: grid(0x20_8560, TWO_THIRDS, 0x80, DrawList::AfterTies, GridVariant::Umbris) },
    // level08 0x2da0f0: 0x2acce8(2/3, 0x1d7060) (FIX 0x80); callback 0x2da0d0 via 0x20bdf0 (list 1).
    SeaPort { name: "327 liquid", level: 8, func: 0x2d_a0f0, classes: &[327], kind: grid(0x1d_7060, TWO_THIRDS, 0x80, DrawList::AfterMobys, GridVariant::Batalia) },
    // level14 0x303370: 0x2aabf8(2/3, 0x1e9c60, 0x40); callback 0x303350 via 0x20c5d0 (list 1).
    SeaPort { name: "1260 liquid", level: 14, func: 0x30_3370, classes: &[1260], kind: grid(0x1e_9c60, TWO_THIRDS, 0x40, DrawList::AfterMobys, GridVariant::Plain) },
    SeaPort { name: "1111 ocean", level: fs::ocean_ref::LEVEL, func: fs::ocean_ref::UPDATE_FN, classes: &[1111], kind: SeaKind::Ocean },
    SeaPort { name: "1901 Hoven liquid", level: fs::hoven_ref::LEVEL, func: fs::hoven_ref::UPDATE_FN, classes: &[1901], kind: SeaKind::Hoven },
];

/// Port indices as the `ClassUpdate::Sea` payload.
pub fn ids() -> impl Iterator<Item = u8> { 0..PORTS.len() as u8 }

/// A liquid grid on the loaded level: the state record and the module's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct GridData {
    pub grid: LiquidGrid,
    pub module: LiquidGridModule,
}

/// Class 1901's strips and globals on the loaded level.
#[derive(Clone, Debug, PartialEq)]
pub struct HovenData {
    /// Group 1 (the camera-cuboid group) and group 2 (the two-layer group), colours as the init leaves them.
    pub groups: [Vec<StripMesh>; 2],
    /// The second layer's ALPHA FIX (`0x1620d4`).
    pub fix2: u8,
    /// Group 1's scroll speed (s, t) and the layers' (s, t) per second.
    pub g1_speed: [f32; 2],
    pub layer_speed: [[f32; 2]; 2],
}

/// One sea port's level data.
#[derive(Clone, Debug, PartialEq)]
pub enum SeaData {
    Grid(Box<GridData>),
    Ocean(OceanTables),
    Hoven(Box<HovenData>),
}

/// A port present on the loaded level (its update has a copy in the level's overlay) and its data.
#[derive(Clone, Debug, PartialEq)]
pub struct SeaPortData {
    pub port: usize,
    pub data: SeaData,
}

/// `0x30bba0(r, g, b, a, table, n, counts, 1)`: every vertex colour of the group becomes `(trunc(r·255), trunc(g·255),
/// trunc(b·255), trunc(a·A))`.
pub fn hoven_rescale(strips: &mut [StripMesh], k: [f32; 4]) {
    let c = [k[0] * 255.0, k[1] * 255.0, k[2] * 255.0].map(|x| x as u32 & 0xff);
    for s in strips {
        for v in &mut s.rgba { *v = (((k[3] * (*v >> 24) as f32) as u32 & 0xff) << 24) | c[2] << 16 | c[1] << 8 | c[0]; }
    }
}

/// The sea ports' data on the level `target` (`ov` its raw overlay), each read through the relocation from its reference
/// level (`rel_of(level)`). A port whose update has no copy on the level is absent; one whose tables do not parse is
/// reported and skipped.
pub fn load<'t>(ov: &Overlay, target: &'t LevelOverlay, rel_of: &dyn Fn(u32) -> Option<Relocation<'t>>) -> Vec<SeaPortData> {
    let _ = target;
    let mut out = Vec::new();
    let module = rel_of(fs::grid_ref::LEVEL).and_then(|r| r.data(fs::grid_ref::ST_BLOCK)).map(|a| LiquidGridModule::parse(ov, a).map_err(|e| e.to_string()));
    for (i, p) in PORTS.iter().enumerate() {
        let Some(rel) = rel_of(p.level) else { continue };
        if rel.func(p.func).is_none() { continue; }
        let data = match p.kind {
            SeaKind::Grid(g) => {
                let Some(Ok(m)) = module.clone() else {
                    eprintln!("sea: {}: the liquid grid module's tables are not on this level", p.name);
                    continue;
                };
                let Some(a) = rel.data(g.state) else {
                    eprintln!("sea: {}: state record {:#x} not found", p.name, g.state);
                    continue;
                };
                match LiquidGrid::parse(ov, a) {
                    Ok(grid) => SeaData::Grid(Box::new(GridData { grid, module: m })),
                    Err(e) => {
                        eprintln!("sea: {}: {e}", p.name);
                        continue;
                    }
                }
            }
            SeaKind::Ocean => {
                let (Some(s), Some(v)) = (rel.data(fs::ocean_ref::SCALE), rel.data(fs::ocean_ref::SPEED)) else { continue };
                match OceanTables::parse(ov, s, v) {
                    Ok(t) => SeaData::Ocean(t),
                    Err(e) => {
                        eprintln!("sea: {}: {e}", p.name);
                        continue;
                    }
                }
            }
            SeaKind::Hoven => match load_hoven(ov, &rel) {
                Ok(h) => SeaData::Hoven(Box::new(h)),
                Err(e) => {
                    eprintln!("sea: {}: {e}", p.name);
                    continue;
                }
            },
        };
        out.push(SeaPortData { port: i, data });
    }
    out
}

fn load_hoven(ov: &Overlay, rel: &Relocation) -> Result<HovenData, rc_formats::FormatError> {
    use fs::hoven_ref as h;
    let at = |l: u32| rel.data(l).ok_or_else(|| rc_formats::FormatError::Invalid(format!("label {l:#x} not found")));
    let group = |g: (u32, u32, u32, u32, usize)| -> Result<Vec<StripMesh>, rc_formats::FormatError> {
        fs::parse_strip_meshes(ov, at(g.0)?, at(g.1)?, at(g.2)?, at(g.3)?, g.4)
    };
    let mut groups = [group(h::G1)?, group(h::G2)?];
    // The first init's 0x30bba0 calls (level12 0x30c030 / 0x30c060).
    hoven_rescale(&mut groups[1], [f32::from_bits(0x3dcc_cccd), 0.5, f32::from_bits(0x3ecc_cccd), 0.5]);
    hoven_rescale(&mut groups[0], [f32::from_bits(0x3dcc_cccd), f32::from_bits(0x3f19_999a), f32::from_bits(0x3ecc_cccd), 0.5]);
    let f = |l: u32| -> Result<f32, rc_formats::FormatError> { ov.f32(at(l)?) };
    Ok(HovenData {
        groups,
        fix2: ov.u32(at(h::FIX2)?)? as u8,
        g1_speed: [f(h::G1_SPEED)?, f(h::G1_SPEED + 4)?],
        layer_speed: [[f(h::LAYER_SPEED)?, f(h::LAYER_SPEED + 4)?], [f(h::LAYER_SPEED + 8)?, f(h::LAYER_SPEED + 12)?]],
    })
}

/// One port's run-time state (the globals and record fields its update and callback share).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SeaRun {
    /// The module init has run (grid; the level-8 `$gp` flag; 1901's colour rescale flag).
    pub inited: bool,
    /// Grid: the state record's z (`+0x08`) as the class leaves it, and its FIX (`+0x3f`).
    pub z: f32,
    pub fix: u8,
    /// Ocean / Hoven: the layers' ST scroll (s, t) (1111: `0x161e08 + 8·layer`; 1901: `0x1620f8 + 8·layer`).
    pub scroll: [[f32; 2]; 2],
    /// 1901 group 1's scroll (`0x162108`, `0x16210c`).
    pub g1_scroll: [f32; 2],
    /// 1111: the camera at the last update (`$gp − 0x43c0`), the sea z it draws (`0x161e18`) and its base (`0x161e1c`).
    pub prev_cam: [f32; 3],
    pub sea_z: f32,
    pub base_z: f32,
}

/// The sea ports' state (in `WaterWorld`), by port index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeaState {
    pub run: [SeaRun; PORTS.len()],
}

/// The port's level data (None: not on this level, or the water data is not loaded).
pub fn data(water: &WaterWorld, port: usize) -> Option<&SeaData> {
    water.data.as_ref()?.sea.iter().find(|d| d.port == port).map(|d| &d.data)
}

/// `(*moby+0x74)(moby)` of sea port `port`. Without the level's data for it nothing runs.
pub fn update(w: &mut World, id: MobyId, port: u8) {
    let port = port as usize;
    let Some(p) = PORTS.get(port) else { return };
    if data(&w.svc.water, port).is_none() { return; }
    match p.kind {
        SeaKind::Grid(g) => grid_update(w, id, port, g),
        SeaKind::Ocean => ocean_update(w, id, port),
        SeaKind::Hoven => hoven_update(w, id, port),
    }
}

/// dt `0x15ed7c` (NTSC).
const DT: f32 = 1.0 / 60.0;

/// The bob every sea uses: `sin((counter % 360)·0.017444445 − 3.14)` (the game's literals).
#[allow(clippy::approx_constant)]
pub fn bob(counter: u64) -> f32 { (((counter % 360) as f32) * 0.017_444_445 - 3.14).sin() }

/// A scroll lane after `+= d`: above 1 → −1, below −1 → +1 (each test once, in that order).
fn wrap(x: f32) -> f32 {
    let x = if x > 1.0 { x - 1.0 } else { x };
    if x < -1.0 { x + 1.0 } else { x }
}

fn register(w: &mut World, id: MobyId, port: usize, list: DrawList) {
    let cb = Callback::Sea(port as u8);
    match list {
        DrawList::AfterMobys => w.svc.draw_callbacks.register(cb, id),
        DrawList::AfterTies => w.svc.draw_callbacks.register_ties(cb, id),
    }
}

/// The module init `0x2ce098(scale, state, FIX)` as the port keeps it: the FIX and the record's z.
fn grid_init(w: &mut World, port: usize, g: &GridPort) {
    let z = match data(&w.svc.water, port) {
        Some(SeaData::Grid(d)) => d.grid.origin[2],
        _ => return,
    };
    let r = &mut w.svc.water.sea.run[port];
    if r.inited { return; }
    *r = SeaRun { inited: true, z, fix: g.fix, ..*r };
}

fn grid_update(w: &mut World, id: MobyId, port: usize, g: GridPort) {
    if g.variant == GridVariant::Batalia {
        // 0x2da0f0: `if (!gp flag) { flag = 1; init }`, then register every tick.
        grid_init(w, port, &g);
        register(w, id, port, g.list);
        return;
    }
    match w.m(id).state {
        0 => {
            grid_init(w, port, &g);
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
        }
        1 => {
            match g.variant {
                GridVariant::Rilgar => {
                    // 0x316110: 61.5 while Ratchet's state group (0x1413dc) is 0x16 (the Hoverboard), else 59.5.
                    let base = if w.hero.group == 0x16 { 61.5 } else { 59.5 };
                    w.svc.water.sea.run[port].z = bob(w.counter) * 0.25 + base;
                }
                GridVariant::Umbris => {
                    // 0x30d440: no registration while the camera (0x166e40) is in the cuboid of pvar 0.
                    let c = fv(w.camera);
                    let cub = pvar::i32(&w.m(id).pvars, 0);
                    if crate::moby_update::triggers::point_in_cuboid(&w.svc.volumes, [c[0], c[1], c[2]], cub) { return; }
                }
                _ => {}
            }
            register(w, id, port, g.list);
        }
        _ => {}
    }
}

/// 879's callback `0x316070`: no draw while the camera (0x1671c0) is in one of the moby's 8 cuboids (pvar words 0..7,
/// −1 = none). The other grid callbacks always draw.
pub fn grid_draws(port: usize, pvars: &[u8], volumes: &rc_formats::volumes::Volumes, camera: [f32; 3]) -> bool {
    let Some(SeaKind::Grid(g)) = PORTS.get(port).map(|p| p.kind) else { return false };
    if g.variant != GridVariant::Rilgar { return true; }
    !(0..8).any(|k| pvars.len() >= 4 * k + 4 && crate::moby_update::triggers::point_in_cuboid(volumes, camera, pvar::i32(pvars, 4 * k)))
}

/// 1901's callback: group 1 is drawn while the camera (0x167240) is in the moby's cuboid (pvar 0).
pub fn hoven_group1_drawn(pvars: &[u8], volumes: &rc_formats::volumes::Volumes, camera: [f32; 3]) -> bool {
    pvars.len() >= 4 && crate::moby_update::triggers::point_in_cuboid(volumes, camera, pvar::i32(pvars, 0))
}

/// Class 1111's pvar fields (module doc).
pub mod ocean_pvar {
    pub const SEA_Z: usize = 0x14;
    pub const TILE: usize = 0x18;
    pub const FX: usize = 0x1c;
    pub const MIN_CAM_Z: usize = 0x20;
    pub const WALL_DROP: usize = 0x24;
    pub const REACH: usize = 0x28;
    pub const FADE: usize = 0x2c;
}

fn ocean_update(w: &mut World, id: MobyId, port: usize) {
    let Some(SeaData::Ocean(t)) = data(&w.svc.water, port).cloned() else { return };
    let cam = fv(w.camera);
    let cam = [cam[0], cam[1], cam[2]];
    let pf = |w: &World, o: usize| pvar::ff(&w.m(id).pvars, o);
    if w.m(id).pvars.len() < 0x30 { return; }
    if w.m(id).state == 0 {
        let base_z = pf(w, ocean_pvar::SEA_Z);
        let m = w.mm(id);
        m.update_dist = 0xff;
        m.state = 1;
        let r = &mut w.svc.water.sea.run[port];
        r.scroll = [[0.0; 2]; 2];
        r.prev_cam = cam;
        r.base_z = base_z;
        return;
    }
    let (tile, min_z) = (pf(w, ocean_pvar::TILE), pf(w, ocean_pvar::MIN_CAM_Z));
    let counter = w.counter;
    let r = &mut w.svc.water.sea.run[port];
    let d = [cam[0] - r.prev_cam[0], cam[1] - r.prev_cam[1]];
    r.prev_cam = cam;
    // `c.lt.s pvar8, cam z` then `bc1f`: no draw unless strictly above (and not on a NaN).
    if min_z.partial_cmp(&cam[2]) != Some(std::cmp::Ordering::Less) { return; }
    for (layer, s) in r.scroll.iter_mut().enumerate() {
        // `vec_scale((scale + scale) / (S·5), delta)`, then each lane less its integer part unless it is exactly 1.
        let k = (t.scale[layer] + t.scale[layer]) / (tile * 5.0);
        for (lane, dv) in d.iter().enumerate() {
            let mut x = dv * k;
            if x != 1.0 { x -= x.trunc(); }
            s[lane] = wrap(s[lane] + x);
        }
    }
    r.sea_z = r.base_z + bob(counter) * 0.25;
    for (layer, s) in r.scroll.iter_mut().enumerate() {
        for (lane, x) in s.iter_mut().enumerate() { *x = wrap(*x + t.speed[layer][lane] * DT); }
    }
    register(w, id, port, DrawList::AfterMobys);
}

fn hoven_update(w: &mut World, id: MobyId, port: usize) {
    let Some(SeaData::Hoven(h)) = data(&w.svc.water, port) else { return };
    let (g1_speed, layer_speed) = (h.g1_speed, h.layer_speed);
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.draw_dist = 0xff;
            m.update_dist = 0xff;
            let r = &mut w.svc.water.sea.run[port];
            r.inited = true;
            r.scroll = [[0.0; 2]; 2];
        }
        1 => {
            let r = &mut w.svc.water.sea.run[port];
            for (layer, s) in r.scroll.iter_mut().enumerate() {
                for (lane, x) in s.iter_mut().enumerate() { *x = wrap(*x + layer_speed[layer][lane] * DT); }
            }
            for (lane, x) in r.g1_scroll.iter_mut().enumerate() { *x = wrap(*x + g1_speed[lane] * DT); }
            register(w, id, port, DrawList::AfterTies);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_tests_each_bound_once() {
        assert_eq!(wrap(1.5), 0.5);
        assert_eq!(wrap(-1.5), -0.5);
        assert_eq!(wrap(0.25), 0.25);
        assert_eq!(wrap(1.0), 1.0);
    }

    #[test]
    fn bob_is_the_games_sine() {
        // counter 180: 180·0.017444445 − 3.14 = 0.00000... ≈ 0.0000001
        assert!(bob(180).abs() < 1e-3);
        assert!((bob(270) - 1.0).abs() < 1e-3);
        assert_eq!(bob(0), bob(360));
    }

    #[test]
    fn hoven_rescale_sets_rgb_and_halves_alpha() {
        let mut s = vec![StripMesh { pos: vec![[0.0; 3]], rgba: vec![0x7f12_3456], st: vec![[0.0; 2]] }];
        hoven_rescale(&mut s, [f32::from_bits(0x3dcc_cccd), 0.5, f32::from_bits(0x3ecc_cccd), 0.5]);
        assert_eq!(s[0].rgba[0], 0x3f_66_7f_19);
    }

    #[test]
    fn ports_are_unique() {
        for (i, a) in PORTS.iter().enumerate() {
            for b in &PORTS[i + 1..] { assert!(a.level != b.level || a.func != b.func); }
        }
    }
}
