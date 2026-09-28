use super::screens::{Quad, Statics, View};
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

#[test]
fn euler_rows_match_the_moby_matrix() {
    for e in [[0.3f32, -0.7, 1.9], [0.0, 0.0, 2.5], [-1.2, 0.4, 0.0]] {
        let a = euler_rows(e);
        let b = rc_formats::moby_light::rotation_rows(e);
        for i in 0..3 {
            for k in 0..3 { assert!((a[i][k] - f32::from_bits(b[i][k])).abs() < 1e-4, "{e:?} row {i} {k}"); }
        }
    }
}

#[test]
fn screen_quad_insets_and_shrinks_about_its_centre() {
    // A 2×1 monitor in the x/z plane.
    let q = Quad::new([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 1.0]], [0.1, 0.05, 0.1, 0.05]);
    assert!((q.c[0] - 0.1).abs() < 1e-6 && (q.c[2] - 0.05).abs() < 1e-6);
    assert!((q.a[0] - 1.8).abs() < 1e-6 && (q.b[2] - 0.9).abs() < 1e-6);
    let h = q.shrink(0.5);
    let (c0, c1) = (q.far(), h.far());
    let mid = |q: &Quad, f: [f32; 3]| [(q.c[0] + f[0]) / 2.0, (q.c[2] + f[2]) / 2.0];
    assert_eq!(mid(&q, c0), mid(&h, c1), "same centre");
    assert!((h.a[0] - 0.9).abs() < 1e-6 && (h.b[2] - 0.45).abs() < 1e-6);
    // Zero: a point at the centre.
    assert!(q.shrink(0.0).a.iter().all(|&v| v == 0.0));
}

#[test]
fn projection_centres_the_view_axis() {
    let v = View::game([0.0, 0.0, 0.0], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    assert_eq!(v.project([5.0, 0.0, 0.0]), Some([256.0, 208.0]));
    // Left (+y) is to the left on screen, up (+z) is up.
    let p = v.project([5.0, 1.0, 1.0]).unwrap();
    assert!(p[0] < 256.0 && p[1] < 208.0);
    assert!((256.0 - p[0] - 256.0 / (5.0 * 0.63)).abs() < 1e-3);
    assert_eq!(v.project([-1.0, 0.0, 0.0]), None, "behind");
    let t = v.target();
    assert_eq!(t.project([5.0, 0.0, 0.0]), Some([256.0, 64.0]));
}

#[test]
fn ticker_static_always_runs_and_other_screens_burst() {
    let mut s = Statics::default();
    let mut rng = Rng::new();
    let d = s.step(222.0, 40.0, 0, &mut rng);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].fx, screens::NOISE_FX);
    assert_eq!(d[0].rgba >> 24, 0x80, "full strength while the counter is below 0xc0");
    // A burst on screen 3: +2 per frame, fading after 0xc0, over at 0x100.
    s.burst[3] = 2;
    let mut alphas = Vec::new();
    for _ in 0..200 {
        let d = s.step(100.0, 50.0, 3, &mut rng);
        if let Some(n) = d.iter().find(|x| x.fx == screens::NOISE_FX) { alphas.push(n.rgba >> 24); }
        if s.burst[3] == 0 { break; }
    }
    assert_eq!(alphas.len(), 127, "0x04..0xfe step 2, then the reset");
    assert_eq!(alphas[0], 0x80);
    assert!(alphas.last().copied().unwrap() < 0x08);
}

#[test]
fn power_factor_follows_the_counters() {
    let mut out = VendorOut::default();
    let gs = GameState::zeroed(rc_formats::save_game::ChunkTables { global: Vec::new(), level: Vec::new() });
    let mut v = Vendor::open(VendorTables { shop: shop(), ..Default::default() }, &gs, false, &mut out);
    assert_eq!(out.sounds, vec![sound::OPEN]);
    assert_eq!(out.anim, vec![VendorAnim::HardCut { seq: 2, frame: 0 }, VendorAnim::Speed(0.5)]);
    assert!(!v.world_runs(), "FadeToBlack(4) blocks");
    v.pre_fade = 0;
    assert!(v.world_runs());
    v.sub = 1;
    v.power_on = true;
    v.power_t = 8;
    assert_eq!(v.power(), 0.0);
    v.power_t = 2;
    assert_eq!(v.power(), 0.75);
    v.power_on = false;
    assert_eq!(v.power(), 1.0);
    v.exit_req = true;
    v.exit_t = 4;
    assert_eq!(v.power(), 0.5);
    assert!(!v.world_runs() && v.screens_shown());
}
