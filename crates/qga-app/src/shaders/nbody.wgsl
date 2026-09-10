struct Particle {
    pos: vec3<f32>,
    mass: f32,
    vel: vec3<f32>,
    pad: f32,
};

struct SimParams {
    n: u32,
    dt: f32,
    g: f32,
    eps2: f32,
    kappa: f32,
    integrator: u32,
    world_r0: f32,
    annulus_k: f32,
    annulus_on: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<storage, read> src: array<Particle>;
@group(0) @binding(1) var<storage, read_write> dst: array<Particle>;
@group(0) @binding(2) var<uniform> params: SimParams;
@group(0) @binding(3) var<storage, read_write> acc_buf: array<vec4<f32>>;

var<workgroup> tile: array<vec4<f32>, 256>;

fn force_at(pos: vec3<f32>, mass: f32, pad: f32, lid: vec3<u32>) -> vec3<f32> {
    let n = params.n;
    var acc = vec3<f32>(0.0);
    let tiles = (n + 255u) / 256u;
    for (var t = 0u; t < tiles; t++) {
        let j = t * 256u + lid.x;
        if j < n {
            let o = src[j];
            tile[lid.x] = vec4<f32>(o.pos, o.mass);
        } else {
            tile[lid.x] = vec4<f32>(0.0);
        }
        workgroupBarrier();
        for (var k = 0u; k < 256u; k++) {
            let other = tile[k];
            let r = other.xyz - pos;
            let r2 = dot(r, r) + params.eps2;
            if r2 > params.eps2 * 1.01 {
                let inv = inverseSqrt(r2);
                acc += other.w * r * (inv * inv * inv);
            }
        }
        workgroupBarrier();
    }
    acc *= params.g;
    acc.z -= params.kappa * pos.z;

    if mass < 1.5 && pad >= 9.5 && params.annulus_on > 0.5 {
        let sp = u32(clamp(pad - 10.0, 0.0, 5.0) + 0.5);
        let ell = 6.0 - f32(sp);
        let r0 = params.world_r0;
        let ri = r0 * pow(6.0 / max(ell, 1.0), 0.6666667);
        var r_lo: f32;
        var r_hi: f32;
        if sp == 0u {
            r_lo = ri * 0.82;
        } else {
            let ell_in = 6.0 - f32(sp - 1u);
            r_lo = 0.5 * (ri + r0 * pow(6.0 / max(ell_in, 1.0), 0.6666667));
        }
        if sp == 5u {
            r_hi = ri * 1.015;
        } else {
            let ell_out = 6.0 - f32(sp + 1u);
            r_hi = 0.5 * (ri + r0 * pow(6.0 / max(ell_out, 1.0), 0.6666667));
        }
        let rho = length(pos.xy);
        if rho > 1e-4 {
            var dr: f32 = 0.0;
            if rho < r_lo {
                dr = rho - r_lo;
            } else if rho > r_hi {
                dr = rho - r_hi;
            }
            let k = params.annulus_k * ell * ell;
            let inv = 1.0 / rho;
            acc.x -= k * dr * pos.x * inv;
            acc.y -= k * dr * pos.y * inv;
        }
    }
    return acc;
}

@compute @workgroup_size(256)
fn integrate(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let i = gid.x;
    let n = params.n;
    if i >= n {
        return;
    }
    let p = src[i];
    let acc = force_at(p.pos, p.mass, p.pad, lid);
    var vel = p.vel + acc * params.dt;
    var pos = p.pos + vel * params.dt;
    dst[i] = Particle(pos, p.mass, vel, p.pad);
}

@compute @workgroup_size(256)
fn eval_force(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let i = gid.x;
    let n = params.n;
    if i >= n {
        return;
    }
    let p = src[i];
    let acc = force_at(p.pos, p.mass, p.pad, lid);
    acc_buf[i] = vec4<f32>(acc, 0.0);
}

@compute @workgroup_size(256)
fn verlet_drift(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let i = gid.x;
    let n = params.n;
    if i >= n {
        return;
    }
    let p = src[i];
    let a_n = acc_buf[i].xyz;
    var vel = p.vel + 0.5 * a_n * params.dt;
    var pos = p.pos + vel * params.dt;
    dst[i] = Particle(pos, p.mass, vel, p.pad);
}

@compute @workgroup_size(256)
fn verlet_kick(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
) {
    let i = gid.x;
    let n = params.n;
    if i >= n {
        return;
    }
    let p = src[i];
    let acc = force_at(p.pos, p.mass, p.pad, lid);
    acc_buf[i] = vec4<f32>(acc, 0.0);
    var vel = p.vel + 0.5 * acc * params.dt;
    dst[i] = Particle(p.pos, p.mass, vel, p.pad);
}
