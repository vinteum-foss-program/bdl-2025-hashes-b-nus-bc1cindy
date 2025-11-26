// GPU-accelerated brute-force search for an 8-character base-62
// Second preimage using a (simple_hash)
// Uses WGPU for portable compute shader execution

use std::fs;
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    target_hash: u32,       // hash we want to match
    batch_size: u32,        // number of candidates processed per dispatch
    stride: u32,            // grid stride for 2D workgroup dispatch
    _pad: u32,              // alignment padding
    start_digits: [u32; 8], // base-62 starting position
}

/// Simple string hash: h = ((h << 5) - h) + c for each byte
fn simple_hash(s: &str) -> u32 {
    let mut h = 0u32;
    for b in s.bytes() {
        h = h.wrapping_shl(5).wrapping_sub(h).wrapping_add(b as u32);
    }
    h
}

/// Converts a 128-bit index into 8 base-62 digits
fn to_base62_digits(mut idx: u128) -> [u32; 8] {
    let mut d = [0u32; 8];
    for i in 0..8 {
        d[i] = (idx % 62u128) as u32;
        idx /= 62u128;
    }
    d
}

// WGSL compute shader — brute forces 8-char base-62 strings
// And selects the first match via atomic exchange.
const SHADER: &str = r#"
struct Params {
    target_hash: u32,
    batch_size: u32,
    stride: u32,
    _pad: u32,
    start_digits: array<u32, 8>,
};

@group(0) @binding(0) var<storage, read_write> found: atomic<u32>;
@group(0) @binding(1) var<storage, read_write> result: array<u32, 8>;
@group(0) @binding(2) var<storage, read> params: Params;

// One step of the simple hash
fn hash_step(h: u32, c: u32) -> u32 {
    return ((h << 5u) & 0xffffffffu) - h + c;
}

// Base-62 → ASCII mapping
fn map62(x: u32) -> u32 {
    if (x < 26u) {
        return 65u + x;              // 'A'..'Z'
    }
    if (x < 52u) {
        return 97u + (x - 26u);      // 'a'..'z'
    }
    return 48u + (x - 52u);          // '0'..'9'
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global_idx: u32 = gid.x + gid.y * params.stride;
    if (global_idx >= params.batch_size) {
        return;
    }

    // Decode global_idx into base-62 digits
    var v: u32 = global_idx;
    var ld0: u32 = v % 62u; v = v / 62u;
    var ld1: u32 = v % 62u; v = v / 62u;
    var ld2: u32 = v % 62u; v = v / 62u;
    var ld3: u32 = v % 62u; v = v / 62u;
    var ld4: u32 = v % 62u; v = v / 62u;
    var ld5: u32 = v % 62u; v = v / 62u;
    var ld6: u32 = v % 62u; v = v / 62u;
    var ld7: u32 = v % 62u;

    // Add start_digits with carry propagation
    var s0 = params.start_digits[0] + ld0;
    var c0 = 0u;
    if (s0 >= 62u) { s0 -= 62u; c0 = 1u; }

    var s1 = params.start_digits[1] + ld1 + c0;
    var c1 = 0u;
    if (s1 >= 62u) { s1 -= 62u; c1 = 1u; }

    var s2 = params.start_digits[2] + ld2 + c1;
    var c2 = 0u;
    if (s2 >= 62u) { s2 -= 62u; c2 = 1u; }

    var s3 = params.start_digits[3] + ld3 + c2;
    var c3 = 0u;
    if (s3 >= 62u) { s3 -= 62u; c3 = 1u; }

    var s4 = params.start_digits[4] + ld4 + c3;
    var c4 = 0u;
    if (s4 >= 62u) { s4 -= 62u; c4 = 1u; }

    var s5 = params.start_digits[5] + ld5 + c4;
    var c5 = 0u;
    if (s5 >= 62u) { s5 -= 62u; c5 = 1u; }

    var s6 = params.start_digits[6] + ld6 + c5;
    var c6 = 0u;
    if (s6 >= 62u) { s6 -= 62u; c6 = 1u; }

    var s7 = params.start_digits[7] + ld7 + c6;
    if (s7 >= 62u) { s7 -= 62u; }

    // Compute hash
    var h: u32 = 0u;
    let c0a = map62(s0);
    let c1a = map62(s1);
    let c2a = map62(s2);
    let c3a = map62(s3);
    let c4a = map62(s4);
    let c5a = map62(s5);
    let c6a = map62(s6);
    let c7a = map62(s7);

    h = hash_step(h, c0a);
    h = hash_step(h, c1a);
    h = hash_step(h, c2a);
    h = hash_step(h, c3a);
    h = hash_step(h, c4a);
    h = hash_step(h, c5a);
    h = hash_step(h, c6a);
    h = hash_step(h, c7a);

    // If match found, atomically store result
    if (h == params.target_hash) {
        let prev = atomicExchange(&found, 1u);
        if (prev == 0u) {
            result[0] = c0a;
            result[1] = c1a;
            result[2] = c2a;
            result[3] = c3a;
            result[4] = c4a;
            result[5] = c5a;
            result[6] = c6a;
            result[7] = c7a;
        }
    }
}
"#;

