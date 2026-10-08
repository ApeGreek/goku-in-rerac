//! Local tool: `RC_EXPORT_CLASS=<o_class>` writes that moby class of the loaded level (a level class, or a gadget such
//! as 192, the Bomb Glove) as Wavefront OBJ + MTL + one PNG per texture into `RC_EXPORT_DIR` (default `export`), for
//! editing in Blender, then the game goes on. High LOD, bind pose, world units (class scale / 1024), Y up (game z),
//! one material per texture; the metal (chrome / glass) packets are left out. Texture alpha is the GS's (0x80 = opaque),
//! doubled for the PNG.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rc_formats::moby::LevelMobyClass;
use rc_formats::texture::TextureTable;
use std::fmt::Write as _;
use std::path::PathBuf;

pub struct ExportObjPlugin;

impl Plugin for ExportObjPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var("RC_EXPORT_CLASS").is_ok() { app.add_systems(Update, export); }
    }
}

fn export(mut done: Local<bool>, level: Option<Res<crate::Level>>) {
    let Some(level) = level else { return };
    if *done { return; }
    *done = true;
    let Some(o_class) = std::env::var("RC_EXPORT_CLASS").ok().and_then(|v| v.trim().parse::<i32>().ok()) else { return };
    let dir = PathBuf::from(std::env::var("RC_EXPORT_DIR").unwrap_or_else(|_| "export".into()));
    let gadgets = crate::moby_attach::load_blobs().map(|(_, g)| g).unwrap_or_default();
    let class = level.0.mobys.classes.iter().chain(gadgets.iter().map(|g| &g.moby)).find(|c| c.o_class == o_class);
    let Some(class) = class else { eprintln!("export: no class {o_class} in this level or its gadgets"); return };
    match write(&level.0, class, &dir) {
        Ok(n) => println!("export: class {o_class} -> {} ({n} triangles)", dir.display()),
        Err(e) => eprintln!("export: class {o_class}: {e:#}"),
    }
}

fn write(level: &crate::level_load::LoadedLevel, class: &LevelMobyClass, dir: &std::path::Path) -> anyhow::Result<usize> {
    std::fs::create_dir_all(dir)?;
    let name = format!("class_{:04}", class.o_class);
    let k = class.class.header.scale / 1024.0;
    let (mut obj, mut mtl) = (format!("mtllib {name}.mtl\n"), String::new());
    let mut textures: Vec<i32> = Vec::new();
    let (mut base, mut tris) = (1usize, 0usize);
    for (si, sub) in class.class.high_lod.iter().enumerate() {
        writeln!(obj, "o packet_{si}")?;
        for v in &sub.vertices {
            // Game (x forward, y left, z up) -> Y up.
            let [x, y, z] = [v.x as f32 * k, v.y as f32 * k, v.z as f32 * k];
            writeln!(obj, "v {x} {z} {}", -y)?;
            writeln!(obj, "vt {} {}", v.st[0] as f32 / 4096.0, 1.0 - v.st[1] as f32 / 4096.0)?;
            let [nx, ny, nz] = class.class.normal(v);
            writeln!(obj, "vn {nx} {nz} {}", -ny)?;
        }
        let mut current = None;
        for t in &sub.triangles {
            if t.texture < -1 { continue; }
            if current != Some(t.texture) {
                current = Some(t.texture);
                if !textures.contains(&t.texture) { textures.push(t.texture); }
                writeln!(obj, "usemtl tex_{}", t.texture)?;
            }
            // The strips' winding is not normalised (the GS does not cull): turn each face to its vertex normals.
            let p = |i: u32| { let v = &sub.vertices[i as usize]; Vec3::new(v.x as f32, v.y as f32, v.z as f32) };
            let n: Vec3 = [t.a, t.b, t.c].iter().map(|&i| Vec3::from(class.class.normal(&sub.vertices[i as usize]))).sum();
            let (b, c) = if (p(t.b) - p(t.a)).cross(p(t.c) - p(t.a)).dot(n) < 0.0 { (t.c, t.b) } else { (t.b, t.c) };
            let f = |i: u32| base + i as usize;
            writeln!(obj, "f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}", f(t.a), f(b), f(c))?;
            tris += 1;
        }
        base += sub.vertices.len();
    }
    for &t in &textures {
        writeln!(mtl, "newmtl tex_{t}\nKd 1 1 1")?;
        let Some(tex) = class.texture_table_index(t).and_then(|i| level.textures.iter().find(|x| x.table == TextureTable::Moby && x.index == i)) else { continue };
        let tx = &tex.texture;
        let rgba: Vec<u8> = tx.rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2], (p[3] as u16 * 2).min(255) as u8]).collect();
        let img = Image::new(Extent3d { width: tx.width, height: tx.height, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::all());
        let png = format!("{name}_tex_{t}.png");
        match img.try_into_dynamic() {
            Ok(d) => d.to_rgba8().save(dir.join(&png))?,
            Err(e) => anyhow::bail!("texture {t}: {e:?}"),
        }
        writeln!(mtl, "map_Kd {png}")?;
    }
    std::fs::write(dir.join(format!("{name}.obj")), obj)?;
    std::fs::write(dir.join(format!("{name}.mtl")), mtl)?;
    Ok(tris)
}
