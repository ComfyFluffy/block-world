use std::ops::Deref;

use image::RgbaImage;
use indexmap::{indexmap, IndexMap};

#[derive(Debug, Clone)]
pub struct Texture {
    pub image: RgbaImage,
}

#[derive(Debug, Clone, Default)]
pub struct TextureRegistry(pub IndexMap<String, Texture>);

impl TextureRegistry {
    pub fn new() -> Self {
        let stone_image = image::open("stone.png").unwrap().to_rgba8();
        TextureRegistry(indexmap! {
            "stone".to_string() => Texture { image: stone_image },
        })
    }
}

impl Deref for TextureRegistry {
    type Target = IndexMap<String, Texture>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
