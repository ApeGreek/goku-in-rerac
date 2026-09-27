//! The ported class updates and the registry that maps level-table addresses to them.
//!
//! | classes | level01 fn | port |
//! |---|---|---|
//! | 13, 14, 15, 16 (bolts) | 0x2bb758 `BoltUpdate` | [`bolt`] |
//! | 500, 501, 502, 505, 511 (crates) | 0x2ea178 `CrateUpdate` | [`crate_`] |
//! | 724, 725 (grass) | 0x2fa720 `GrassUpdate` | [`grass`] |
//! | 660 (Blarg flyers) | 0x2f4428 `BlargFlyerUpdate` + 0x2f5168 `FlyerPathDriver` | [`flyer`] |
//! | 1135 (teleporter pads) | 0x308bd8 `TeleporterPadUpdate` | [`teleporter`] |
//! | 304, 1456–1465 (gold-weapon offers) | 0x2e1ac0 `ItemOfferUpdate` | [`item_offer`] |
//! | 149, 348, 349, 354–359, 363, 364 (debris pieces) | 0x2c5218 `DebrisUpdate` | [`debris`] |
//! | 112, 1192 (explosion flashes) | 0x2c22a8 `FlashUpdate` | [`debris`] |
//! | 726 (path platform / lift) | 0x2b9eb0 `PathPlatformUpdate` | [`path_platform`] |
//! | 805 (checkpoint triggers) | 0x300220 `CheckpointTriggerUpdate` | [`checkpoint`] |
//! | 679 (flow chutes; levels 1, 5, 8, 15) | 0x2f6328 `FlowUpdate` | [`flow`] |
//! | 758, 803 (Swingshot pull / swing targets; not on level01) | level03 0x2d0bd8 | [`swing_target`] |
//! | 577 (critters; level 1) | 0x2efc60 `GroundCritterUpdate` | [`critter`] |
//! | 572, 865, 866 (amoeboids; levels 1, 5, 11) | 0x2edca0 `AmoeboidUpdate` | [`amoeboid`] |
//! | 1736–1738, 1747–1749, 1761–1763, 1770, 1814, 1815, 1817 (body pieces; level 1) | 0x30cd18 `FxGroupUpdate` | [`crate::moby_update::creature::fx`] |
//! | 639 (explosion light; every level) | 0x2f3748 | [`crate::moby_update::creature::fx`] |
//! | 121 (Bomb Glove bomb), 122 (its fireballs) | 0x2c3300, 0x2c4d88 | [`bomb`] |
//! | 280 (bolt cranks; levels 1, 4, 8), 641 / 665 (the Novalis rotator / sliders they drive) | 0x2e0c68, 0x2f4348, 0x2f4710 | [`bolt_crank`] |
//! | 459 (robot troopers; level 1), 722 (their fire globs) | 0x2e6bf0 `PathEnemyUpdate`, 0x2fa4c0 | [`path_enemy`] |
//! | 666 (dropship; level 1) | 0x2f4960 `DropshipUpdate` | [`dropship`] |
//! | 688 (gunship), 686 (its shells), 700 (fires), 696–698 (embers; level 1) | 0x2f7728, 0x2f6a30, 0x2f8c58, 0x2f8718 | [`gunship`] |
//! | 815 (enemy spawner; level 1) | 0x3021b8 `EnemySpawnerUpdate` | [`enemy_spawner`] |
//! | 11 (Gadgetron vendor; every level) | 0x2bb128 | [`vendor`] |
//! | 774 (talking NPC; levels 1, 8) | 0x2ff118 `TalkingNpcUpdate` | [`talking_npc`] |
//! | 760 (fire / smoke fields on the bombed buildings), 809 (their smoke scroll; levels 0, 1, 14) | 0x2fdbc0 `ParticleFieldUpdate`, 0x2ba658 | [`fire_field`] (draw callbacks: [`draw_callbacks`]) |
//! | 705 (spinners), 703 / 715 (elevators), 768 / 769 (sliding doors), 1042 (shootables), 701 (collapsing platforms; level 1) | 0x2f9bf0, 0x2f95c0, 0x2fed68, 0x307c48, 0x2f9080 | [`props`] |
//! | 704 (rocks), 709–711 (shell walls), 729 (big wall), 778 (pipe) and its spray 779, 754 (pots), 1813 (boxes), 1816 (their remains; level 1) | 0x2f9810, 0x2f9d80, 0x2fa800, 0x2ff860, 0x2ffb28, 0x2fd9a0, 0x30d0f0, 0x30d200 | [`breakables`] |
//! | 737 (camera triggers; levels 1, 2, 3, 7, 10, 13), 730 / 790 (mission NPCs), 746 (hinged bridge; level 1) | 0x2fb5b0, 0x2fad68, 0x2fb8a8 | [`camera_trigger`], [`mission_npc`], [`hinged_bridge`] |

