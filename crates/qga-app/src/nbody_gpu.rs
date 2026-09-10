//! Cosmos N-body compute. Lives in qga-app; qga-gpu only draws the particles.
//!
//! Integrators are Software fact: semi-implicit Euler (default) or velocity
//! Verlet / leapfrog with cached a. Same symplectic family, one force eval
//! per step. Not a flux-flywheel theorem.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use qga_gpu::{GpuContext, GpuParticle};
use qga_sim::{cosmos_diag, nbody_substeps, CosmosDiag, Particle, NBODY_WORKGROUP};
use wgpu::util::DeviceExt;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Integrator {
    #[default]
    Euler = 0,
    Verlet = 1,
}

impl Integrator {
    pub fn name(self) -> &'static str {
        match self {
            Self::Euler => "euler",
            Self::Verlet => "verlet",
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SimParams {
    pub n: u32,
    pub dt: f32,
    pub g: f32,
    pub eps2: f32,
    pub kappa: f32,
    pub integrator: u32,
    pub world_r0: f32,
    pub annulus_k: f32,
    pub annulus_on: f32,
    pub pad: [f32; 3],
}

const _: () = assert!(std::mem::size_of::<SimParams>() == 48);

pub struct NbodyGpu {
    integrate: wgpu::ComputePipeline,
    eval_force: wgpu::ComputePipeline,
    verlet_drift: wgpu::ComputePipeline,
    verlet_kick: wgpu::ComputePipeline,
    a: wgpu::Buffer,
    b: wgpu::Buffer,
    /// Kept alive for the compute bind groups (Verlet cached a).
    #[allow(dead_code)]
    acc: wgpu::Buffer,
    /// Kept alive for the compute bind groups.
    #[allow(dead_code)]
    sim_buf: wgpu::Buffer,
    bind_ab: wgpu::BindGroup,
    bind_ba: wgpu::BindGroup,
    staging: wgpu::Buffer,
    n: u32,
    sim: SimParams,
    integrator: Integrator,
    m_heavy: f32,
    /// true → `a` is the current source.
    ping: bool,
    cpu: Vec<GpuParticle>,
    cpu_stale: bool,
}

impl NbodyGpu {
    pub fn new(
        gpu: &GpuContext,
        particles: &[GpuParticle],
        sim: SimParams,
        integrator: Integrator,
    ) -> Result<Self> {
        let device = &gpu.device;
        let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nbody"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/nbody.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nbody-layout"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, false),
                uniform_entry(2),
                storage_entry(3, false),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nbody-pl"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |entry: &'static str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &sm,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let integrate = pipeline("integrate");
        let eval_force = pipeline("eval_force");
        let verlet_drift = pipeline("verlet_drift");
        let verlet_kick = pipeline("verlet_kick");

        let usage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        let bytes = bytemuck::cast_slice(particles);
        let a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nbody-a"),
            contents: bytes,
            usage,
        });
        let b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nbody-b"),
            contents: bytes,
            usage,
        });
        let acc_bytes = vec![0u8; particles.len() * 16];
        let acc = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nbody-acc"),
            contents: &acc_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let mut sim = sim;
        sim.integrator = integrator as u32;
        let sim_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nbody-sim"),
            contents: bytemuck::bytes_of(&sim),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_ab = bind_nbody(device, &layout, &a, &b, &sim_buf, &acc);
        let bind_ba = bind_nbody(device, &layout, &b, &a, &sim_buf, &acc);
        let n = particles.len() as u32;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nbody-read"),
            size: (n as u64 * 32).max(32),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut gpu_nb = Self {
            integrate,
            eval_force,
            verlet_drift,
            verlet_kick,
            a,
            b,
            acc,
            sim_buf,
            bind_ab,
            bind_ba,
            staging,
            n,
            sim,
            integrator,
            m_heavy: particles.first().map(|p| p.mass).unwrap_or(9.0),
            ping: true,
            cpu: particles.to_vec(),
            cpu_stale: false,
        };
        if integrator == Integrator::Verlet && n > 0 {
            gpu_nb.dispatch(gpu, Pass::EvalForce, 1);
            gpu_nb.cpu_stale = true;
        }
        Ok(gpu_nb)
    }

    pub fn len(&self) -> u32 {
        self.n
    }

    pub fn integrator(&self) -> Integrator {
        self.integrator
    }

    pub fn params(&self) -> SimParams {
        self.sim
    }

    pub fn workgroup() -> u32 {
        NBODY_WORKGROUP
    }

    pub fn substeps(&self) -> u32 {
        nbody_substeps(
            self.sim.dt,
            self.sim.kappa,
            self.sim.g,
            self.sim.eps2.sqrt(),
            self.m_heavy,
        )
    }

    pub fn step(&mut self, gpu: &GpuContext, n_substeps: u32) {
        if self.n == 0 {
            return;
        }
        match self.integrator {
            Integrator::Euler => self.dispatch(gpu, Pass::Integrate, n_substeps.max(1)),
            Integrator::Verlet => {
                for _ in 0..n_substeps.max(1) {
                    self.dispatch(gpu, Pass::VerletDrift, 1);
                    self.dispatch(gpu, Pass::VerletKick, 1);
                }
            }
        }
        self.cpu_stale = true;
    }

    pub fn download(&mut self, gpu: &GpuContext) -> Result<&[GpuParticle]> {
        if self.n == 0 {
            self.cpu.clear();
            self.cpu_stale = false;
            return Ok(&self.cpu);
        }
        if !self.cpu_stale {
            return Ok(&self.cpu);
        }
        let bytes = self.n as u64 * 32;
        let src = if self.ping { &self.a } else { &self.b };
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nbody-download"),
            });
        encoder.copy_buffer_to_buffer(src, 0, &self.staging, 0, bytes);
        gpu.queue.submit(Some(encoder.finish()));
        let slice = self.staging.slice(0..bytes);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::Maintain::Wait);
        {
            let data = slice.get_mapped_range();
            let parts: &[GpuParticle] = bytemuck::cast_slice(&data);
            self.cpu.clear();
            self.cpu.extend_from_slice(parts);
        }
        self.staging.unmap();
        self.cpu_stale = false;
        Ok(&self.cpu)
    }

    pub fn diag(&mut self, gpu: &GpuContext) -> Result<CosmosDiag> {
        let parts = self.download(gpu)?;
        let cpu: Vec<Particle> = parts
            .iter()
            .map(|p| Particle {
                pos: p.pos.into(),
                mass: p.mass,
                vel: p.vel.into(),
                pad: p.pad,
            })
            .collect();
        Ok(cosmos_diag(&cpu, self.sim.g, self.sim.kappa))
    }

    fn dispatch(&mut self, gpu: &GpuContext, which: Pass, n_substeps: u32) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nbody"),
            });
        for _ in 0..n_substeps.max(1) {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("nbody-pass"),
                    timestamp_writes: None,
                });
                let pipeline = match which {
                    Pass::Integrate => &self.integrate,
                    Pass::EvalForce => &self.eval_force,
                    Pass::VerletDrift => &self.verlet_drift,
                    Pass::VerletKick => &self.verlet_kick,
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(
                    0,
                    if self.ping {
                        &self.bind_ab
                    } else {
                        &self.bind_ba
                    },
                    &[],
                );
                pass.dispatch_workgroups(self.n.div_ceil(NBODY_WORKGROUP), 1, 1);
            }
            self.ping = !self.ping;
        }
        gpu.queue.submit(Some(encoder.finish()));
    }
}

#[derive(Clone, Copy)]
enum Pass {
    Integrate,
    EvalForce,
    VerletDrift,
    VerletKick,
}

fn bind_nbody(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    src: &wgpu::Buffer,
    dst: &wgpu::Buffer,
    sim: &wgpu::Buffer,
    acc: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nbody-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: src.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: dst.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: sim.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: acc.as_entire_binding(),
            },
        ],
    })
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
