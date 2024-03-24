use std::collections::HashMap;

use crate::types::{BlockTypeId, Chunk, ChunkPosition, Direction, World};
use rayon::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VisibleFace {
    position: (u32, u32, u32),
    direction: Direction,
    block_type_id: BlockTypeId,
}

impl VisibleFace {
    pub fn all_faces(
        position: (u32, u32, u32),
        block_type_id: BlockTypeId,
    ) -> impl Iterator<Item = Self> {
        Direction::ALL
            .into_iter()
            .map(move |direction| VisibleFace {
                position,
                direction,
                block_type_id,
            })
    }
}

fn cull_faces_for_chunk(
    world: &World,
    chunk: &Chunk,
    chunk_position: ChunkPosition,
) -> Vec<VisibleFace> {
    chunk
        .blocks
        .par_iter()
        .enumerate()
        .flat_map_iter(move |(y, xz_plane)| {
            xz_plane.iter().enumerate().flat_map(move |(x, z_column)| {
                z_column
                    .iter()
                    .enumerate()
                    .flat_map(move |(z, block_type_id)| {
                        check_visible_faces_for_block(
                            *block_type_id,
                            world,
                            chunk,
                            chunk_position,
                            (x as u32, y as u32, z as u32),
                        )
                    })
            })
        })
        .collect()
}

fn cull_faces(world: &World) -> HashMap<ChunkPosition, Vec<VisibleFace>> {
    world
        .chunks
        .par_iter()
        .map(|(chunk_position, chunk)| {
            let visible_faces = cull_faces_for_chunk(world, chunk, *chunk_position);
            (*chunk_position, visible_faces)
        })
        .collect()
}

fn check_visible_faces_for_block(
    block_type_id: BlockTypeId,
    world: &World,
    chunk: &Chunk,
    chunk_position: ChunkPosition,
    block_position: (u32, u32, u32),
) -> Vec<VisibleFace> {
    if block_type_id == 0 {
        return Vec::new();
    }

    let (x, y, z) = block_position;

    let block_registry = &world.block_registry;
    if block_registry.is_block_transparent(block_type_id) {
        return VisibleFace::all_faces(block_position, block_type_id).collect();
    }
    let mut visible_faces = Vec::new();

    // If the block is at the edge of the chunk, check for
    // adjacent blocks in the neighboring chunk using the
    // chunk_position to index into the world's chunks.
    // If the neighboring chunk doesn't exist, the face is
    // invisible.
    // For blocks at the top or bottom of the chunk, treat
    // the neighboring chunk as air.
    for direction in Direction::ALL.into_iter() {
        let (dx, dy, dz) = direction.to_offset();
        let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);

        if y <= 0 && direction == Direction::Down || y >= 255 && direction == Direction::Up {
            visible_faces.push(VisibleFace {
                position: (x as u32, y as u32, z as u32),
                direction,
                block_type_id,
            });
            continue;
        }

        if nx < 0 || nx >= 16 || nz < 0 || nz >= 16 {
            let (cx, cz) = (chunk_position.x, chunk_position.z);
            let (ncx, ncz) = (cx + dx, cz + dz);
            let neighbor_chunk_position = ChunkPosition { x: ncx, z: ncz };
            let neighbor_chunk = world.chunks.get(&neighbor_chunk_position);

            if let Some(neighbor_chunk) = neighbor_chunk {
                let neighbor_block_type_id = neighbor_chunk.blocks[y as usize]
                    [((nx + 16) % 16) as usize][((nz + 16) % 16) as usize];

                if block_registry.is_block_transparent(neighbor_block_type_id) {
                    visible_faces.push(VisibleFace {
                        position: (x as u32, y as u32, z as u32),
                        direction,
                        block_type_id,
                    });
                }
            }
        } else {
            let neighbor_block_type_id = chunk.blocks[ny as usize][nx as usize][nz as usize];

            if block_registry.is_block_transparent(neighbor_block_type_id) {
                visible_faces.push(VisibleFace {
                    position: (x as u32, y as u32, z as u32),
                    direction,
                    block_type_id,
                });
            }
        }
    }
    visible_faces
}

