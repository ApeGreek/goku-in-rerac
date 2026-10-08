//! Local mod: `RC_HERO_GLB=<file under assets/>` draws that glTF in place of Ratchet's moby meshes (which are then
//! hidden, crate::gameplay::ratchet_visibility). The model follows his moby's position and yaw and plays the BT3
//! fighter motion matching his hero state ([`motion`]); a Bomb Glove throw is a charged ki blast, wind-up then
//! release, timed like Ratchet's. Clank and the back pack hang from the model's spine, the hand weapon from its right hand
//! ([`anchors`]); the wrench is not drawn (crate::moby_attach). The look stance (L1) is a Kamehameha: the charge while
//! it is held, the firing when □ throws the wrench, with a ki beam ([`beam`]) from the hand to the flying wrench, which
//! keeps the game's flight and hits.
//! `RC_HERO_SCALE`, `RC_HERO_YAW` (degrees) and `RC_CLANK_POS` (`x,y,z` in the model's bind space) tune the fit;
//! `RC_HERO_FORCE=<motion id>` plays that one motion in a loop (to look at the clips).

use crate::gameplay::Play;
use crate::moby_attach::{MobyAttach, Slot};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use crate::tfrag_render::game_to_bevy;
use bevy::prelude::*;
use std::sync::OnceLock;
use std::time::Duration;

/// The look stance in first person (L1): with the model the camera stays behind it ([`over_shoulder`]) and the model,
/// Clank and the hand weapon stay drawn (the game hides Ratchet and his items there).
pub fn third_person_look(p: &Play) -> bool {
    let h = &p.game.hero;
    path().is_some() && (h.state == 1 || h.state == 0x1e) && h.f13f5 != 0
}

/// The look stance's camera behind the model's right shoulder, blended in and out over ~0.2 s (the game's own camera
/// zooms into Ratchet's head meanwhile): it orbits the model's chest at a fixed distance, looking where the game's
/// camera looks, so the aim (along the game's own camera) is where the view looks.
pub fn over_shoulder(t: Transform, p: &Play) -> Transform {
    static K: std::sync::Mutex<f32> = std::sync::Mutex::new(0.0);
    let look = path().is_some() && matches!(p.game.hero.state, 1 | 0x1e);
    let mut k = K.lock().unwrap();
    let Some((pos, _, _)) = p.hero_pose() else { return t };
    let chest = game_to_bevy([pos[0], pos[1], pos[2] + 1.1]);
    // Out of the stance, held while the game's camera is still coming back out of the head.
    let back_out = t.translation.distance(chest) < 2.8;
    *k = (*k + if look { 0.08 } else if back_out { 0.0 } else { -0.08 }).clamp(0.0, 1.0);
    if *k == 0.0 { return t; }
    let (fwd, up) = (t.forward().as_vec3(), t.up().as_vec3());
    let eye = chest - fwd * 3.0 + up * 0.45 + fwd.cross(up) * 0.75;
    t.with_translation(t.translation.lerp(eye, *k * *k * (3.0 - 2.0 * *k)))
}

pub fn path() -> Option<&'static str> {
    static P: OnceLock<Option<String>> = OnceLock::new();
    P.get_or_init(|| std::env::var("RC_HERO_GLB").ok().filter(|s| !s.trim().is_empty())).as_deref()
}

fn env_f32(name: &str, default: f32) -> f32 { std::env::var(name).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default) }

