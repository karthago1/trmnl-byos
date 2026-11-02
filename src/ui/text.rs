use std::collections::HashMap;

use ab_glyph::{FontRef, PxScale};
use image::Pixel;
use imageproc::{
    definitions::Clamp,
    drawing::{Canvas, draw_text_mut, text_size},
};
use serde::Deserialize;

use crate::ui::{Area, Item, Position, Size};

thread_local! {
static FONT:FontRef<'static> = FontRef::try_from_slice(FONT_BYTE).expect("failed to parse font");
}

const FONT_BYTE: &[u8] = include_bytes!("../../data/0xProtoNerdFont-Regular.ttf");
#[derive(Deserialize, Clone, Debug)]
pub struct Text {
    pub text: String,
    #[serde(flatten)]
    pub pos: Position,
    pub size: f32,
    #[serde(default)]
    final_text: String,
    #[serde(default)]
    final_size: Size,
}

impl Text {
    pub fn new(text: String, pos: Position, font_size: f32) -> Self {
        Self {
            text,
            pos,
            size: font_size,
            final_text: String::new(),
            final_size: Size::default(),
        }
    }

    pub fn final_size(&self) -> Size {
        self.final_size.clone()
    }

    pub fn pre_draw(&mut self, data: &HashMap<String, String>) {
        let scale = PxScale {
            x: self.size,
            y: self.size,
        };
        let mut text = self.text.clone();
        for (key, value) in data.iter() {
            let key = format!("{{{key}}}");
            text = text.replace(&key, value);
        }
        self.final_text = text;
        let (w, h) = FONT.with(|font| text_size(scale, &font, &self.text));
        self.final_size.width = w;
        self.final_size.height = h;
    }

    pub fn draw<C>(&self, area: &Area, brush: C::Pixel, image: &mut C)
    where
        C: Canvas,
        <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
    {
        let scale = PxScale {
            x: self.size,
            y: self.size,
        };

        let (x, y) = self.pos.get(area, &self.final_size);
        FONT.with(|font| {
            draw_text_mut(
                image,
                brush,
                x as i32,
                y as i32,
                scale,
                &font,
                &self.final_text,
            )
        });
    }
}

impl From<Text> for Item {
    fn from(value: Text) -> Self {
        Item::Text(value)
    }
}
