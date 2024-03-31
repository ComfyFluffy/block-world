use std::cmp::Ordering;

use crate::types::{Direction, TextureId};

#[derive(Debug, Clone, PartialEq)]
struct Face {
    pub uv: [f32; 4],
    pub texture: TextureId,
    pub cullface: Option<Direction>,
}

#[derive(Debug, Clone, PartialEq)]
struct Faces([Face; 6]);

impl Faces {
    pub fn new_with_texture_default_cullface(texture: TextureId) -> Self {
        Self(Direction::ALL.map(|direction| Face {
            uv: [0.0, 0.0, 1.0, 1.0],
            texture,
            cullface: Some(direction),
        }))
    }

    pub fn new_with_texture_no_cullface(texture: TextureId) -> Self {
        Self(Direction::ALL.map(|_direction| Face {
            uv: [0.0, 0.0, 1.0, 1.0],
            texture,
            cullface: None,
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Voxel {
    from: [f32; 3],
    to: [f32; 3],
    faces: Faces,
}

impl Voxel {
    fn volume(&self) -> f32 {
        let [x1, y1, z1] = self.from;
        let [x2, y2, z2] = self.to;
        (x2 - x1) * (y2 - y1) * (z2 - z1)
    }
}

struct Model {
    pub voxels: Vec<Voxel>,
}

impl Model {
    pub fn from_voxels(voxels: impl IntoIterator<Item = Voxel>) -> Self {
        let mut voxels = voxels.into_iter().collect::<Vec<_>>();
        voxels.sort_by(|a, b| {
            b.volume()
                .partial_cmp(&a.volume())
                .unwrap_or(Ordering::Equal)
        });
        Self { voxels }
    }
}

#[cfg(test)]
mod tests {
    use super::{Faces, Model, Voxel};

    #[test]
    fn test_new_model() {
        let voxels = vec![
            Voxel {
                from: [-1.0, -1.0, -1.0],
                to: [0.0, 0.0, 0.0],
                faces: Faces::new_with_texture_no_cullface(0),
            },
            Voxel {
                from: [0.0, 0.0, 0.0],
                to: [16.0, 16.0, 16.0],
                faces: Faces::new_with_texture_default_cullface(0),
            },
        ];
        let model = Model::from_voxels(voxels);
    }
}
