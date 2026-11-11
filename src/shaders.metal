#include <metal_stdlib>
using namespace metal;

// ───────────────────────── shared layouts (mirrored in gfx.rs) ─────────────────────────
struct Vertex { packed_float3 pos; packed_float3 nrm; packed_float2 uv; uint color; uint mat; };
struct Inst { float4x4 model; float4 tint; float4 misc; };   // misc: x emissive boost, y viewmodel flag
struct Light { float4 pos; float4 color; };                 // pos.w = range
struct Globals {
    float4x4 viewProj;       // jittered
    float4x4 invViewProj;
    float4x4 prevViewProj;   // previous frame, unjittered
    float4x4 vmViewProj;     // the weapon in your hands has its own lens
    float4x4 invVmViewProj;
    float4x4 curViewProj;    // this frame, unjittered
    float4 camPos;           // xyz, time
    float4 prevCam;          // xyz, near
    float4 sunDir;           // xyz, intensity
    float4 sunColor;         // rgb, night amount
    float4 skyTop;           // rgb, cloud cover
    float4 skyMid;
    float4 skyBot;
    float4 fog;              // rgb, density
    float4 ambient;          // rgb, water height
    float4 water;            // rgb, underwater flag
    float4 res;              // w, h, 1/w, 1/h
    float4 misc;             // frame, rt scale, exposure, bloom
    float4 post;             // saturation, rush, hurt, chromatic aberration
    float4 post2;            // vignette, history valid, taa blend, low health
    float4 counts;           // lights, spheres
    Light lights[32];
    float4 spheres[96];      // pairs: (centre, radius), (tint rgb)
};
struct Ray { packed_float3 origin; float minDistance; packed_float3 direction; float maxDistance; };

constexpr sampler sRep(filter::linear, mip_filter::linear, address::repeat, max_anisotropy(8));
constexpr sampler sLin(filter::linear, mip_filter::linear, address::clamp_to_edge);
constexpr sampler sNear(filter::nearest, address::clamp_to_edge);

// ───────────────────────── helpers ─────────────────────────
static float hash21(float2 p) { p = fract(p * float2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
static float vnoise(float2 p) {
    float2 i = floor(p), f = fract(p); f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i + float2(1, 0)), f.x), mix(hash21(i + float2(0, 1)), hash21(i + float2(1, 1)), f.x), f.y);
}
static float fbm(float2 p) { float s = 0.0, a = 0.5; for (int i = 0; i < 4; i++) { s += a * vnoise(p); p = p * 2.03 + 17.1; a *= 0.5; } return s / 0.9375; }
static float3 clean(float3 c) { return select(c, float3(0.0), isnan(c) || isinf(c)); }
static float luma(float3 c) { return dot(c, float3(0.2126, 0.7152, 0.0722)); }
static float3 srgb2lin(float3 c) { return c * (c * (c * 0.305306011 + 0.682171111) + 0.012522878); }
static float2 ndcOf(float2 uv) { return float2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0); }
static float2 uvOf(float2 ndc) { return float2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5); }
// Depth is reversed (1 near, 0 far); the world lives in [0, 0.9] and the viewmodel in (0.9, 1].
static float3 worldPos(float2 uv, float d, constant Globals& G) {
    float2 ndc = ndcOf(uv);
    float4 p = d > 0.9 ? G.invVmViewProj * float4(ndc, (d - 0.9) / 0.1, 1.0) : G.invViewProj * float4(ndc, d / 0.9, 1.0);
    return p.xyz / p.w;
}
static float2 prevUv(float2 uv, float3 wp, constant Globals& G) { float4 a = G.prevViewProj * float4(wp, 1.0), b = G.curViewProj * float4(wp, 1.0); return uv + uvOf(a.xy / a.w) - uvOf(b.xy / b.w); }
static float3 rayDir(float2 uv, constant Globals& G) { float4 p = G.invViewProj * float4(ndcOf(uv), 0.5, 1.0); return normalize(p.xyz / p.w - G.camPos.xyz); }

// Sky: gradient, sun glow and disc, a moving cloud deck, stars after dark.
static float3 sky(float3 d, constant Globals& G, bool sunDisc) {
    float h = d.y;
    float3 c = h > 0.0 ? mix(G.skyMid.rgb, G.skyTop.rgb, pow(saturate(h), 0.45)) : mix(G.skyMid.rgb, G.skyBot.rgb, saturate(-h * 4.0));
    float sd = saturate(dot(d, G.sunDir.xyz));
    float3 sun = G.sunColor.rgb * G.sunDir.w;
    c += sun * (pow(sd, 8.0) * 0.035 + pow(sd, 64.0) * 0.12);
    float night = G.sunColor.w;
    if (night > 0.6 && h > 0.0) { float2 sp = d.xz / (h + 0.4) * 180.0; float st = hash21(floor(sp)); c += smoothstep(0.992, 1.0, st) * (night - 0.6) * 2.5 * float3(0.9, 0.95, 1.0) * saturate(h * 4.0); }
    float cl = 0.0;
    if (h > 0.01) {
        float2 uv = d.xz / (h + 0.12) * 0.55 + G.camPos.w * float2(0.006, 0.002);
        float n = fbm(uv * 1.4), cover = G.skyTop.w;
        cl = smoothstep(1.0 - cover, 1.0 - cover + 0.32, n + 0.12) * saturate(h * 7.0);
        float lit = saturate(0.55 + (n - fbm(uv * 1.4 + G.sunDir.xz * 0.12)) * 3.0);
        float3 cc = mix(G.ambient.rgb * 0.75 + G.skyMid.rgb * 0.25, G.ambient.rgb * 0.9 + sun * 0.22, lit);
        c = mix(c, cc, cl * 0.92);
    }
    if (sunDisc) c += sun * 40.0 * smoothstep(0.99955, 0.9998, sd) * (1.0 - cl);
    return c;
}
// Height fog with the sun scattering through it; much denser under water.
static float3 applyFog(float3 col, float3 ro, float3 rd, float dist, constant Globals& G) {
    if (G.water.w > 0.5) { float f = 1.0 - exp(-dist * 0.16); return mix(col, G.water.rgb * (G.ambient.rgb * 2.0 + 0.1), f); }
    float b = 0.045, ry = abs(rd.y) < 1e-4 ? 1e-4 : rd.y;
    float amt = G.fog.w * exp(-max(ro.y, 0.0) * b) * (1.0 - exp(-dist * ry * b)) / (ry * b);
    float f = 1.0 - exp(-max(amt, 0.0));
    float sd = saturate(dot(rd, G.sunDir.xyz));
    float3 fc = G.fog.rgb * (G.ambient.rgb * 1.3 + G.sunColor.rgb * G.sunDir.w * (0.06 + pow(sd, 6.0) * 0.35));
    return mix(col, fc, f);
}
static float D_GGX(float NoH, float a) { float a2 = a * a, d = NoH * NoH * (a2 - 1.0) + 1.0; return a2 / (M_PI_F * d * d + 1e-7); }
static float V_Smith(float NoV, float NoL, float a) { float a2 = a * a; return 0.5 / (NoL * sqrt(NoV * NoV * (1.0 - a2) + a2) + NoV * sqrt(NoL * NoL * (1.0 - a2) + a2) + 1e-5); }
static float3 F_Schlick(float3 f0, float c) { float f = pow(1.0 - c, 5.0); return f0 + (1.0 - f0) * f; }
static float3 shade(float3 N, float3 V, float3 L, float3 diff, float3 f0, float rough) {
    float NoL = saturate(dot(N, L)); if (NoL <= 0.0) return float3(0.0);
    float3 H = normalize(V + L); float NoV = max(dot(N, V), 1e-3), a = rough * rough;
    float3 spec = D_GGX(saturate(dot(N, H)), a) * V_Smith(NoV, NoL, a) * F_Schlick(f0, saturate(dot(V, H)));
    return (diff / M_PI_F + spec) * NoL;
}
static float sphShadow(float3 ro, float3 rd, float4 s, float k) {
    float3 oc = ro - s.xyz; float b = dot(oc, rd), c = dot(oc, oc) - s.w * s.w, h = b * b - c;
    float d = sqrt(max(0.0, s.w * s.w - h)) - s.w, t = -b - sqrt(max(h, 0.0));
    return t < 0.0 ? 1.0 : smoothstep(0.0, 1.0, 2.5 * k * d / t);
}
static float sphOcc(float3 p, float3 n, float4 s) { float3 di = s.xyz - p; float l = length(di); float nl = dot(n, di / l), h = l / s.w; return 1.0 - saturate(nl) / (h * h) * 0.75; }

