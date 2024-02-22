use crate::types::{BlockTypeId, Chunk, ChunkPosition, World};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

impl Direction {
    const ALL: [Direction; 6] = {
        use Direction::*;
        [Up, Down, North, South, East, West]
    };

    fn to_offset(&self) -> (i32, i32, i32) {
        use Direction::*;
        match self {
            Up => (0, 1, 0),
            Down => (0, -1, 0),
            North => (0, 0, -1),
            South => (0, 0, 1),
            East => (1, 0, 0),
            West => (-1, 0, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibleFace {
    position: (u32, u32, u32),
    direction: Direction,
    block_type_id: BlockTypeId,
}

fn cull_faces(world: &World) -> Vec<VisibleFace> {
    world
        .chunks
        .par_iter()
        .flat_map(|(chunk_position, chunk)| {
            chunk
                .blocks
                .into_par_iter()
                .enumerate()
                .flat_map(move |(y, xz_plane)| {
                    xz_plane
                        .into_par_iter()
                        .enumerate()
                        .flat_map(move |(x, z_column)| {
                            z_column.into_par_iter().enumerate().flat_map(
                                move |(z, block_type_id)| {
                                    check_visible_faces_for_block(
                                        block_type_id,
                                        world,
                                        chunk,
                                        *chunk_position,
                                        (x as i32, y as i32, z as i32),
                                    )
                                },
                            )
                        })
                })
        })
        .collect()
}

fn check_visible_faces_for_block(
    block_type_id: BlockTypeId,
    world: &World,
    chunk: &Chunk,
    chunk_position: ChunkPosition,
    block_position: (i32, i32, i32),
) -> Vec<VisibleFace> {
    let (x, y, z) = block_position;

    let mut visible_faces = Vec::new();

    let block_registry = &world.block_registry;
    if block_registry.is_block_transparent(block_type_id) {
        return visible_faces; // Skip transparent blocks and air (id 0)
    }

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

#[cfg(test)]
mod tests {
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
        );
        assert_eq!(visible_faces_left.len(), 5);

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
        let chunk = Chunk::default();
        let chunk_position = ChunkPosition { x: 0, z: 0 };
        let block_type_id = 1; // Replace with the actual block type ID

        let neighbor_chunk = Chunk::default();
        let neighbor_chunk_position = ChunkPosition { x: 1, z: 0 };
        world.chunks.insert(neighbor_chunk_position, neighbor_chunk);

        let block_position_x_plus = (15, 64, 8);
        let visible_faces = check_visible_faces_for_block(
            block_type_id,
            &world,
            &chunk,
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
            &chunk,
            chunk_position,
            block_position_x_plus,
        );
        assert_eq!(visible_faces.len(), 5);
    }
}