/// BT3 fighter motion ids (Tenkaichi3Decomp src/battle/btl_act_d.c, btl_act_a.c): 0 ground idle, 3 run loop, 0x20 jump
/// rising, 0x21 falling, 0x37..0x39 the first three rush hits, 0x6D flying kick strike, and the charged ki blast
/// (action 0xAF): 0x7B wind-up with the hand drawn back, 0x7D release; the first Blast 2 beam, the Kamehameha (btl_act_f.c
/// `BtlAct_SuperBeamHandler`, class 2): 0x105 start, 0x106 charge loop, 0x107 fire, 0x108 beam loop. goku.glb names its
/// clips `anim_<id>` and skips a few ids above 152 only, so below that the glTF animation index is the motion id; 0x105..0x108
/// are its clips 258..261. The last slot is `RC_HERO_FORCE`'s.
const MOTIONS: [usize; 15] = [0, 3, 0x20, 0x21, 0x37, 0x38, 0x39, 0x6D, 0x7B, 0x7D, 258, 259, 260, 261, 0];
const WIND_UP: usize = 8;
const RELEASE: usize = 9;
const KAME_START: usize = 10;
const KAME_CHARGE: usize = 11;
const KAME_FIRE: usize = 12;
const KAME_BEAM: usize = 13;
const FORCED: usize = 14;
/// The lengths of the clips 0x105 and 0x107 (seconds).
const KAME_START_SECS: f32 = 0.2;
const KAME_FIRE_SECS: f32 = 0.283;
/// The glove throw state (`rc_game::hero::registry`, 0x23): Ratchet's Bomb Glove throw standing. In the original he
/// swings the glove back and down for about a quarter second, then throws forward and up; the bomb leaves at the
/// throw's tick 16, when `Weapons::throws` counts it.
const GLOVE_THROW: i32 = 0x23;
/// How long the release motion holds after the bomb leaves.
const RELEASE_SECS: f32 = 0.4;

fn forced() -> Option<usize> { std::env::var("RC_HERO_FORCE").ok().and_then(|v| parse_id(v.trim())) }
fn parse_id(v: &str) -> Option<usize> { v.strip_prefix("0x").map_or_else(|| v.parse().ok(), |h| usize::from_str_radix(h, 16).ok()) }

/// Ratchet's hero state and wrench combo swing (0..2) → (index into [`MOTIONS`], loop).
fn motion(state: i32, swing: i32) -> (usize, bool) {
    use rc_game::hero::state::*;
    match state {
        WALK => (1, true),
        JUMP | RUN_JUMP | LONG_JUMP | DOUBLE_JUMP => (2, false),
        FALL => (3, true),
        COMBO => (4 + swing.clamp(0, 2) as usize, false),
        JUMP_ATTACK => (7, false),
        _ => (0, true),
    }
}

#[derive(Component)]
struct HeroGlb { key: Option<(usize, bool)>, throws: u32, blast: f32, stance: f32, beam: Option<f32> }

/// The Kamehameha's beam (a cylinder) and its head (a sphere): [`beam`].
#[derive(Component)]
struct BeamPart { head: bool }

#[derive(Resource)]
struct Anims { graph: Handle<AnimationGraph>, nodes: Vec<AnimationNodeIndex> }

pub struct HeroGlbPlugin;

impl Plugin for HeroGlbPlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = path() else { return };
        println!("hero_glb: drawing {path} in place of Ratchet");
        app.add_systems(Startup, load).add_systems(Update, (spawn, unlit, attach_graph, tag_joints)).add_systems(
            PostUpdate,
            (
                follow.before(bevy::app::AnimationSystems),
                (pin_root, anchors, beam)
                    .chain()
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .before(crate::moby_attach::upload),
            ),
        );
    }
}

fn load(mut commands: Commands, assets: Res<AssetServer>, mut graphs: ResMut<Assets<AnimationGraph>>) {
    let path = path().unwrap();
    let mut ids = MOTIONS;
    ids[FORCED] = forced().unwrap_or(0);
    let (graph, nodes) = AnimationGraph::from_clips(ids.map(|i| assets.load(GltfAssetLabel::Animation(i).from_asset(path))));
    commands.insert_resource(Anims { graph: graphs.add(graph), nodes });
}

/// One model per running game (a level change drops `Play` and despawns it with the level's entities).
fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    play: Option<Res<Play>>,
    q: Query<(), With<HeroGlb>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if play.is_none() || !q.is_empty() { return; }
    let ki = materials.add(StandardMaterial { base_color: Color::srgba(0.35, 0.75, 1.0, 0.8), unlit: true, alpha_mode: AlphaMode::Add, ..default() });
    commands.spawn((Mesh3d(meshes.add(Cylinder::new(1.0, 1.0))), MeshMaterial3d(ki.clone()), Transform::default(), Visibility::Hidden, BeamPart { head: false }));
    commands.spawn((Mesh3d(meshes.add(Sphere::new(1.0))), MeshMaterial3d(ki), Transform::default(), Visibility::Hidden, BeamPart { head: true }));
    commands.spawn((
        HeroGlb { key: None, throws: 0, blast: 0.0, stance: 0.0, beam: None },
        WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(path().unwrap()))),
        Transform::default(),
        Visibility::Hidden,
        Name::new("Hero glTF"),
    ));
}