fn update_visible_faces(
    world: &World,
    visible_faces: &mut HashMap<ChunkPosition, Vec<VisibleFace>>,
    chunk_positions: &[ChunkPosition],
) {
    for chunk_position in chunk_positions {
        let chunk = world.chunks.get(chunk_position).unwrap();
        let new_visible_faces = cull_faces_for_chunk(world, chunk, *chunk_position);
        visible_faces.insert(*chunk_position, new_visible_faces);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::types::BlockRegistry;

    use super::*;

    #[test]
    fn test_middle_block() {
        let block_registry = BlockRegistry::new();
        let world = World::new(block_registry);
        let chunk = Chunk::default();
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let block_position = (8, 64, 8);
        let block_type_id = 1; // Replace with the actual block type ID

        let visible_faces = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position,
        );

        assert_eq!(visible_faces.len(), 6);
    }

    #[test]
    fn test_top_and_bottom_blocks() {
        let block_registry = BlockRegistry::new();
        let world = World::new(block_registry);
        let chunk = Chunk::default();
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let block_type_id = 1; // Replace with the actual block type ID

        // Top block
        let block_position_top = (8, 255, 8);
        let visible_faces_top = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_top,
        );
        assert_eq!(visible_faces_top.len(), 6);

        // Bottom block
        let block_position_bottom = (8, 0, 8);
        let visible_faces_bottom = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_bottom,
        );
        assert_eq!(visible_faces_bottom.len(), 6);
    }

    #[test]
    fn test_chunk_edge_not_loaded() {
        let block_registry = BlockRegistry::new();
        let world = World::new(block_registry);
        let chunk = Chunk::default();
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let block_type_id = 1; // Replace with the actual block type ID

        // Left edge
        let block_position_left = (0, 64, 8);
        let visible_faces_left = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_left,
        )
        .into_iter()
        .map(|f| f.direction)
        .collect::<HashSet<_>>();
        assert_eq!(visible_faces_left.len(), 5);
        assert!(!visible_faces_left.contains(&Direction::West));

        // Right edge
        let block_position_right = (15, 64, 8);
        let visible_faces_right = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_right,
        );
        assert_eq!(visible_faces_right.len(), 5);

        // Front edge
        let block_position_front = (8, 64, 0);
        let visible_faces_front = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_front,
        );
        assert_eq!(visible_faces_front.len(), 5);

        // Back edge
        let block_position_back = (8, 64, 15);
        let visible_faces_back = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
            chunk_position,
            block_position_back,
        );
        assert_eq!(visible_faces_back.len(), 5);
    }

    #[test]
    fn test_chunk_edge_loaded() {
        let block_registry = BlockRegistry::new();
        let mut world = World::new(block_registry);
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        world.chunks.insert(chunk_position, Chunk::default());
        let block_type_id = 1; // Replace with the actual block type ID

        let neighbor_chunk = Chunk::default();
        let neighbor_chunk_position = ChunkPosition { x: 1, z: 0 };
        world.chunks.insert(neighbor_chunk_position, neighbor_chunk);

        let block_position_x_plus = (15, 64, 8);
        let visible_faces = check_visible_faces_for_block(
            block_type_id,
            &world,
            &world.chunks[&chunk_position],
            chunk_position,
            block_position_x_plus,
        );
        assert_eq!(visible_faces.len(), 6);

        world
            .chunks
            .get_mut(&neighbor_chunk_position)
            .unwrap()
            .blocks[64][0][8] = 1; // solid block
        assert!(!world.block_registry.block_types[1].transparent);
        let visible_faces = check_visible_faces_for_block(
            block_type_id,
            &world,
            &world.chunks[&chunk_position],
            chunk_position,
            block_position_x_plus,
        );
        assert_eq!(visible_faces.len(), 5);
    }

    #[test]
    fn test_chunk_dig_one_block() {
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let block_registry = BlockRegistry::new();
        let mut world = World::new(block_registry);
        world.chunks.insert(chunk_position, Chunk::default());
        let chunk = world.chunks.get_mut(&chunk_position).unwrap();

        assert!(!world.block_registry.is_block_transparent(1));

        let stone_id = 1;

        for y in 0..64 {
            for x in 0..16 {
                for z in 0..16 {
                    chunk.blocks[y as usize][x][z] = stone_id;
                }
            }
        }
        let visible_faces = cull_faces(&world);
        assert_eq!(
            visible_faces
                .into_iter()
                .map(|(_, v)| v.len())
                .sum::<usize>(),
            16 * 16 * 2
        );

        world.chunks.get_mut(&chunk_position).unwrap().blocks[63][1][1] = 0;

        let visible_faces = cull_faces(&world);
        assert_eq!(
            visible_faces
                .into_iter()
                .map(|(_, v)| v.len())
                .sum::<usize>(),
            16 * 16 * 2 + 4
        );
    }
}
