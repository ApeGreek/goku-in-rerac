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

pub mod bolt;
pub mod crate_;
pub mod debris;
pub mod flyer;
pub mod grass;
pub mod item_offer;
pub mod path_platform;
pub mod teleporter;

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
}

impl ClassUpdate {
    pub const ALL: [ClassUpdate; 9] = [
        ClassUpdate::Bolt,
        ClassUpdate::Crate,
        ClassUpdate::Grass,
        ClassUpdate::Flyer,
        ClassUpdate::TeleporterPad,
        ClassUpdate::ItemOffer,
        ClassUpdate::Debris,
        ClassUpdate::Flash,
        ClassUpdate::PathPlatform,
    ];

    /// The level01 class-table address of this update.
    pub const fn address(self) -> u32 {
        match self {
            ClassUpdate::Bolt => bolt::UPDATE_FN,
            ClassUpdate::Crate => crate_::UPDATE_FN,
            ClassUpdate::Grass => grass::UPDATE_FN,
            ClassUpdate::Flyer => flyer::UPDATE_FN,
            ClassUpdate::TeleporterPad => teleporter::UPDATE_FN,
            ClassUpdate::ItemOffer => item_offer::UPDATE_FN,
            ClassUpdate::Debris => debris::UPDATE_FN,
            ClassUpdate::Flash => debris::FLASH_UPDATE_FN,
            ClassUpdate::PathPlatform => path_platform::UPDATE_FN,
        }
    }

    pub fn from_address(a: u32) -> Option<ClassUpdate> { ClassUpdate::ALL.into_iter().find(|u| u.address() == a) }

    /// The classes the level01 table maps to this function.
    pub fn classes(self) -> &'static [i16] {
        match self {
            ClassUpdate::Bolt => &bolt::CLASSES,
            ClassUpdate::Crate => &crate_::CLASSES,
            ClassUpdate::Grass => &grass::CLASSES,
            ClassUpdate::Flyer => &flyer::CLASSES,
            ClassUpdate::TeleporterPad => &teleporter::CLASSES,
            ClassUpdate::ItemOffer => &item_offer::CLASSES,
            ClassUpdate::Debris => &debris::CLASSES,
            ClassUpdate::Flash => &debris::FLASH_CLASSES,
            ClassUpdate::PathPlatform => &path_platform::CLASSES,
        }
    }
}

/// The registry: the Rust port of `o_class`'s update, if any.
pub fn for_class(o_class: i16) -> Option<ClassUpdate> { ClassUpdate::ALL.into_iter().find(|u| u.classes().contains(&o_class)) }

/// `(*moby+0x74)(moby)`.
pub fn dispatch(u: ClassUpdate, w: &mut World, id: MobyId) {
    match u {
        ClassUpdate::Bolt => bolt::update(w, id),
        ClassUpdate::Crate => crate_::update(w, id),
        ClassUpdate::Grass => grass::update(w, id),
        ClassUpdate::Flyer => flyer::update(w, id),
        ClassUpdate::TeleporterPad => teleporter::update(w, id),
        ClassUpdate::ItemOffer => item_offer::update(w, id),
        ClassUpdate::Debris => debris::update(w, id),
        ClassUpdate::Flash => debris::flash_update(w, id),
        ClassUpdate::PathPlatform => path_platform::update(w, id),
    }
}

/// Classes whose update reads joint points (`FUN_002645a8`, [`World::joint_point`]): the loader fills
/// [`Services::joint_lists`](crate::moby_update::Services) for them from the class blob.
pub fn needs_joint_lists(o_class: i16) -> bool { flyer::CLASSES.contains(&o_class) }