// ───────────────────────── fullscreen triangle ─────────────────────────
struct FsIn { float4 pos [[position]]; float2 uv; };
vertex FsIn fs_vs(uint vid [[vertex_id]]) {
    float2 p = float2((vid << 1) & 2, vid & 2);
    FsIn o; o.pos = float4(p * 2.0 - 1.0, 0.0, 1.0); o.uv = float2(p.x, 1.0 - p.y); return o;
}

// ───────────────────────── G-buffer ─────────────────────────
struct GOut { float4 pos [[position]]; float3 wpos; float3 nrm; float2 uv; float4 color; float4 mat [[flat]]; float2 emis [[flat]]; };
vertex GOut g_vs(uint vid [[vertex_id]], uint iid [[instance_id]], const device Vertex* verts [[buffer(0)]], const device Inst* insts [[buffer(1)]], constant Globals& G [[buffer(2)]]) {
    Vertex v = verts[vid]; Inst I = insts[iid];
    float4 wp = I.model * float4(float3(v.pos), 1.0);
    GOut o; o.wpos = wp.xyz;
    if (I.misc.y > 0.5) { o.pos = G.vmViewProj * wp; o.pos.z = 0.9 * o.pos.w + 0.1 * o.pos.z; } else { o.pos = G.viewProj * wp; o.pos.z *= 0.9; }
    float3x3 m = float3x3(I.model[0].xyz, I.model[1].xyz, I.model[2].xyz);
    o.nrm = float3x3(cross(m[1], m[2]), cross(m[2], m[0]), cross(m[0], m[1])) * float3(v.nrm);
    o.uv = float2(v.uv);
    float4 c = unpack_unorm4x8_to_float(v.color);
    o.color = float4(srgb2lin(c.rgb) * I.tint.rgb, c.a);
    o.mat = unpack_unorm4x8_to_float(v.mat);
    o.emis = float2(o.mat.w * 6.0 + I.misc.x, (1.0 - c.a) * G.sunColor.w);
    return o;
}
struct GFrag { float4 albedo [[color(0)]]; float4 normal [[color(1)]]; float4 emissive [[color(2)]]; };
fragment GFrag g_fs(GOut in [[stage_in]], constant Globals& G [[buffer(2)]], texture2d_array<float> texA [[texture(0)]], texture2d_array<float> texN [[texture(1)]]) {
    uint layer = uint(in.mat.x * 255.0 + 0.5);
    float4 a = texA.sample(sRep, in.uv, layer), nr = texN.sample(sRep, in.uv, layer);
    float3 N = normalize(in.nrm + float3(0.0, 1e-6, 0.0));
    float3 dp1 = dfdx(in.wpos), dp2 = dfdy(in.wpos); float2 du1 = dfdx(in.uv), du2 = dfdy(in.uv);
    float3 p2 = cross(dp2, N), p1 = cross(N, dp1);
    float3 T = p2 * du1.x + p1 * du2.x, B = p2 * du1.y + p1 * du2.y;
    float inv = rsqrt(max(dot(T, T), dot(B, B)) + 1e-20);
    float2 t = nr.xy * 2.0 - 1.0;
    if (layer == 4u || layer == 7u || layer == 14u) {   // big open surfaces: fold in a second copy, turned and rescaled, so the pattern never lines up
        float2 uv2 = float2(0.8 * in.uv.x - 0.6 * in.uv.y, 0.6 * in.uv.x + 0.8 * in.uv.y) * 0.57 + float2(0.37, 0.11);
        float m = smoothstep(0.3, 0.7, vnoise(in.wpos.xz * 0.06 + 3.7) * 0.65 + vnoise(in.wpos.xz * 0.19 + 11.0) * 0.35);
        float4 a2 = texA.sample(sRep, uv2, layer), n2 = texN.sample(sRep, uv2, layer);
        float2 t2 = n2.xy * 2.0 - 1.0; t2 = float2(0.8 * t2.x + 0.6 * t2.y, -0.6 * t2.x + 0.8 * t2.y);
        a = mix(a, a2, m); t = mix(t, t2, m); nr.z = mix(nr.z, n2.z, m);
    }
    float3 n = layer == 0 ? N : normalize(T * inv * t.x + B * inv * t.y + N * sqrt(saturate(1.0 - dot(t, t))));
    float macro = 0.86 + 0.28 * vnoise(in.wpos.xz * 0.11 + in.wpos.y * 0.17);   // break up tiling
    float near = saturate(1.0 - distance(in.wpos, G.camPos.xyz) / 22.0);
    macro *= 1.0 + (vnoise(in.wpos.xz * 41.0 + in.wpos.y * 33.0) - 0.5) * 0.16 * near;   // grain up close, where the texture runs out of pixels
    float3 alb = a.rgb * in.color.rgb * (layer == 0 ? 1.0 : macro);
    GFrag o;
    o.albedo = float4(alb, in.mat.z);
    o.normal = float4(n, in.mat.y > 0.004 ? in.mat.y : nr.z);
    o.emissive = float4(alb * in.emis.x + float3(1.0, 0.7, 0.36) * in.emis.y * 3.2 + alb * a.a * G.sunColor.w * 2.6, 1.0);
    return o;
}
// Decals blend into albedo, and either roughen (paint) or wet (puddles, goo) the surface.
struct DOut { float4 pos [[position]]; float3 nrm; float2 uv; float4 color; };
vertex DOut d_vs(uint vid [[vertex_id]], const device Vertex* verts [[buffer(0)]], constant Globals& G [[buffer(2)]]) {
    Vertex v = verts[vid]; DOut o;
    o.pos = G.viewProj * float4(float3(v.pos), 1.0); o.pos.z *= 0.9;
    o.nrm = float3(v.nrm); o.uv = float2(v.uv);
    float4 c = unpack_unorm4x8_to_float(v.color); o.color = float4(srgb2lin(c.rgb), c.a);
    return o;
}
struct DFrag { float4 albedo [[color(0)]]; float4 normal [[color(1)]]; };
fragment DFrag d_fs(DOut in [[stage_in]], texture2d<float> atlas [[texture(0)]]) {
    float4 t = atlas.sample(sLin, in.uv) * in.color.a;
    DFrag o; o.albedo = float4(t.rgb * in.color.rgb, t.a); o.normal = float4(normalize(in.nrm), t.a); return o;
}