/// The levels have no Bevy lights: draw the model's materials unlit.
fn unlit(mut events: MessageReader<AssetEvent<StandardMaterial>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for e in events.read() {
        if let AssetEvent::Added { id } = e {
            if let Some(mut m) = materials.get_mut(*id) { m.unlit = true; }
        }
    }
}

fn attach_graph(mut commands: Commands, anims: Res<Anims>, mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>) {
    for (e, mut p) in &mut players {
        let mut t = AnimationTransitions::new();
        t.play(&mut p, anims.nodes[0], Duration::ZERO).repeat();
        commands.entity(e).insert((AnimationGraphHandle(anims.graph.clone()), t));
    }
}

fn follow(
    play: Option<Res<Play>>,
    anims: Res<Anims>,
    time: Res<Time>,
    mut q: Query<(&mut HeroGlb, &mut Transform, &mut Visibility)>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let (Some(p), Ok((mut h, mut t, mut v))) = (play, q.single_mut()) else { return };
    let Some((pos, rows, hidden)) = p.hero_pose() else { return };
    let hidden = hidden && !third_person_look(&p);
    let want = if hidden { Visibility::Hidden } else { Visibility::Inherited };
    if *v != want { *v = want; }
    let yaw = rows[0][1].atan2(rows[0][0]);
    t.translation = game_to_bevy(pos);
    t.rotation = Quat::from_rotation_y(yaw + env_f32("RC_HERO_YAW", 90.0).to_radians());
    t.scale = Vec3::splat(env_f32("RC_HERO_SCALE", 0.09));
    let hero = &p.game.hero;
    // The bomb leaving the hand (`Weapons::throws` counts the throws, standing or on the run): the release for a moment.
    if hero.weapons.throws != h.throws {
        h.throws = hero.weapons.throws;
        h.blast = RELEASE_SECS;
        h.key = None;
    }
    h.blast = (h.blast - time.delta_secs()).max(0.0);
    // The look stance (L1) charges the Kamehameha; the wrench thrown from it (`rc_game::hero::comet::throw_wrench`) fires
    // it, the beam held until the wrench is back in the hand.
    let dt = time.delta_secs();
    let look = hero.state == 1 || hero.state == 0x1e;
    h.stance = if look { h.stance + dt } else { 0.0 };
    let out = hero.items.slot.item.as_ref().is_some_and(|i| i.o_class == WRENCH_O_CLASS && i.mstate != 0);
    h.beam = match (out, h.beam) {
        (false, _) => None,
        (true, Some(t)) => Some(t + dt),
        (true, None) => look.then_some(0.0),
    };
    let key = if forced().is_some() {
        (FORCED, true)
    } else if h.blast > 0.0 {
        (RELEASE, false)
    } else if hero.state == GLOVE_THROW {
        (WIND_UP, false)
    } else if let Some(t) = h.beam {
        if t < KAME_FIRE_SECS { (KAME_FIRE, false) } else { (KAME_BEAM, true) }
    } else if look {
        if h.stance < KAME_START_SECS { (KAME_START, false) } else { (KAME_CHARGE, true) }
    } else {
        motion(hero.state, hero.melee.combo)
    };
    // Not before the scene's player exists (attach_graph), or the first key would be spent on nothing.
    if h.key != Some(key) && !players.is_empty() {
        // Landing (jump or fall → the idle stance) blends in 20 ms, every other change in 120 ms.
        let landing = matches!(h.key, Some((2 | 3, _))) && key.0 == 0;
        let blend = Duration::from_millis(if landing { 20 } else { 120 });
        h.key = Some(key);
        let (k, looped) = key;
        for (mut pl, mut tr) in &mut players {
            let a = tr.play(&mut pl, anims.nodes[k], blend);
            if looped { a.repeat(); } else { a.replay(); }
        }
    }
}

