use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Direction {
    Up = 0,
    Down,
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const ALL: [Direction; 6] = {
        use Direction::*;
        [Up, Down, North, South, East, West]
    };

    pub fn to_offset(&self) -> (i32, i32, i32) {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockType {
    pub name: String,
    pub texture: Option<String>,
    pub transparent: bool,
}

pub type BlockTypeId = u32;
pub type TextureId = usize;

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    pub block_types: IndexMap<String, BlockType>,
    pub block_textures: HashMap<(BlockTypeId, Direction), TextureId>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let block_types = indexmap! {
            "air".to_string() => BlockType {
                name: "air".to_string(),
                texture: None,
                transparent: true,
            },
            "stone".to_string() => BlockType {
                name: "stone".to_string(),
                texture: Some("stone.png".to_string()),
                transparent: false,
            },
            "grass".to_string() => BlockType {
                name: "grass".to_string(),
                texture: Some("grass.png".to_string()),
                transparent: false,
            },
        };

        assert!(block_types.get_index_of("air") == Some(0));

        Self {
            block_types,
            block_textures: HashMap::new(),
        }
    }

    pub fn is_block_transparent(&self, block_type_id: BlockTypeId) -> bool {
        self.block_types[block_type_id as usize].transparent
    }

    pub fn set_uniform_texture(&mut self, block_type_id: BlockTypeId, texture_id: TextureId) {
        for &direction in Direction::ALL.iter() {
            self.block_textures
                .insert((block_type_id, direction), texture_id);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub blocks: [[[BlockTypeId; 16]; 16]; 256],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            blocks: [[[0; 16]; 16]; 256],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Hash, Copy)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

pub struct World {
    pub chunks: HashMap<ChunkPosition, Chunk>,
    pub block_registry: BlockRegistry,
}

impl World {
    pub fn new(block_registry: BlockRegistry) -> Self {
        Self {
            chunks: HashMap::new(),
            block_registry,
        }
    }

    pub fn fill_sphere(&mut self, center: [i32; 3], radius: i32, block_type_id: BlockTypeId) {
        for x in center[0] - radius..center[0] + radius {
            for y in center[1] - radius..center[1] + radius {
                for z in center[2] - radius..center[2] + radius {
                    let dx = x - center[0];
                    let dy = y - center[1];
                    let dz = z - center[2];

                    if dx * dx + dy * dy + dz * dz <= radius * radius {
                        self[[x, y, z]] = block_type_id;
                    }
                }
            }
        }
    }

    pub fn fill_cuboid(&mut self, min: [i32; 3], max: [i32; 3], block_type_id: BlockTypeId) {
        for x in min[0]..max[0] {
            for y in min[1]..max[1] {
                for z in min[2]..max[2] {
                    self[[x, y, z]] = block_type_id;
                }
            }
        }
    }
}

impl Index<[i32; 3]> for World {
    type Output = BlockTypeId;

    fn index(&self, index: [i32; 3]) -> &Self::Output {
        let chunk_position = ChunkPosition {
            x: index[0] / 16,
            z: index[2] / 16,
        };

        if let Some(chunk) = self.chunks.get(&chunk_position) {
            &chunk.blocks[(index[0] % 16) as usize][(index[2] % 16) as usize]
                [(index[1] % 256) as usize]
        } else {
            &0
        }
    }
}

impl IndexMut<[i32; 3]> for World {
    fn index_mut(&mut self, index: [i32; 3]) -> &mut Self::Output {
        let chunk_position = ChunkPosition {
            x: index[0] / 16,
            z: index[2] / 16,
        };

        let chunk = self
            .chunks
            .entry(chunk_position)
            .or_insert_with(|| Chunk::default());

        &mut chunk.blocks[(index[0] % 16) as usize][(index[2] % 16) as usize]
            [(index[1] % 256) as usize]
    }
}