// ───────────────────────── MPS ray tracing: generate and resolve ─────────────────────────
static float2 rnd2(uint2 p, uint frame, uint salt) {
    uint n = p.x * 1973u + p.y * 9277u + frame * 26699u + salt * 7919u; n = (n << 13u) ^ n; uint a = n * (n * n * 15731u + 789221u) + 1376312589u;
    uint b = a * 1664525u + 1013904223u; return float2(float(a & 0xffffffu), float(b & 0xffffffu)) / 16777216.0;
}
static void basis(float3 n, thread float3& t, thread float3& b) { float3 u = abs(n.y) > 0.99 ? float3(1, 0, 0) : float3(0, 1, 0); t = normalize(cross(u, n)); b = cross(n, t); }
static uint2 fullPx(uint2 gid, constant Globals& G) { uint s = uint(G.misc.y), f = uint(G.misc.x); return s == 1 ? gid : gid * 2 + uint2(f & 1u, (f >> 1u) & 1u); }
kernel void rt_gen(uint2 gid [[thread_position_in_grid]], constant Globals& G [[buffer(0)]], device Ray* rays [[buffer(1)]], depth2d<float, access::read> gD [[texture(0)]], texture2d<float, access::read> gN [[texture(1)]]) {
    uint w = uint(G.res.x) / uint(G.misc.y), h = uint(G.res.y) / uint(G.misc.y);
    if (gid.x >= w || gid.y >= h) return;
    uint idx = gid.y * w + gid.x, cnt = w * h;
    uint2 fp = min(fullPx(gid, G), uint2(uint(G.res.x) - 1, uint(G.res.y) - 1));
    float d = gD.read(fp);
    Ray sr, ar; sr.minDistance = 0.0; ar.minDistance = 0.0; sr.maxDistance = -1.0; ar.maxDistance = -1.0; sr.origin = float3(0); ar.origin = float3(0); sr.direction = float3(0, 1, 0); ar.direction = float3(0, 1, 0);
    if (d > 0.0) {
        float2 uv = (float2(fp) + 0.5) * G.res.zw;
        float3 wp = worldPos(uv, d, G), N = normalize(gN.read(fp).xyz);
        float dist = length(wp - G.camPos.xyz);
        float3 o = wp + N * (0.03 + dist * 0.002), t, b;
        float2 x1 = rnd2(gid, uint(G.misc.x), 1u), x2 = rnd2(gid, uint(G.misc.x), 2u);
        basis(G.sunDir.xyz, t, b);
        float r = sqrt(x1.x) * 0.022, ph = x1.y * 6.2831853;   // the sun has a size: soft penumbrae
        float3 L = normalize(G.sunDir.xyz + (t * cos(ph) + b * sin(ph)) * r);
        sr.origin = o; sr.direction = L; sr.maxDistance = dot(N, L) > 0.0 ? 600.0 : -1.0;
        basis(N, t, b);
        float r2 = sqrt(x2.x), p2 = x2.y * 6.2831853;
        ar.origin = o; ar.direction = normalize(t * (r2 * cos(p2)) + b * (r2 * sin(p2)) + N * sqrt(max(0.0, 1.0 - x2.x))); ar.maxDistance = 4.0;
    }
    rays[idx] = sr; rays[idx + cnt] = ar;
}
// Turn hit distances into shadow + ambient occlusion and fold them into the history.
kernel void rt_resolve(uint2 gid [[thread_position_in_grid]], constant Globals& G [[buffer(0)]], const device float* hits [[buffer(1)]], depth2d<float, access::read> gD [[texture(0)]], texture2d<float> hist [[texture(1)]], texture2d<float, access::write> outT [[texture(2)]]) {
    uint w = uint(G.res.x) / uint(G.misc.y), h = uint(G.res.y) / uint(G.misc.y);
    if (gid.x >= w || gid.y >= h) return;
    uint idx = gid.y * w + gid.x, cnt = w * h;
    uint2 fp = min(fullPx(gid, G), uint2(uint(G.res.x) - 1, uint(G.res.y) - 1));
    float d = gD.read(fp);
    if (d <= 0.0) { outT.write(float4(1.0, 1.0, 4000.0, 1.0), gid); return; }
    float2 uv = (float2(fp) + 0.5) * G.res.zw;
    float3 wp = worldPos(uv, d, G); float dist = length(wp - G.camPos.xyz);
    float hs = hits[idx], ha = hits[idx + cnt];
    float shadow = hs >= 0.0 ? 0.0 : 1.0;
    float ao = ha >= 0.0 ? saturate(ha / 4.0) * 0.75 : 1.0;
    float2 puv = d > 0.9 ? uv : prevUv(uv, wp, G);
    float n = 1.0; float2 acc = float2(shadow, ao);
    if (G.post2.y > 0.5 && all(puv > 0.0) && all(puv < 1.0)) {
        float4 hv = hist.sample(sLin, puv);
        float pd = d > 0.9 ? dist : length(wp - G.prevCam.xyz);
        if (abs(hv.z - pd) < 0.06 * pd + 0.05) { n = min(hv.w + 1.0, 14.0); acc = mix(hv.xy, acc, 1.0 / n); }
    }
    outT.write(float4(acc, dist, n), gid);
}
// Edge-aware blur: neighbours only count if they sit at a similar distance.
kernel void rt_blur(uint2 gid [[thread_position_in_grid]], constant float4& P [[buffer(0)]], texture2d<float, access::read> src [[texture(0)]], texture2d<float, access::write> dst [[texture(1)]]) {
    int w = int(src.get_width()), h = int(src.get_height());
    if (int(gid.x) >= w || int(gid.y) >= h) return;
    float4 c = src.read(gid); float2 sum = c.xy * 1.0; float ws = 1.0; int st = int(P.x);
    float spread = clamp(6.0 / c.w, 0.6, 3.0);   // lean harder on neighbours while the history is thin
    for (int y = -2; y <= 2; y++) for (int x = -2; x <= 2; x++) {
        if (x == 0 && y == 0) continue;
        int2 q = clamp(int2(gid) + int2(x, y) * st, int2(0), int2(w - 1, h - 1));
        float4 s = src.read(uint2(q));
        float wt = exp(-float(x * x + y * y) / (2.0 * spread)) * exp(-abs(s.z - c.z) / (0.03 * c.z + 0.02));
        sum += s.xy * wt; ws += wt;
    }
    dst.write(float4(sum / ws, c.z, c.w), gid);
}

