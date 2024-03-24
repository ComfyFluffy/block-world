use image::RgbaImage;
use indexmap::{indexmap, IndexMap};

pub struct Texture {
    pub image: RgbaImage,
}

pub struct TextureRegistry {
    pub textures: IndexMap<String, Texture>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        let stone_image = image::open("stone.png").unwrap().to_rgba8();
        TextureRegistry {
            textures: indexmap! {
                "stone".to_string() => Texture { image: stone_image },
            },
        }
    }
}
