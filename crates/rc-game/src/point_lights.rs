//! The eight dynamic point lights (level01 bank 0x180740, 8 × 0x20 bytes; per slot `{r, g, b, intensity}` then
//! `{position, radius}`, the active flag at 0x180950 + 0x30·i): `WritePointLight_B` 0x252750 takes the first free
//! slot (only while the frame-load measure 0x15f5d4 is at most 0.8), the owner rewrites its slot every tick, and
//! `FreePointLight` 0x252850 clears it. The explosion light moby (class 0x27f, `crate::moby_update::creature::fx`)
//! is the owner on every level.
//!
//! Who reads the bank in the game: `MobyProc` merges the lights in range of a moby into its third directional light
//! (docs/plan/moby_skinning_lighting.md "Point lights"; the port: `rc-engine` moby_render `point_light_merge`), and
//! `LightTfrags` / `LightTies` / `LightShrubs` relight the world instances whose point-light lists name a slot
//! (docs/plan/tfrag_lighting.md; not in the port yet).

/// One slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    /// +0x00..+0x08: colour (the light moby's channels / 100; 1.0 adds 128 to a colour byte).
    pub color: [f32; 3],
    /// +0x0c: intensity (the moby's +0x24 byte / 100).
    pub intensity: f32,
    /// +0x10..+0x18: position (game units).
    pub pos: [f32; 3],
    /// +0x1c: radius (game units).
    pub radius: f32,
}

/// The bank.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointLights {
    pub slots: [Option<PointLight>; 8],
}

impl PointLights {
    /// `WritePointLight_B` 0x252750: the first free slot gets `l` (None when all eight are taken, or when the frame
    /// load `load` (0x15f5d4) is above 0.8).
    pub fn alloc(&mut self, l: PointLight, load: f32) -> Option<usize> {
        if load > 0.8 { return None; }
        let i = self.slots.iter().position(|s| s.is_none())?;
        self.slots[i] = Some(l);
        Some(i)
    }

    /// The owner's per-tick rewrite of slot `i`.
    pub fn set(&mut self, i: usize, l: PointLight) {
        if let Some(s) = self.slots.get_mut(i) { *s = Some(l); }
    }

    /// `FreePointLight` 0x252850.
    pub fn free(&mut self, i: usize) {
        if let Some(s) = self.slots.get_mut(i) { *s = None; }
    }

    /// The active slots.
    pub fn active(&self) -> impl Iterator<Item = &PointLight> { self.slots.iter().flatten() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_free_slot_and_load_gate() {
        let l = PointLight { color: [1.0; 3], intensity: 0.0, pos: [0.0; 3], radius: 5.0 };
        let mut b = PointLights::default();
        assert_eq!(b.alloc(l, 0.0), Some(0));
        assert_eq!(b.alloc(l, 0.0), Some(1));
        b.free(0);
        assert_eq!(b.alloc(l, 0.9), None);
        assert_eq!(b.alloc(l, 0.8), Some(0));
        for _ in 0..6 { assert!(b.alloc(l, 0.0).is_some()); }
        assert_eq!(b.alloc(l, 0.0), None);
        assert_eq!(b.active().count(), 8);
    }
}