fn main() {
    let target = "Cindy";
    let target_hash = simple_hash(target);

    const BATCH_SIZE: u32 = 16_777_216u32; // 2^24 candidates per GPU dispatch
    const WG_SIZE: u32 = 64u32; // threads per group

    let total: u128 = 62u128.pow(8); // total search space: 62^8

    println!("Target '{}', hash 0x{:08x}", target, target_hash);
    println!("Total: {} (62^8)", total);
    println!("Batch size: {}", BATCH_SIZE);

    // Pollster allows to run async code in main().
    pollster::block_on(async {
        // GPU device & queue setup
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default()); // connects to the GPU driver
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default()) // request the most powerful available GPU
            .await
            .expect("No adapter");

        // "Device" (full access) + "queue" (for sending commands)
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Device");

        // Compile WGSL shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("preimage"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        // Create GPU buffers for atomic flag, result, and parameters
        let found_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("found"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST, // storage: read/write, source: read, destination: write
        });

        let result_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("result"),
            contents: bytemuck::cast_slice(&[0u32; 8]),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Staging buffers for reading GPU results back to CPU
        let staging_found = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_found"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_result = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_result"),
            size: 8 * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Define bind group layout
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl"),
            entries: &[
                // found flag
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // result buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout and compute pipeline
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("comp"),
            layout: Some(&pl),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        // Bind GPU buffers to shader bindings
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: found_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: result_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let start = Instant::now();
        let mut start_index: u128 = 0;

        // Main brute-force loop
        'outer: while start_index < total {
            let start_digits = to_base62_digits(start_index);

            // Grid configuration
            let total_invocations = ((BATCH_SIZE + WG_SIZE - 1) / WG_SIZE) as u32;
            let wg_x = total_invocations.min(65_535u32);
            let wg_y = (total_invocations + wg_x - 1) / wg_x;
            let stride = wg_x * WG_SIZE;

            // Update params
            let params = Params {
                target_hash,
                batch_size: BATCH_SIZE,
                stride,
                _pad: 0,
                start_digits,
            };

            // Push parameters to GPU
            queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));
            queue.write_buffer(&found_buf, 0, bytemuck::cast_slice(&[0u32]));
            queue.write_buffer(&result_buf, 0, bytemuck::cast_slice(&[0u32; 8]));

            // Build compute commands
            let mut enc = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("enc") });
            {
                let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(wg_x, wg_y, 1);
            }

            enc.copy_buffer_to_buffer(&found_buf, 0, &staging_found, 0, 4);
            enc.copy_buffer_to_buffer(&result_buf, 0, &staging_result, 0, 8 * 4);

            queue.submit(Some(enc.finish()));
            device.poll(wgpu::Maintain::Wait);

            // Read 'found' flag from GPU
            let slice = staging_found.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| ());
            device.poll(wgpu::Maintain::Wait);
            let data = slice.get_mapped_range();
            let found_val = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            drop(data);
            staging_found.unmap();

            // If match found, read result and save to file
            if found_val != 0 {
                let slice = staging_result.slice(..);
                slice.map_async(wgpu::MapMode::Read, |_| ());
                device.poll(wgpu::Maintain::Wait);
                let data = slice.get_mapped_range();
                let mut bytes = [0u8; 8];
                for i in 0..8 {
                    let ascii_val = u32::from_le_bytes([
                        data[i * 4],
                        data[i * 4 + 1],
                        data[i * 4 + 2],
                        data[i * 4 + 3],
                    ]);
                    bytes[i] = ascii_val as u8;
                }
                drop(data);
                staging_result.unmap();

                let found_str = String::from_utf8_lossy(&bytes).to_string();

                println!("\nFOUND preimage in {:.2?}!", start.elapsed());
                println!("Original: '{}'", target);
                println!("Found:    '{}' (len={})", found_str, found_str.len());
                println!("Bytes: {:?}", bytes);

                // Save result to file: "target,preimage"
                let mut content = Vec::new();
                content.extend_from_slice(target.as_bytes());
                content.push(b',');
                content.extend_from_slice(&bytes);

                fs::write("solutions/exercise05.txt", &content).ok();

                break 'outer;
            }

            start_index += BATCH_SIZE as u128;

            // Progress indicator
            if start_index % (BATCH_SIZE as u128 * 16) == 0 {
                let pct = (start_index as f64) / (total as f64) * 100.0;
                println!("[progress] {:.6}%  elapsed: {:.2?}", pct, start.elapsed());
            }
        }

        println!("Finished (elapsed {:.2?})", start.elapsed());
    });
}