// ───────────────────────── lighting ─────────────────────────
struct LOut { float4 color [[color(0)]]; float lin [[color(1)]]; };
fragment LOut light_fs(FsIn in [[stage_in]], constant Globals& G [[buffer(0)]], texture2d<float> gA [[texture(0)]], texture2d<float> gN [[texture(1)]], texture2d<float> gE [[texture(2)]], depth2d<float> gD [[texture(3)]], texture2d<float> rt [[texture(4)]]) {
    uint2 px = uint2(in.pos.xy);
    float d = gD.read(px);
    float3 cam = G.camPos.xyz, rd = rayDir(in.uv, G);
    LOut o;
    if (d <= 0.0) { o.color = float4(applyFog(sky(rd, G, true), cam, rd, G.water.w > 0.5 ? 60.0 : 0.0, G), 1.0); o.lin = 4000.0; return o; }
    float3 wp = worldPos(in.uv, d, G);
    float dist = length(wp - cam); rd = (wp - cam) / dist;
    float4 a = gA.read(px), nr = gN.read(px);
    float3 N = normalize(nr.xyz), V = -rd, albedo = a.rgb;
    float metal = a.a, rough = clamp(nr.w, 0.035, 1.0);
    // ray-traced shadow + AO, upsampled with depth-aware weights
    float2 sa = float2(1.0);
    {
        float2 rs = G.res.xy / G.misc.y; float2 p = in.uv * rs - 0.5; float2 b = floor(p), f = p - b; float ws = 0.0; float2 acc = float2(0.0);
        for (int j = 0; j < 2; j++) for (int i = 0; i < 2; i++) {
            float4 s = rt.read(uint2(clamp(b + float2(i, j), float2(0.0), rs - 1.0)));
            float wt = (i == 0 ? 1.0 - f.x : f.x) * (j == 0 ? 1.0 - f.y : f.y) * exp(-abs(s.z - dist) / (0.03 * dist + 0.02)) + 1e-4;
            acc += s.xy * wt; ws += wt;
        }
        sa = acc / ws;
    }
    float shadow = sa.x, ao = sa.y;
    bool vm = d > 0.9;
    if (vm) { shadow = G.post2.w; ao = 1.0; }
    float3 Ls = G.sunDir.xyz, tint = float3(1.0);
    int ns = int(G.counts.y);
    for (int i = 0; i < ns; i++) {     // gummies cast soft, tinted shadows: light passes through them
        float4 s = G.spheres[2 * i];
        float sh = sphShadow(wp + N * 0.03, Ls, s, 9.0);
        float core = 1.0 - sh; core *= core * core;
        tint *= mix(G.spheres[2 * i + 1].rgb * (0.42 + 1.0 * core), float3(1.0), sh);   // dim at the rim, bright in the middle, like light focused through candy
        ao *= sphOcc(wp, N, s);
    }
    float3 f0 = mix(float3(0.04), albedo, metal), diff = albedo * (1.0 - metal);
    if (vm) { diff = diff * 1.35 + 0.006; f0 = max(f0, float3(0.05)); }
    float3 sun = G.sunColor.rgb * G.sunDir.w;
    float3 col = shade(N, V, Ls, diff, f0, rough) * sun * shadow * tint;
    float3 amb = mix(float3(luma(G.ambient.rgb)), G.ambient.rgb, 0.72);
    float3 bounce = G.skyBot.rgb * 0.3 + sun * float3(0.085, 0.075, 0.062) * saturate(Ls.y);
    float3 irr = mix(bounce, amb, N.y * 0.5 + 0.5);
    col += diff * irr * ao;
    float3 R = reflect(rd, N);
    float3 env = mix(sky(R, G, false), irr, saturate(rough * 1.15));
    float3 Fe = f0 + (max(float3(1.0 - rough), f0) - f0) * pow(1.0 - saturate(dot(N, V)), 5.0);
    col += env * Fe * ao * mix(shadow * 0.5 + 0.5, 1.0, rough);
    int nl = int(G.counts.x);
    for (int i = 0; i < nl; i++) {
        float3 lv = G.lights[i].pos.xyz - wp; float l2 = dot(lv, lv), r = G.lights[i].pos.w;
        if (l2 > r * r) continue;
        float l = sqrt(l2), fall = saturate(1.0 - l2 / (r * r)); fall = fall * fall / (1.0 + l2 * 0.03);
        col += shade(N, V, lv / l, diff, f0, rough) * G.lights[i].color.rgb * fall * mix(ao, 1.0, 0.4);
    }
    if (vm) {   // hero lighting on the weapon: a key over the right shoulder and a cool rim, scaled by how bright the surroundings are
        float3 fw = rayDir(float2(0.5), G), rgt = normalize(rayDir(float2(1.0, 0.5), G) - rayDir(float2(0.0, 0.5), G)), up = cross(rgt, fw);
        float3 lvl = max(float3(luma(amb)) * float3(1.04, 1.0, 0.95) + sun * (0.05 + 0.13 * shadow), float3(0.16));
        col += shade(N, V, normalize(rgt * 0.45 + up * 0.85 - fw * 0.3), diff, f0, rough) * lvl * 2.4;
        col += shade(N, V, normalize(-rgt * 0.9 + up * 0.25 + fw * 0.35), diff, f0, rough) * (luma(G.skyTop.rgb) * 0.5 + luma(amb) * 0.7) * float3(0.9, 0.97, 1.1) * 1.6;
        // worn edges and grimy creases, read from how the normals bend across neighbouring pixels
        int2 ip = int2(px), mxp = int2(G.res.xy) - 1;
        float3 nb[4]; const int2 off[4] = { int2(-2, 0), int2(2, 0), int2(0, -2), int2(0, 2) };
        for (int i = 0; i < 4; i++) { uint2 q = uint2(clamp(ip + off[i], int2(0), mxp)); nb[i] = gD.read(q) > 0.9 ? normalize(gN.read(q).xyz) : N; }
        float curv = dot(nb[1] - nb[0], rgt) + dot(nb[2] - nb[3], up);
        col = col * (1.0 - saturate(-curv * 1.3) * 0.5) + saturate(curv * 1.3) * lvl * (0.07 + 0.6 * f0);
    }
    col += gE.read(px).rgb;
    col = applyFog(col, cam, rd, dist, G);
    o.color = float4(clean(col), 1.0); o.lin = dist; return o;
}