pub mod amoeboid;
pub mod bolt;
pub mod bolt_crank;
pub mod bomb;
pub mod breakables;
pub mod camera_trigger;
pub mod checkpoint;
pub mod crate_;
pub mod critter;
pub mod debris;
pub mod draw_callbacks;
pub mod dropship;
pub mod enemy_spawner;
pub mod flow;
pub mod flyer;
pub mod fire_field;
pub mod grass;
pub mod hinged_bridge;
pub mod gunship;
pub mod item_offer;
pub mod mission_npc;
pub mod path_enemy;
pub mod path_platform;
pub mod props;
pub mod swing_target;
pub mod talking_npc;
pub mod teleporter;
pub mod vendor;

use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

/// A ported update function (the value of `moby+0x74`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClassUpdate {
    Bolt,
    Crate,
    Grass,
    Flyer,
    TeleporterPad,
    ItemOffer,
    Debris,
    Flash,
    PathPlatform,
    Checkpoint,
    Flow,
    SwingTarget,
    Critter,
    FxPiece,
    ExplosionLight,
    Amoeboid,
    Bomb,
    Fireball,
    BoltCrank,
    CrankRotator,
    CrankSlider,
    PathEnemy,
    PathEnemyShot,
    Dropship,
    Gunship,
    GunshipShell,
    GunshipFire,
    GunshipEmber,
    EnemySpawner,
    Vendor,
    TalkingNpc,
    FireField,
    TextureScroll,
    Spinner,
    Elevator,
    SlidingDoor,
    Shootable,
    Collapse,
    Rock,
    ShellWall,
    Wall,
    Pipe,
    Spray,
    Pot,
    Box,
    Remains,
    CameraTrigger,
    MissionNpc,
    HingedBridge,
}

impl ClassUpdate {
    pub const ALL: [ClassUpdate; 49] = [
        ClassUpdate::Bolt,
        ClassUpdate::Crate,
        ClassUpdate::Grass,
        ClassUpdate::Flyer,
        ClassUpdate::TeleporterPad,
        ClassUpdate::ItemOffer,
        ClassUpdate::Debris,
        ClassUpdate::Flash,
        ClassUpdate::PathPlatform,
        ClassUpdate::Checkpoint,
        ClassUpdate::Flow,
        ClassUpdate::SwingTarget,
        ClassUpdate::Critter,
        ClassUpdate::FxPiece,
        ClassUpdate::ExplosionLight,
        ClassUpdate::Amoeboid,
        ClassUpdate::Bomb,
        ClassUpdate::Fireball,
        ClassUpdate::BoltCrank,
        ClassUpdate::CrankRotator,
        ClassUpdate::CrankSlider,
        ClassUpdate::PathEnemy,
        ClassUpdate::PathEnemyShot,
        ClassUpdate::Dropship,
        ClassUpdate::Gunship,
        ClassUpdate::GunshipShell,
        ClassUpdate::GunshipFire,
        ClassUpdate::GunshipEmber,
        ClassUpdate::EnemySpawner,
        ClassUpdate::Vendor,
        ClassUpdate::TalkingNpc,
        ClassUpdate::FireField,
        ClassUpdate::TextureScroll,
        ClassUpdate::Spinner,
        ClassUpdate::Elevator,
        ClassUpdate::SlidingDoor,
        ClassUpdate::Shootable,
        ClassUpdate::Collapse,
        ClassUpdate::Rock,
        ClassUpdate::ShellWall,
        ClassUpdate::Wall,
        ClassUpdate::Pipe,
        ClassUpdate::Spray,
        ClassUpdate::Pot,
        ClassUpdate::Box,
        ClassUpdate::Remains,
        ClassUpdate::CameraTrigger,
        ClassUpdate::MissionNpc,
        ClassUpdate::HingedBridge,
    ];

