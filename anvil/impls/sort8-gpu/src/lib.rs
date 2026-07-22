//! ANV-003 GPU lane: the 19-comparator sorting network as a compute shader.
//!
//! Same algorithm as sort8-network - the size-optimal sorting network for 8
//! inputs - but run on the GPU, one word per shader invocation, the whole
//! benchmark input stream sorted in one dispatch. The interesting question
//! this lane asks the leaderboard: for a job this small per element, does a
//! GPU's throughput beat a CPU core's latency once you pay for moving the
//! data there and back? The score is honest end-to-end time: generate,
//! upload, sort, download, checksum.
//!
//! Correctness story: the Lean model is `Razor.Anvil.sortNetwork` - the
//! same 19 comparators, in the same order, as the WGSL below - and
//! `Razor.Anvil.network_refines` is the admission proof that the network
//! agrees with the bubble-sort spec on all 2^64 inputs. The shader is a
//! transliteration of that model, exactly as the CPU lanes are
//! transliterations of theirs, and the differential check runs this lane
//! against the executable spec on the full benchmark input stream.
//!
//! Native-only: there is no wasm-fuel score for a GPU (fuel counts
//! interpreter instructions, and the work here happens outside the
//! interpreter). On a machine with no GPU adapter, `available()` is false
//! and the harness reports the lane as not measurable rather than failing.

use std::sync::OnceLock;

// WGSL has no 64-bit integers: each word travels as two u32 lanes (lo, hi),
// and the shader works on the 8 extracted bytes directly.
const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<vec2<u32>>;

fn cswap(b: ptr<function, array<u32, 8>>, i: u32, j: u32) {
    let x = (*b)[i];
    let y = (*b)[j];
    (*b)[i] = min(x, y);
    (*b)[j] = max(x, y);
}

@compute @workgroup_size(64)
fn sort8(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&data)) {
        return;
    }
    let v = data[idx];
    var b: array<u32, 8>;
    for (var i = 0u; i < 4u; i = i + 1u) {
        b[i] = (v.x >> (8u * i)) & 0xffu;
        b[i + 4u] = (v.y >> (8u * i)) & 0xffu;
    }
    // The 19-comparator size-optimal network (model: Razor.Anvil.sortNetwork).
    cswap(&b, 0u, 1u); cswap(&b, 2u, 3u); cswap(&b, 4u, 5u); cswap(&b, 6u, 7u);
    cswap(&b, 0u, 2u); cswap(&b, 1u, 3u); cswap(&b, 4u, 6u); cswap(&b, 5u, 7u);
    cswap(&b, 1u, 2u); cswap(&b, 5u, 6u); cswap(&b, 0u, 4u); cswap(&b, 3u, 7u);
    cswap(&b, 1u, 5u); cswap(&b, 2u, 6u);
    cswap(&b, 1u, 4u); cswap(&b, 3u, 6u);
    cswap(&b, 2u, 4u); cswap(&b, 3u, 5u);
    cswap(&b, 3u, 4u);
    var lo = 0u;
    var hi = 0u;
    for (var i = 0u; i < 4u; i = i + 1u) {
        lo = lo | (b[i] << (8u * i));
        hi = hi | (b[i + 4u] << (8u * i));
    }
    data[idx] = vec2<u32>(lo, hi);
}
"#;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
}

fn gpu() -> Option<&'static Gpu> {
    static GPU: OnceLock<Option<Gpu>> = OnceLock::new();
    GPU.get_or_init(|| {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .ok()?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sort8"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sort8"),
            layout: None,
            module: &module,
            entry_point: Some("sort8"),
            compilation_options: Default::default(),
            cache: None,
        });
        Some(Gpu { device, queue, pipeline })
    })
    .as_ref()
}

/// Whether this machine has a usable GPU adapter at all.
pub fn available() -> bool {
    gpu().is_some()
}

/// Sort the 8 bytes of every input word on the GPU. Streams of any size
/// are processed in chunks that respect the adapter's dispatch and buffer
/// binding limits (2^21 words = 16 MB and 32768 workgroups per dispatch).
pub fn solve_many(inputs: &[u64]) -> Vec<u64> {
    const CHUNK: usize = 1 << 21;
    if inputs.len() > CHUNK {
        return inputs.chunks(CHUNK).flat_map(solve_chunk).collect();
    }
    solve_chunk(inputs)
}

fn solve_chunk(inputs: &[u64]) -> Vec<u64> {
    let g = gpu().expect("no GPU adapter (check available() first)");
    let mut bytes = Vec::with_capacity(inputs.len() * 8);
    for x in inputs {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    use wgpu::util::DeviceExt;
    let storage = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sort8 data"),
        contents: &bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let readback = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sort8 readback"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let layout = g.pipeline.get_bind_group_layout(0);
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: storage.as_entire_binding() }],
    });
    let mut enc = g.device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&g.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((inputs.len() as u32).div_ceil(64), 1, 1);
    }
    enc.copy_buffer_to_buffer(&storage, 0, &readback, 0, bytes.len() as u64);
    g.queue.submit([enc.finish()]);
    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map_async dropped").expect("map failed");
    let out = slice.get_mapped_range();
    let result = out
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    drop(out);
    readback.unmap();
    result
}

/// Single-word entry point (one dispatch per call - used by differential
/// spot checks, never for scoring).
pub fn solve(x: u64) -> u64 {
    solve_many(&[x])[0]
}

/// Whole-stream benchmark entry: same input stream and checksum as every
/// other lane, but the sorting happens on the GPU in one dispatch.
pub fn bench_batch(seed: u64, iters: u64) -> u64 {
    let inputs: Vec<u64> = anvil_abi::input_stream(seed, iters, |x| x).collect();
    solve_many(&inputs).into_iter().fold(0u64, |acc, x| acc.wrapping_add(x))
}