/// The skeleton's top joint (`nodo_0` in goku.glb). BT3's attack and dash motions move it (root motion, which the
/// battle code rebases into the fighter's position); the idle and run motions leave it where the last motion put it.
/// Ratchet's moby already carries the movement, so it is pinned to the model origin after every animation pass.
#[derive(Component)]
struct RootJoint;

/// The joints the items hang from: the spine for Clank and the pack ([`back_joint`]; bind positions `nodo_16`
/// (0, 11.4, 0.5), `nodo_17` (0, 13.1, 0.4)), the right hand `nodo_21` (−7.4, 14.5, −0.3) for the hand weapon.
#[derive(Component)]
struct BackJoint;
#[derive(Component)]
struct HandJoint;

fn tag_joints(mut commands: Commands, q: Query<(Entity, &Name), Added<Name>>) {
    for (e, n) in &q {
        match n.as_str() {
            "nodo_0" => { commands.entity(e).insert(RootJoint); }
            "nodo_21" => { commands.entity(e).insert(HandJoint); }
            n if n == back_joint() => { commands.entity(e).insert(BackJoint); }
            _ => {}
        }
    }
}

fn pin_root(mut q: Query<&mut Transform, With<RootJoint>>) {
    for mut t in &mut q { t.translation = Vec3::ZERO; }
}

/// The items' frames on the model, every frame after the animation. A joint's skinning matrix (its transforms up to
/// the model root, times its inverse bind matrix) is the mesh node's own matrix at the bind pose, so
/// `delta = skin · mesh⁻¹` is how that part of the body has moved away from the bind pose. An item is placed at the
/// bind pose as on Ratchet, its rows = Ratchet's body rows (Clank's model faces backward on his own), at a point of
/// the model (bind space: y up, face toward +z), then moved by its joint's `delta`: Clank and the pack at
/// `RC_CLANK_POS` (default just behind the shoulder blades) on the spine, the hand weapon (not the wrench) at the
/// right hand. crate::moby_attach draws them there (game-space rows / position).
#[allow(clippy::too_many_arguments)]
fn anchors(
    attach: Option<ResMut<MobyAttach>>,
    play: Option<Res<Play>>,
    root: Query<Entity, With<HeroGlb>>,
    back: Query<Entity, With<BackJoint>>,
    hand: Query<Entity, With<HandJoint>>,
    locals: Query<(&Transform, Option<&ChildOf>)>,
    skins: Query<(Entity, &SkinnedMesh)>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    let (Some(mut attach), Some(p), Ok(root)) = (attach, play, root.single()) else { return };
    let Some((_, rows, hidden)) = p.hero_pose() else { return };
    if hidden && !third_person_look(&p) { return; }
    // An entity's matrix in the world: its local transforms up to the model root (which has no parent).
    let world = |mut e: Entity| -> Option<Mat4> {
        let mut m = Mat4::IDENTITY;
        loop {
            let (t, parent) = locals.get(e).ok()?;
            m = t.to_matrix() * m;
            if e == root { return Some(m); }
            e = parent?.parent();
        }
    };
    let body = Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y) * Mat3::from_cols(rows[0].into(), rows[1].into(), rows[2].into());
    // Bevy -> game axes (the inverse of `game_to_bevy`).
    let game = |v: Vec3| [v.x, -v.z, v.y];
    let frame = |joint: Entity, bind_point: Vec3| -> Option<([[f32; 3]; 3], [f32; 3])> {
        let (mesh, ibm) = skins.iter().find_map(|(e, s)| {
            let i = s.joints.iter().position(|&j| j == joint)?;
            Some((e, *bindposes.get(&s.inverse_bindposes)?.get(i)?))
        })?;
        let mesh = world(mesh)?;
        let delta = world(joint)? * ibm * mesh.inverse();
        let at = mesh.transform_point3(bind_point * Vec3::new(1.0, -1.0, -1.0));
        let b = delta * Mat4::from_mat3_translation(body, at);
        Some(([b.x_axis, b.y_axis, b.z_axis].map(|c| game(c.truncate().normalize_or_zero())), game(b.w_axis.truncate())))
    };
    if let Some((r, at)) = back.single().ok().and_then(|j| frame(j, clank_pos())) { attach.set_anchor(Slot::Back, r, at); }
    let h = hand.single().ok().and_then(|j| frame(j, Vec3::new(-7.4, 14.5, -0.3)));
    if let Some((r, at)) = h { attach.set_anchor(Slot::Hand, r, at); }
    *HAND.lock().unwrap() = h;
}