    /// The level01 class-table address of this update.
    pub const fn address(self) -> u32 {
        match self {
            ClassUpdate::Bomb => bomb::UPDATE_FN,
            ClassUpdate::Fireball => bomb::FIREBALL_UPDATE_FN,
            ClassUpdate::BoltCrank => bolt_crank::UPDATE_FN,
            ClassUpdate::CrankRotator => bolt_crank::ROTATOR_UPDATE_FN,
            ClassUpdate::CrankSlider => bolt_crank::SLIDER_UPDATE_FN,
            ClassUpdate::PathEnemy => path_enemy::UPDATE_FN,
            ClassUpdate::PathEnemyShot => path_enemy::SHOT_UPDATE_FN,
            ClassUpdate::Dropship => dropship::UPDATE_FN,
            ClassUpdate::Gunship => gunship::UPDATE_FN,
            ClassUpdate::GunshipShell => gunship::SHELL_FN,
            ClassUpdate::GunshipFire => gunship::FIRE_FN,
            ClassUpdate::GunshipEmber => gunship::EMBER_FN,
            ClassUpdate::EnemySpawner => enemy_spawner::UPDATE_FN,
            ClassUpdate::Vendor => vendor::UPDATE_FN,
            ClassUpdate::TalkingNpc => talking_npc::UPDATE_FN,
            ClassUpdate::Bolt => bolt::UPDATE_FN,
            ClassUpdate::Crate => crate_::UPDATE_FN,
            ClassUpdate::Grass => grass::UPDATE_FN,
            ClassUpdate::Flyer => flyer::UPDATE_FN,
            ClassUpdate::TeleporterPad => teleporter::UPDATE_FN,
            ClassUpdate::ItemOffer => item_offer::UPDATE_FN,
            ClassUpdate::Debris => debris::UPDATE_FN,
            ClassUpdate::Flash => debris::FLASH_UPDATE_FN,
            ClassUpdate::PathPlatform => path_platform::UPDATE_FN,
            ClassUpdate::Checkpoint => checkpoint::UPDATE_FN,
            ClassUpdate::Flow => flow::UPDATE_FN,
            ClassUpdate::SwingTarget => swing_target::UPDATE_FN,
            ClassUpdate::Critter => critter::UPDATE_FN,
            ClassUpdate::FxPiece => crate::moby_update::creature::fx::PIECE_UPDATE_FN,
            ClassUpdate::ExplosionLight => crate::moby_update::creature::fx::LIGHT_UPDATE_FN,
            ClassUpdate::Amoeboid => amoeboid::UPDATE_FN,
            ClassUpdate::FireField => fire_field::UPDATE_FN,
            ClassUpdate::TextureScroll => fire_field::SCROLL_UPDATE_FN,
            ClassUpdate::Spinner => props::SPINNER_FN,
            ClassUpdate::Elevator => props::ELEVATOR_FN,
            ClassUpdate::SlidingDoor => props::DOOR_FN,
            ClassUpdate::Shootable => props::SHOOTABLE_FN,
            ClassUpdate::Collapse => props::COLLAPSE_FN,
            ClassUpdate::Rock => breakables::ROCK_FN,
            ClassUpdate::ShellWall => breakables::SHELL_WALL_FN,
            ClassUpdate::Wall => breakables::WALL_FN,
            ClassUpdate::Pipe => breakables::PIPE_FN,
            ClassUpdate::Spray => breakables::SPRAY_FN,
            ClassUpdate::Pot => breakables::POT_FN,
            ClassUpdate::Box => breakables::BOX_FN,
            ClassUpdate::Remains => breakables::REMAINS_FN,
            ClassUpdate::CameraTrigger => camera_trigger::UPDATE_FN,
            ClassUpdate::MissionNpc => mission_npc::UPDATE_FN,
            ClassUpdate::HingedBridge => hinged_bridge::UPDATE_FN,
        }
    }