// ───────────────────────── water ─────────────────────────
fragment float4 water_fs(FsIn in [[stage_in]], constant Globals& G [[buffer(0)]], texture2d<float> lin [[texture(0)]]) {
    float3 cam = G.camPos.xyz, rd = rayDir(in.uv, G);
    float wy = G.ambient.w;
    if (abs(rd.y) < 1e-4) discard_fragment();
    float t = (wy - cam.y) / rd.y;
    float sd = lin.read(uint2(in.pos.xy)).r;
    if (t <= 0.0 || t > sd) discard_fragment();
    float3 p = cam + rd * t;
    float2 q = p.xz; float tm = G.camPos.w;
    float e = 0.12, fade = 1.0 / (1.0 + t * 0.02);
    float h0 = fbm(q * 0.35 + tm * float2(0.21, 0.13)) + 0.5 * fbm(q * 1.3 - tm * float2(0.3, 0.17));
    float hx = fbm((q + float2(e, 0)) * 0.35 + tm * float2(0.21, 0.13)) + 0.5 * fbm((q + float2(e, 0)) * 1.3 - tm * float2(0.3, 0.17));
    float hz = fbm((q + float2(0, e)) * 0.35 + tm * float2(0.21, 0.13)) + 0.5 * fbm((q + float2(0, e)) * 1.3 - tm * float2(0.3, 0.17));
    float3 n = normalize(float3(-(hx - h0) / e * 0.55 * fade, 1.0, -(hz - h0) / e * 0.55 * fade));
    bool below = cam.y < wy; if (below) n = -n;
    float fres = 0.02 + 0.98 * pow(1.0 - saturate(dot(n, -rd)), 5.0);
    float3 sun = G.sunColor.rgb * G.sunDir.w;
    float3 deep = G.water.rgb * (G.ambient.rgb * 1.4 + sun * 0.12 * saturate(G.sunDir.y));
    float3 col; float alpha;
    if (below) { col = mix(deep * 3.0 + sky(float3(rd.x, abs(rd.y), rd.z), G, true) * 0.5, deep, fres); alpha = 0.85; }
    else { col = mix(deep, sky(reflect(rd, n), G, true), fres); alpha = saturate(fres + 1.0 - exp(-(sd - t) * 0.55)); col = applyFog(col, cam, rd, t, G); }
    return float4(col, alpha);
}

