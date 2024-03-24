use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone)]
pub struct BlockRegistry {
    pub block_types: IndexMap<String, BlockType>,
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

        Self { block_types }
    }

    pub fn is_block_transparent(&self, block_type_id: BlockTypeId) -> bool {
        self.block_types[block_type_id as usize].transparent
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
}