const WRENCH_O_CLASS: i16 = rc_formats::gadget::WRENCH_O_CLASS as i16;

/// The Kamehameha's beam while it is fired: from the model's right hand (Ratchet's chest in first person, where the
/// model is hidden) to the thrown wrench, the head on the wrench.
fn beam(
    play: Option<Res<Play>>,
    hero: Query<&HeroGlb>,
    mut parts: Query<(&BeamPart, &mut Transform, &mut Visibility), Without<HeroGlb>>,
) {
    let (Some(p), Ok(h)) = (play, hero.single()) else { return };
    let wrench = p.game.hero.items.slot.item.as_ref().filter(|_| h.beam.is_some()).map(|i| i.position);
    let from = p.hero_pose().map(|(pos, _, hidden)| match *HAND.lock().unwrap() {
        Some((_, at)) if !hidden || third_person_look(&p) => at,
        _ => [pos[0], pos[1], pos[2] + 1.0],
    });
    for (part, mut t, mut v) in &mut parts {
        let Some((a, b)) = from.zip(wrench).map(|(a, b)| (game_to_bevy(a), game_to_bevy(b))) else {
            v.set_if_neq(Visibility::Hidden);
            continue;
        };
        v.set_if_neq(Visibility::Inherited);
        // From half a unit out along the beam (in first person it starts just below the camera).
        let a = a + (b - a).clamp_length_max(0.5);
        let d = b - a;
        *t = if part.head {
            Transform::from_translation(b).with_scale(Vec3::splat(0.25))
        } else {
            Transform::from_translation((a + b) / 2.0)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)))
                .with_scale(Vec3::new(0.08, d.length(), 0.08))
        };
    }
}

/// `RC_CLANK_JOINT`: the joint Clank hangs from (default `nodo_16`, the middle of the spine: it turns with the torso without the chest joint's roll in the guard stance).
fn back_joint() -> String { std::env::var("RC_CLANK_JOINT").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "nodo_16".into()) }

/// The hand weapon's frame on the model (game-space rows, position) as [`anchors`] last placed it.
static HAND: std::sync::Mutex<Option<([[f32; 3]; 3], [f32; 3])>> = std::sync::Mutex::new(None);

/// Where to draw a point the game keeps in its own hand weapon (`glove`: that weapon's rows and position, Ratchet's
/// hand): the same point in the weapon on the model's hand. The bomb in the Bomb Glove (crate::gameplay's dynamic
/// mobys) is drawn there; the game still throws it from its own hand point.
pub fn in_model_hand(glove: ([[f32; 3]; 3], [f32; 3]), p: [f32; 3]) -> Option<[f32; 3]> {
    path()?;
    let (rows, at) = (*HAND.lock().unwrap())?;
    let m = |r: [[f32; 3]; 3]| Mat3::from_cols(Vec3::from(r[0]).normalize_or_zero(), Vec3::from(r[1]).normalize_or_zero(), Vec3::from(r[2]).normalize_or_zero());
    let local = m(glove.0).transpose() * (Vec3::from(p) - Vec3::from(glove.1));
    Some((m(rows) * local + Vec3::from(at)).into())
}

fn clank_pos() -> Vec3 {
    std::env::var("RC_CLANK_POS")
        .ok()
        .and_then(|v| {
            let f: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (f.len() == 3).then(|| Vec3::new(f[0], f[1], f[2]))
        })
        .unwrap_or(Vec3::new(0.0, 12.6, -1.3))
}