    pub fn from_address(a: u32) -> Option<ClassUpdate> { ClassUpdate::ALL.into_iter().find(|u| u.address() == a) }

    /// The classes the level01 table maps to this function.
    pub fn classes(self) -> &'static [i16] {
        match self {
            ClassUpdate::Bomb => &bomb::CLASSES,
            ClassUpdate::Fireball => &bomb::FIREBALL_CLASSES,
            ClassUpdate::BoltCrank => &bolt_crank::CLASSES,
            ClassUpdate::CrankRotator => &bolt_crank::ROTATOR_CLASSES,
            ClassUpdate::CrankSlider => &bolt_crank::SLIDER_CLASSES,
            ClassUpdate::PathEnemy => &path_enemy::CLASSES,
            ClassUpdate::PathEnemyShot => &path_enemy::SHOT_CLASSES,
            ClassUpdate::Dropship => &dropship::CLASSES,
            ClassUpdate::Gunship => &gunship::CLASSES,
            ClassUpdate::GunshipShell => &gunship::SHELL_CLASSES,
            ClassUpdate::GunshipFire => &gunship::FIRE_CLASSES,
            ClassUpdate::GunshipEmber => &gunship::EMBER_CLASSES,
            ClassUpdate::EnemySpawner => &enemy_spawner::CLASSES,
            ClassUpdate::Vendor => &vendor::CLASSES,
            ClassUpdate::TalkingNpc => &talking_npc::CLASSES,
            ClassUpdate::Bolt => &bolt::CLASSES,
            ClassUpdate::Crate => &crate_::CLASSES,
            ClassUpdate::Grass => &grass::CLASSES,
            ClassUpdate::Flyer => &flyer::CLASSES,
            ClassUpdate::TeleporterPad => &teleporter::CLASSES,
            ClassUpdate::ItemOffer => &item_offer::CLASSES,
            ClassUpdate::Debris => &debris::CLASSES,
            ClassUpdate::Flash => &debris::FLASH_CLASSES,
            ClassUpdate::PathPlatform => &path_platform::CLASSES,
            ClassUpdate::Checkpoint => &checkpoint::CLASSES,
            ClassUpdate::Flow => &flow::CLASSES,
            ClassUpdate::SwingTarget => &swing_target::CLASSES,
            ClassUpdate::Critter => &critter::CLASSES,
            ClassUpdate::FxPiece => &crate::moby_update::creature::fx::PIECE_CLASSES,
            ClassUpdate::ExplosionLight => &[crate::moby_update::creature::fx::LIGHT_CLASS],
            ClassUpdate::Amoeboid => &amoeboid::CLASSES,
            ClassUpdate::FireField => &fire_field::CLASSES,
            ClassUpdate::TextureScroll => &fire_field::SCROLL_CLASSES,
            ClassUpdate::Spinner => &props::SPINNER_CLASSES,
            ClassUpdate::Elevator => &props::ELEVATOR_CLASSES,
            ClassUpdate::SlidingDoor => &props::DOOR_CLASSES,
            ClassUpdate::Shootable => &props::SHOOTABLE_CLASSES,
            ClassUpdate::Collapse => &props::COLLAPSE_CLASSES,
            ClassUpdate::Rock => &breakables::ROCK_CLASSES,
            ClassUpdate::ShellWall => &breakables::SHELL_WALL_CLASSES,
            ClassUpdate::Wall => &breakables::WALL_CLASSES,
            ClassUpdate::Pipe => &breakables::PIPE_CLASSES,
            ClassUpdate::Spray => &breakables::SPRAY_CLASSES,
            ClassUpdate::Pot => &breakables::POT_CLASSES,
            ClassUpdate::Box => &breakables::BOX_CLASSES,
            ClassUpdate::Remains => &breakables::REMAINS_CLASSES,
            ClassUpdate::CameraTrigger => &camera_trigger::CLASSES,
            ClassUpdate::MissionNpc => &mission_npc::CLASSES,
            ClassUpdate::HingedBridge => &hinged_bridge::CLASSES,
        }
    }
}

/// The registry: the Rust port of `o_class`'s update, if any.
pub fn for_class(o_class: i16) -> Option<ClassUpdate> { ClassUpdate::ALL.into_iter().find(|u| u.classes().contains(&o_class)) }

