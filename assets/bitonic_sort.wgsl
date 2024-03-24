const WORKGROUP_SIZE: u32 = 32;

struct BitSorter {
    block_size: u32,
    dim: u32,
}

@group(0) @binding(0) var<uniform> num_particles: u32;
@group(0) @binding(1) var<storage, read_write> particle_indicies: array<u32>;
@group(0) @binding(2) var<storage> particle_cell_indicies: array<u32>;
// Used in bitsort
@group(0) @binding(3) var<uniform> bit_sorter: BitSorter;
// Used in calculating the cell offsets
@group(0) @binding(3) var<storage, read_write> cell_offsets: array<atomic<u32> >;

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn bitonic_sort(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let i = invocation_id.x + invocation_id.y * 262144u;  // 256 * 1024
    let j = i ^ bit_sorter.block_size;

    if j < i || i >= num_particles {
        return;
    }

    var sign = 1;
    if (i & bit_sorter.dim) != 0 {
        sign = -1;
    }

    let key_i = particle_indicies[i];
    let key_j = particle_indicies[j];
    let value_i = particle_cell_indicies[key_i];
    let value_j = particle_cell_indicies[key_j];

    let diff = i32(value_i - value_j) * sign;
    if diff > 0 {
        particle_indicies[i] = key_j;
        particle_indicies[j] = key_i;
    }
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn calculate_cell_offsets(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= num_particles {
        return;
    }

    let particle_index = particle_indicies[index];
    let cell_index = particle_cell_indicies[particle_index];
    atomicMin(&cell_offsets[cell_index], index);
}