// ───────────────────────── gummies: ray-marched jelly ─────────────────────────
struct GummyU { float4x4 invModel; float4x4 model; float4 color; float4 params; float4 eyes[3]; float4 prims[32]; };   // params: prim count, sun visibility, glow, scale
struct GumIn { float4 pos [[position]]; float3 wpos; };
constant float3 GB0 = float3(-1.1, -0.06, -0.72);
constant float3 GB1 = float3(1.1, 2.42, 1.0);
vertex GumIn gum_vs(uint vid [[vertex_id]], constant Globals& G [[buffer(0)]], constant GummyU& U [[buffer(1)]]) {
    const uint8_t idx[36] = { 1,3,7, 1,7,5,  0,4,6, 0,6,2,  2,6,7, 2,7,3,  0,1,5, 0,5,4,  4,5,7, 4,7,6,  0,2,3, 0,3,1 };
    uint c = idx[vid];
    float3 lp = mix(GB0, GB1, float3(c & 1, (c >> 1) & 1, (c >> 2) & 1));
    float4 wp = U.model * float4(lp, 1.0);
    GumIn o; o.pos = G.viewProj * wp; o.pos.z *= 0.9; o.wpos = wp.xyz; return o;
}
static float sdEll(float3 p, float3 r) { float k0 = length(p / r), k1 = length(p / (r * r)); return k0 * (k0 - 1.0) / max(k1, 1e-5); }
static float sdCap(float3 p, float3 a, float3 b, float r) { float3 pa = p - a, ba = b - a; float h = saturate(dot(pa, ba) / dot(ba, ba)); return length(pa - ba * h) - r; }
static float smin(float a, float b, float k) { float h = max(k - abs(a - b), 0.0) / k; return min(a, b) - h * h * k * 0.25; }
static float gmap(float3 p, constant GummyU& U) {
    float d = 1e9; int n = int(U.params.x);
    for (int i = 0; i < n; i++) { float4 a = U.prims[2 * i], b = U.prims[2 * i + 1]; float di = a.w > 0.5 ? sdCap(p, a.xyz, b.xyz, b.w) : sdEll(p - a.xyz, b.xyz); d = smin(d, di, 0.1); }
    return d;
}
struct GumOut { float4 color [[color(0)]]; float lin [[color(1)]]; float depth [[depth(any)]]; };
fragment GumOut gum_fs(GumIn in [[stage_in]], constant Globals& G [[buffer(0)]], constant GummyU& U [[buffer(1)]], texture2d<float> scene [[texture(0)]]) {
    float3 cam = G.camPos.xyz, rdw = normalize(in.wpos - cam);
    float3 ro = (U.invModel * float4(cam, 1.0)).xyz, rd = (U.invModel * float4(rdw, 0.0)).xyz;
    float sc = length(rd); rd /= sc;
    float3 i0 = (GB0 - ro) / rd, i1 = (GB1 - ro) / rd, lo = min(i0, i1), hi = max(i0, i1);
    float tn = max(max(lo.x, lo.y), max(lo.z, 0.0)), tf = min(hi.x, min(hi.y, hi.z));
    if (tn > tf) discard_fragment();
    float t = tn; bool hit = false;
    for (int i = 0; i < 64; i++) { float d = gmap(ro + rd * t, U); if (d < 0.0016) { hit = true; break; } t += d; if (t > tf) break; }
    if (!hit) discard_fragment();
    float3 p = ro + rd * t;
    const float2 k = float2(1.0, -1.0); const float e = 0.0025;
    float3 nl = normalize(k.xyy * gmap(p + k.xyy * e, U) + k.yyx * gmap(p + k.yyx * e, U) + k.yxy * gmap(p + k.yxy * e, U) + k.xxx * gmap(p + k.xxx * e, U));
    float3 wp = (U.model * float4(p, 1.0)).xyz;
    float3 N = normalize((float4(nl, 0.0) * U.invModel).xyz);    // inverse-transpose
    float4 clip = G.viewProj * float4(wp, 1.0);
    GumOut o; o.depth = clip.z / clip.w * 0.9; o.lin = length(wp - cam);

    float scale = U.params.w;
    float3 V = -rdw, L = G.sunDir.xyz, base = U.color.rgb;
    float NoV = saturate(dot(N, V)), NoL = dot(N, L);
    // Jelly is mostly clear and only faintly cloudy: light goes in and through far more than it bounces back.
    // So the look comes from depth: thin parts stay pale and let the world show, thick parts go deep and dark.
    float3 ext = -log(clamp(base, 0.04, 0.96)) + 0.8;                     // what a unit of jelly soaks up per channel, plus the cloudiness
    float dens = scale;
    // the view ray bends on the way in; follow it to see how much jelly it has to cross
    float3 rin = refract(rd, nl, 1.0 / 1.38); if (dot(rin, rin) < 0.5) rin = rd;
    float ti = 0.03; for (int i = 0; i < 16; i++) { float d = gmap(p + rin * ti, U); if (d > 0.0) break; ti += max(-d, 0.02); }
    // sunlight scattered toward the eye from three depths along that ray, each dimmed by the jelly it crossed going in and coming out
    float3 ll = normalize((U.invModel * float4(L, 0.0)).xyz);
    float3 lit = float3(0.0), qm = p;
    const float fr[3] = { 0.15, 0.5, 0.9 };
    for (int s = 0; s < 3; s++) {
        float ts = ti * fr[s]; float3 q = p + rin * ts; if (s == 1) qm = q;
        float tl = 0.02; for (int i = 0; i < 6; i++) { float d = gmap(q + ll * tl, U); if (d > 0.0) break; tl += max(-d, 0.05); }
        lit += exp(-ext * (ts + tl) * dens);
    }
    lit /= 3.0;
    float3 sun = G.sunColor.rgb * G.sunDir.w * U.params.y;
    float fwd = pow(saturate(dot(rdw, L) * 0.5 + 0.5), 5.0);             // looking toward the sun through the body: it glows
    float wrap = saturate((NoL + 0.7) / 1.7);
    float strength = 1.0 - exp(-0.9 * dens * ti);                         // more jelly along the ray, more of it to light up
    float3 scat = mix(float3(1.0), base, 0.55) * sun * lit * (0.15 + 0.12 * wrap + 0.9 * fwd) * strength;
    float3 amb = mix(float3(luma(G.ambient.rgb)), G.ambient.rgb, 0.72);
    float3 irr = mix(G.skyBot.rgb * 0.35 + sun * 0.05, amb, N.y * 0.5 + 0.5);
    float3 deep = mix(base, base * base, saturate(ti * dens * 0.45));     // thick jelly runs deeper in colour
    float occ = saturate(0.4 + 0.6 * gmap(p + nl * 0.1, U) / 0.1);        // creases: neck, armpits, between the feet
    float cloud = 0.88 + 0.24 * vnoise(qm.xy * 4.3 + qm.z * 2.9);         // it is never perfectly even inside
    // point lights: muzzle flashes, blasts, street lamps
    float3 spec = float3(0.0), local = float3(0.0);
    int nlights = int(G.counts.x);
    for (int i = 0; i < nlights; i++) {
        float3 lv = G.lights[i].pos.xyz - wp; float l2 = dot(lv, lv), r = G.lights[i].pos.w;
        if (l2 > r * r) continue;
        float l = sqrt(l2), fall = saturate(1.0 - l2 / (r * r)); fall = fall * fall / (1.0 + l2 * 0.03);
        float3 Lp = lv / l, lc = G.lights[i].color.rgb * fall;
        local += lc * (0.07 + 0.2 * saturate((dot(N, Lp) + 0.7) / 1.7) + 0.2 * pow(saturate(-dot(N, Lp)), 2.0));
        spec += lc * D_GGX(saturate(dot(N, normalize(V + Lp))), 0.09) * 0.02 * saturate(dot(N, Lp));
    }
    // what is behind, bent through the body and smeared by it: sharper at the thin edges
    float3 we = (U.model * float4(p + rin * ti, 1.0)).xyz + normalize((U.model * float4(rin, 0.0)).xyz) * 0.5 * scale;
    float4 ce = G.viewProj * float4(we, 1.0);
    float2 suv = clamp(uvOf(ce.xy / ce.w), float2(0.002), float2(0.998));
    float br = clamp(ti * dens, 0.0, 1.6) * 0.017 * scale / max(o.lin * 0.14, 1.0);
    float ang = hash21(in.pos.xy + G.misc.x * 7.3) * 6.2831853;
    float2 asp = float2(G.res.y / G.res.x, 1.0);
    float3 behind = scene.sample(sLin, suv).rgb * 0.25;
    for (int i = 0; i < 6; i++) { float a = ang + float(i) * 1.0472; behind += scene.sample(sLin, clamp(suv + float2(cos(a), sin(a)) * asp * br * ((i & 1) == 0 ? 1.0 : 0.55), float2(0.002), float2(0.998))).rgb * 0.125; }
    float3 Tv = exp(-ext * ti * dens);
    float edge = pow(1.0 - NoV, 2.5);
    float3 body = (scat + (deep * irr * 0.65 + base * local) * strength * occ + base * irr * 0.12) * cloud + behind * Tv + base * (irr * 0.35 + sun * 0.05) * edge;
    // the skin: a soft waxy sheen with a small wet glint on top, not a mirror
    float3 R = reflect(rdw, N);
    float3 refl = mix(sky(R, G, false), mix(G.skyBot.rgb, mix(G.skyMid.rgb, G.skyTop.rgb, saturate(R.y)), saturate(R.y * 4.0 + 0.5)), 0.75) * mix(0.4, 0.75, U.params.y);
    float fres = 0.03 + 0.6 * pow(1.0 - NoV, 5.0);
    float3 H = normalize(V + L); float NoH = saturate(dot(N, H));
    spec += sun * (D_GGX(NoH, 0.1) * 0.016 + D_GGX(NoH, 0.3) * 0.03) * saturate(NoL);
    body += base * G.sunColor.w * (0.03 + 0.1 * edge);                    // after dark they keep a faint glow of their own, so they still read
    float3 col = mix(body, refl, fres) + spec + base * U.params.z + U.color.w * float3(2.4);
    for (int i = 0; i < 3; i++) if (length((p - U.eyes[i].xyz) / U.eyes[i].w) < 1.0) col = mix(float3(0.006), refl, fres * 0.6) + spec;
    col = applyFog(col, cam, rdw, o.lin, G);
    o.color = float4(col, 1.0); return o;
}

