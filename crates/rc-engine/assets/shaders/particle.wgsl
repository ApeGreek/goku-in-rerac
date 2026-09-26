// Particle sprites (render kind 0) as PartProc + VU1 program 221571 draw them. Rust side, the cull/sort rules
// and every constant: particle_render.rs (docs/plan/particles.md §7).
//
// Per vertex: sprite index << 2 | corner, from a persistent quad mesh. Per sprite (storage buffer, rewritten
// every frame in draw order; quads past `count` are dropped): the particle centre (Bevy world), the corner
// vectors a and b in 512x416 frame-buffer pixels (x right, y down; corners S+a, S+b, S-b, S-a with ST (0,0),
// (1,0), (0,1), (1,1)), the vertex RGBA (0x80 = 1.0, alpha already faded) and the texture layer | additive << 16.
// The corner keeps the centre's depth: the offset is added in clip space (x / 256, -y / 208 in NDC).
//
// Fragment: MODULATE (C = Ct·Cv >> 7, A = At·Av >> 7, At in GS units: the texture stores 0x80 = 1.0),
// TEST_1 0x5380b (draw 0 keeps A >= 0x80 and writes Z, draw 1 keeps the rest), then premultiplied output for
// `One, OneMinusSrcAlpha`: rgb = Cs·As, alpha = As (ALPHA_1 0x44); for 0x48 (additive) alpha 0 and rgb = the
// linear increment that makes the target's display bytes Cd + (Cs·As >> 7) (clamped to 255), as the GS adds:
// Cd is read from the scene snapshot (`view_transmission_texture`, the frame after the opaque passes; see
// particle_render.rs "Additive in display bytes"). Draw 2 is the snapshot trigger and draws nothing.

#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::view_transmission_texture

struct ParticleParams {
    // x = draw (0 = A >= AREF with Z, 1 = the rest), y = AREF.
    misc: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var tex: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: ParticleParams;

struct Sprite {
    // centre xyz, a.x
    p: vec4<f32>,
    // a.y, b.x, b.y, 0
    q: vec4<f32>,
    // RGBA, tag
    c: vec4<u32>,
}

struct Sprites {
    count: vec4<u32>,
    items: array<Sprite>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<storage, read> sprites: Sprites;

struct ParticleVertex {
    @location(0) sprite_corner: u32,
}

struct ParticleVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // Vertex RGBA bytes (0..255, 0x80 = 1.0); flat like the GS (PRIM 0x54: no Gouraud).
    @location(1) @interpolate(flat) color: vec4<u32>,
    @location(2) @interpolate(flat) tag: u32,
}

@vertex
fn vertex(v: ParticleVertex) -> ParticleVertexOutput {
    var out: ParticleVertexOutput;
    let i = v.sprite_corner >> 2u;
    let k = v.sprite_corner & 3u;
    if (i >= sprites.count.x) {
        // Unused quad: all four corners on one point outside the clip volume (no fragments).
        out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }
    let s = sprites.items[i];
    let a = vec2<f32>(s.p.w, s.q.x);
    let b = s.q.yz;
    var corner = a;
    var st = vec2<f32>(0.0, 0.0);
    if (k == 1u) { corner = b; st = vec2<f32>(1.0, 0.0); }
    if (k == 2u) { corner = -b; st = vec2<f32>(0.0, 1.0); }
    if (k == 3u) { corner = -a; st = vec2<f32>(1.0, 1.0); }
    var clip = position_world_to_clip(s.p.xyz);
    clip.x += corner.x * (1.0 / 256.0) * clip.w;
    clip.y -= corner.y * (1.0 / 208.0) * clip.w;
    out.position = clip;
    out.uv = st;
    let c = s.c.x;
    out.color = vec4<u32>(c & 0xffu, (c >> 8u) & 0xffu, (c >> 16u) & 0xffu, c >> 24u);
    out.tag = s.c.y;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

@fragment
fn fragment(in: ParticleVertexOutput) -> @location(0) vec4<f32> {
    let layer = i32(in.tag & 0xffffu);
    let additive = (in.tag >> 16u) != 0u;
    let t = textureSample(tex, tex_sampler, in.uv, layer);
    // Texel in GS units: RGB 0..255, A 0..0x80.
    let ct = round(t.rgb * 255.0);
    let at = round(t.a * 255.0);
    let cv = vec3<f32>(in.color.rgb);
    let av = f32(in.color.a);
    let a = min(floor(at * av / 128.0), 255.0);
    let draw = params.misc.x;
    let aref = params.misc.y;
    if ((draw < 0.5 && a < aref) || (draw >= 0.5 && a >= aref)) { discard; }
    let cs = min(floor(ct * cv / 128.0), vec3<f32>(255.0));
    if (additive) {
        // ALPHA_1 0x48 on display bytes: Cd' = min(Cd + (Cs·As >> 7), 255). Cd = the snapshot under this pixel
        // (the zero fallback before the first snapshot: then this is lin(Cs·As), the plain linear add).
        let bg = textureLoad(view_transmission_texture, vec2<i32>(floor(in.position.xy)), 0).rgb;
        let cd = round(linear_to_srgb(clamp(bg, vec3<f32>(0.0), vec3<f32>(1.0))) * 255.0);
        let res = min(cd + floor(cs * a / 128.0), vec3<f32>(255.0));
        return vec4<f32>(max(srgb_to_linear(res / 255.0) - srgb_to_linear(cd / 255.0), vec3<f32>(0.0)), 0.0);
    }
    let as_ = a / 128.0;
    let rgb = srgb_to_linear(cs / 255.0) * as_;
    return vec4<f32>(rgb, min(as_, 1.0));
}
