//! The world's colour-only draws before the shadow pass (docs/plan/shadows.md §7 risk 5): the halves of the
//! world's alpha tests that write colour only (`GsPass::ColorOnlyLowAlpha`: tfrags, ties, shrubs) and the shrub
//! billboards' pass 2 (`GsPass::BlendNoZ`). The game draws them with the world, before the shadow volumes darken the
//! frame, so a shadow darkens them too; Bevy would draw them in the transparent pass, after crate::shadow_render's
//! darkening. Their items are taken out of the view's `Transparent3d` phase (in sorted order) and drawn right after
//! the opaque pass, against the scene depth, then put back for Bevy's retained phase (as crate::display_blend does
//! for the effects).
//!
//! Which entities: [`BeforeShadows`], added when a world entity first gets a material whose GS pass is one of the
//! two ([`tag`], one system per world material type).

use crate::gs_state::GsPass;
use bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d, Transparent3d};
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_phase::{SortedRenderPhase, ViewSortedRenderPhases};
use bevy::render::render_resource::{RenderPassDescriptor, StoreOp};
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::sync_world::MainEntity;
use bevy::render::view::{ExtractedView, ViewDepthTexture, ViewTarget};
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use std::sync::Mutex;

/// A world entity drawn before the shadow pass (module doc).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BeforeShadows;

/// Tags the new entities of material `M` whose GS pass writes colour only.
pub fn tag<M: Material>(mut commands: Commands, q: Query<(Entity, &MeshMaterial3d<M>), Added<MeshMaterial3d<M>>>, mats: Res<Assets<M>>)
where
    for<'a> GsPass: From<&'a M>,
{
    for (e, m) in &q {
        let Some(mat) = mats.get(&m.0) else { continue };
        if matches!(GsPass::from(mat), GsPass::ColorOnlyLowAlpha { .. } | GsPass::BlendNoZ) { commands.entity(e).insert(BeforeShadows); }
    }
}

pub struct PreShadowPlugin;

impl Plugin for PreShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                tag::<crate::tfrag_render::TfragMaterial>,
                tag::<crate::tie_render::TieMaterial>,
                tag::<crate::shrub_render::ShrubMaterial>,
                tag::<crate::shrub_billboard::BillboardMaterial>,
            ),
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app.init_resource::<Tagged>().init_resource::<Stash>().add_systems(ExtractSchedule, extract).add_systems(
            Core3d,
            (
                (split, draw).chain().after(main_opaque_pass_3d).before(crate::shadow_render::ShadowPassSet),
                restore.after(main_transparent_pass_3d),
            )
                .in_set(Core3dSystems::MainPass),
        );
    }
}

/// The main-world entities tagged this frame.
#[derive(Resource, Default)]
struct Tagged(HashSet<MainEntity>);

fn extract(mut set: ResMut<Tagged>, q: Extract<Query<Entity, With<BeforeShadows>>>) {
    set.0.clear();
    set.0.extend(q.iter().map(MainEntity::from));
}

/// Per view: the items taken out of its `Transparent3d` phase this frame.
#[derive(Resource, Default)]
struct Stash(Mutex<HashMap<Entity, SortedRenderPhase<Transparent3d>>>);

fn split(view: ViewQuery<&ExtractedView>, mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, set: Res<Tagged>, stash: Res<Stash>) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    if set.0.is_empty() { return; }
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    let keys: Vec<_> = phase.items.keys().filter(|k| set.0.contains(&k.1)).copied().collect();
    if keys.is_empty() { return; }
    let mut out = SortedRenderPhase::<Transparent3d>::default();
    for k in keys {
        if let Some(item) = phase.items.shift_remove(&k) { out.items.insert(k, item); }
    }
    stash.0.lock().unwrap_or_else(|e| e.into_inner()).insert(view_entity, out);
}

fn draw(world: &World, view: ViewQuery<(&ExtractedCamera, &ViewTarget, &ViewDepthTexture)>, mut ctx: RenderContext) {
    let view_entity = view.entity();
    let (camera, target, depth) = view.into_inner();
    let stash = world.resource::<Stash>();
    let guard = stash.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(phase) = guard.get(&view_entity).filter(|p| !p.items.is_empty()) else { return };
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("world colour-only draws (before the shadows)"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(v) = camera.viewport.as_ref() { pass.set_camera_viewport(v); }
    if let Err(err) = phase.render(&mut pass, world, view_entity) { error!("pre-shadow draws: {err:?}"); }
}

fn restore(view: ViewQuery<&ExtractedView>, mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>, stash: Res<Stash>) {
    let view_entity = view.entity();
    let ev = view.into_inner();
    let Some(out) = stash.0.lock().unwrap_or_else(|e| e.into_inner()).remove(&view_entity) else { return };
    let Some(phase) = phases.get_mut(&ev.retained_view_entity) else { return };
    for (k, item) in out.items { phase.items.insert(k, item); }
}