/// `(*moby+0x74)(moby)`.
pub fn dispatch(u: ClassUpdate, w: &mut World, id: MobyId) {
    match u {
        ClassUpdate::Bomb => bomb::update(w, id),
        ClassUpdate::Fireball => bomb::fireball_update(w, id),
        ClassUpdate::BoltCrank => bolt_crank::update(w, id),
        ClassUpdate::CrankRotator => bolt_crank::rotator_update(w, id),
        ClassUpdate::CrankSlider => bolt_crank::slider_update(w, id),
        ClassUpdate::PathEnemy => path_enemy::update(w, id),
        ClassUpdate::PathEnemyShot => path_enemy::shot_update(w, id),
        ClassUpdate::Dropship => dropship::update(w, id),
        ClassUpdate::Gunship => gunship::update(w, id),
        ClassUpdate::GunshipShell => gunship::shell_update(w, id),
        ClassUpdate::GunshipFire => gunship::fire_update(w, id),
        ClassUpdate::GunshipEmber => gunship::ember_update(w, id),
        ClassUpdate::EnemySpawner => enemy_spawner::update(w, id),
        ClassUpdate::Vendor => vendor::update(w, id),
        ClassUpdate::TalkingNpc => talking_npc::update(w, id),
        ClassUpdate::Bolt => bolt::update(w, id),
        ClassUpdate::Crate => crate_::update(w, id),
        ClassUpdate::Grass => grass::update(w, id),
        ClassUpdate::Flyer => flyer::update(w, id),
        ClassUpdate::TeleporterPad => teleporter::update(w, id),
        ClassUpdate::ItemOffer => item_offer::update(w, id),
        ClassUpdate::Debris => debris::update(w, id),
        ClassUpdate::Flash => debris::flash_update(w, id),
        ClassUpdate::PathPlatform => path_platform::update(w, id),
        ClassUpdate::Checkpoint => checkpoint::update(w, id),
        ClassUpdate::Flow => flow::update(w, id),
        ClassUpdate::SwingTarget => swing_target::update(w, id),
        ClassUpdate::Critter => critter::update(w, id),
        ClassUpdate::FxPiece => crate::moby_update::creature::fx::piece_update(w, id),
        ClassUpdate::ExplosionLight => crate::moby_update::creature::fx::light_update(w, id),
        ClassUpdate::Amoeboid => amoeboid::update(w, id),
        ClassUpdate::FireField => fire_field::update(w, id),
        ClassUpdate::TextureScroll => fire_field::scroll_update(w, id),
        ClassUpdate::Spinner => props::spinner_update(w, id),
        ClassUpdate::Elevator => props::elevator_update(w, id),
        ClassUpdate::SlidingDoor => props::door_update(w, id),
        ClassUpdate::Shootable => props::shootable_update(w, id),
        ClassUpdate::Collapse => props::collapse_update(w, id),
        ClassUpdate::Rock => breakables::rock_update(w, id),
        ClassUpdate::ShellWall => breakables::shell_wall_update(w, id),
        ClassUpdate::Wall => breakables::wall_update(w, id),
        ClassUpdate::Pipe => breakables::pipe_update(w, id),
        ClassUpdate::Spray => breakables::spray_update(w, id),
        ClassUpdate::Pot => breakables::pot_update(w, id),
        ClassUpdate::Box => breakables::box_update(w, id),
        ClassUpdate::Remains => breakables::remains_update(w, id),
        ClassUpdate::CameraTrigger => camera_trigger::update(w, id),
        ClassUpdate::MissionNpc => mission_npc::update(w, id),
        ClassUpdate::HingedBridge => hinged_bridge::update(w, id),
    }
}

/// Classes whose update reads joint points (`FUN_002645a8`, [`World::joint_point`]): the loader fills
/// [`Services::joint_lists`](crate::moby_update::Services) for them from the class blob.
pub fn needs_joint_lists(o_class: i16) -> bool {
    flyer::CLASSES.contains(&o_class) || path_enemy::CLASSES.contains(&o_class) || gunship::CLASSES.contains(&o_class)
}