// ───────────────────────── sprites: sparks, smoke, tracers, beams ─────────────────────────
struct Sprite { packed_float3 pos; float size; float4 color; packed_float3 axis; float kind; };   // kind: 0 glow, 1 smoke, 2 streak along axis
struct SpOut { float4 pos [[position]]; float2 uv; float4 color [[flat]]; float kind [[flat]]; float dist; };
vertex SpOut sp_vs(uint vid [[vertex_id]], uint iid [[instance_id]], constant Globals& G [[buffer(0)]], const device Sprite* sprites [[buffer(1)]]) {
    Sprite s = sprites[iid];
    float2 c = float2(vid == 1 || vid == 2 || vid == 4 ? 1.0 : -1.0, vid == 2 || vid == 4 || vid == 5 ? 1.0 : -1.0);
    float3 cam = G.camPos.xyz, pos = float3(s.pos), toCam = normalize(cam - pos), wp;
    if (s.kind > 1.5) { float3 ax = float3(s.axis); float3 side = normalize(cross(ax, toCam)) * s.size; wp = pos + ax * (c.y * 0.5 + 0.5) + side * c.x; }
    else { float3 right = normalize(cross(float3(0, 1, 0), toCam)), up = cross(toCam, right); wp = pos + (right * c.x + up * c.y) * s.size; }
    SpOut o; o.pos = G.viewProj * float4(wp, 1.0); o.pos.z *= 0.9; o.uv = c; o.color = s.color; o.kind = s.kind; o.dist = length(wp - cam); return o;
}
fragment float4 sp_fs(SpOut in [[stage_in]], texture2d<float> lin [[texture(0)]]) {
    float soft = saturate((lin.read(uint2(in.pos.xy)).r - in.dist) * 2.5 + 0.2);
    if (in.kind > 1.5) { float a = (1.0 - abs(in.uv.x)); a *= a; float cap = smoothstep(1.0, 0.8, abs(in.uv.y)); return float4(in.color.rgb * a * cap * in.color.a * soft, 0.0); }
    float r = length(in.uv); if (r > 1.0) discard_fragment();
    if (in.kind > 0.5) { float a = (1.0 - r) * (1.0 - r) * in.color.a * soft; return float4(in.color.rgb * a, a); }
    float a = pow(1.0 - r, 2.2) * in.color.a * soft; return float4(in.color.rgb * a, 0.0);
}

// ───────────────────────── temporal anti-aliasing ─────────────────────────
fragment float4 taa_fs(FsIn in [[stage_in]], constant Globals& G [[buffer(0)]], texture2d<float> cur [[texture(0)]], texture2d<float> hist [[texture(1)]], texture2d<float> lin [[texture(2)]], depth2d<float> gD [[texture(3)]]) {
    int2 px = int2(in.pos.xy), mx = int2(G.res.xy) - 1;
    float3 c = clean(cur.read(uint2(px)).rgb), mn = c, mxc = c;
    for (int y = -1; y <= 1; y++) for (int x = -1; x <= 1; x++) { float3 s = clean(cur.read(uint2(clamp(px + int2(x, y), int2(0), mx))).rgb); mn = min(mn, s); mxc = max(mxc, s); }
    if (G.post2.y < 0.5) return float4(c, 1.0);
    float dist = lin.read(uint2(px)).r;
    float3 wp = G.camPos.xyz + rayDir(in.uv, G) * dist;
    float2 puv = gD.read(uint2(px)) > 0.9 ? in.uv : prevUv(in.uv, wp, G);
    if (any(puv < 0.0) || any(puv > 1.0)) return float4(c, 1.0);
    float3 h = clamp(hist.sample(sLin, puv).rgb, mn, mxc);
    float b = G.post2.z, wc = b / (1.0 + luma(c)), wh = (1.0 - b) / (1.0 + luma(h));
    return float4((c * wc + h * wh) / (wc + wh), 1.0);
}

// ───────────────────────── bloom, tone mapping, grade ─────────────────────────
fragment float4 down_fs(FsIn in [[stage_in]], constant float4& P [[buffer(0)]], texture2d<float> src [[texture(0)]]) {
    float2 t = 1.0 / float2(src.get_width(), src.get_height());
    float3 a = src.sample(sLin, in.uv + t * float2(-1, -1)).rgb, b = src.sample(sLin, in.uv + t * float2(1, -1)).rgb, c = src.sample(sLin, in.uv + t * float2(-1, 1)).rgb, d = src.sample(sLin, in.uv + t * float2(1, 1)).rgb;
    if (P.x > 0.5) {   // first step: keep only what is bright, and weigh down single hot pixels
        float wa = 1.0 / (1.0 + luma(a)), wb = 1.0 / (1.0 + luma(b)), wc = 1.0 / (1.0 + luma(c)), wd = 1.0 / (1.0 + luma(d));
        float3 m = (a * wa + b * wb + c * wc + d * wd) / (wa + wb + wc + wd);
        float l = luma(m); return float4(m * smoothstep(P.y, P.y * 2.2, l), 1.0);
    }
    return float4((a + b + c + d) * 0.25, 1.0);
}
static float3 aces(float3 c) {
    c *= 0.95;
    float3 v = float3(dot(c, float3(0.59719, 0.35458, 0.04823)), dot(c, float3(0.07600, 0.90834, 0.01566)), dot(c, float3(0.02840, 0.13383, 0.83777)));
    v = (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
    return saturate(float3(dot(v, float3(1.60475, -0.53108, -0.07367)), dot(v, float3(-0.10208, 1.10813, -0.00605)), dot(v, float3(-0.00327, -0.07276, 1.07602))));
}
fragment float4 final_fs(FsIn in [[stage_in]], constant Globals& G [[buffer(0)]], texture2d<float> hdr [[texture(0)]], texture2d<float> b0 [[texture(1)]], texture2d<float> b1 [[texture(2)]], texture2d<float> b2 [[texture(3)]], texture2d<float> b3 [[texture(4)]]) {
    float tm = G.camPos.w, wob = G.water.w;
    float2 uv = in.uv + wob * 0.004 * float2(sin(in.uv.y * 38.0 + tm * 3.0), cos(in.uv.x * 34.0 + tm * 2.6));
    float2 dv = uv - 0.5; float r2 = dot(dv, dv);
    float2 off = dv * (G.post.w + wob * 0.004) * (0.4 + r2 * 6.0);
    float3 c = float3(hdr.sample(sLin, uv + off).r, hdr.sample(sLin, uv).g, hdr.sample(sLin, uv - off).b);
    {
        float2 t1 = 1.0 / G.res.xy; float3 ctr = hdr.sample(sLin, uv).rgb;
        float3 nb = (hdr.sample(sLin, uv + float2(t1.x, 0.0)).rgb + hdr.sample(sLin, uv - float2(t1.x, 0.0)).rgb + hdr.sample(sLin, uv + float2(0.0, t1.y)).rgb + hdr.sample(sLin, uv - float2(0.0, t1.y)).rgb) * 0.25;
        c = max(c + (ctr - nb) * 0.75 / (1.0 + max(luma(ctr), luma(nb))), float3(0.0));
    }
    float3 bloom = b0.sample(sLin, uv).rgb * 0.5 + b1.sample(sLin, uv).rgb * 0.3 + b2.sample(sLin, uv).rgb * 0.2 + b3.sample(sLin, uv).rgb * 0.12;
    c += bloom * G.misc.w;
    c *= G.misc.z;
    c = mix(c, c * float3(1.18, 0.86, 1.3) + float3(0.02, 0.0, 0.035), G.post.y * (0.35 + r2 * 1.6));   // sugar rush
    c = mix(c, c * float3(0.6, 0.95, 1.0), wob * 0.6);
    c = aces(c);
    float l = luma(c);
    c = mix(float3(l), c, G.post.x);
    c = saturate((c - 0.5) * 1.06 + 0.5);
    c *= 1.0 - (G.post2.x + wob * 0.3) * smoothstep(0.12, 0.62, r2);
    c = mix(c, float3(0.95, 0.04, 0.12) * (0.3 + l), saturate(G.post.z) * 0.8 * smoothstep(0.1, 0.55, r2));
    c = pow(saturate(c), float3(1.0 / 2.2));
    c += (hash21(in.pos.xy + fract(tm) * 91.7) - 0.5) * (1.6 / 255.0);   // dither after encoding, so it never shows as grain in the dark
    return float4(saturate(c), 1.0);
}

// ───────────────────────── interface ─────────────────────────
struct UiVert { packed_float2 pos; packed_float2 uv; uint color; packed_float4 shape; };   // shape: half size xy, corner radius, mode (0 box, 1 text, 2 ring)
struct UiOut { float4 pos [[position]]; float2 uv; float4 color; float4 shape [[flat]]; };
vertex UiOut ui_vs(uint vid [[vertex_id]], const device UiVert* verts [[buffer(0)]], constant float4& screen [[buffer(1)]]) {
    UiVert v = verts[vid]; UiOut o;
    o.pos = float4(float2(v.pos) * screen.zw * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    o.uv = float2(v.uv); o.color = unpack_unorm4x8_to_float(v.color); o.shape = float4(v.shape); return o;
}
fragment float4 ui_fs(UiOut in [[stage_in]], texture2d<float> font [[texture(0)]]) {
    float4 c = in.color; c.rgb = c.rgb;
    if (in.shape.w > 0.5 && in.shape.w < 1.5) { c.a *= font.sample(sLin, in.uv).r; }
    else {
        float2 q = abs(in.uv) - in.shape.xy + in.shape.z;
        float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - in.shape.z;
        if (in.shape.w < -0.5) {   // five-pointed star of radius shape.x
            const float2 k1 = float2(0.809016994375, -0.587785252292), k2 = float2(-0.809016994375, -0.587785252292);
            float2 p = float2(abs(in.uv.x), -in.uv.y); float r = in.shape.x * 0.94;
            p -= 2.0 * max(dot(k1, p), 0.0) * k1; p -= 2.0 * max(dot(k2, p), 0.0) * k2;
            p.x = abs(p.x); p.y -= r;
            float2 ba = 0.47 * float2(-k1.y, k1.x) - float2(0.0, 1.0);
            float h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
            d = length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y) - in.shape.x * 0.05;
        }
        float aa = saturate(0.5 - d);
        if (in.shape.w > 1.5) aa *= saturate(d + (in.shape.w - 1.0) + 0.5);   // ring of the given thickness
        c.a *= aa;
    }
    return float4(c.rgb * c.a, c.a);
}
// plain copy with optional sharpening, used when the internal image is already at window size
fragment float4 copy_fs(FsIn in [[stage_in]], texture2d<float> src [[texture(0)]]) { return src.sample(sLin, in.uv); }
