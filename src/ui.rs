use std::{
    collections::HashMap,
    io::{BufWriter, Cursor},
};

use ::png::{BitDepth, ColorType, Encoder};
use image::{DynamicImage, EncodableLayout, Pixel, Rgb, RgbImage};
use imageproc::{
    definitions::Clamp,
    drawing::{Canvas, draw_filled_rect_mut, draw_hollow_rect_mut},
    rect::Rect,
};
use serde::{Deserialize, Serialize};

mod table;
pub use table::*;
mod text;
pub use text::*;

#[derive(clap::Args, Deserialize, Clone, Debug, Default)]
pub struct Size {
    /// width of the image
    #[arg(short, long, default_value_t = 800)]
    width: u32,
    /// heigth of the image
    #[arg(short('x'), long, default_value_t = 480)]
    height: u32,
}

#[derive(Deserialize, Clone, Copy, Debug, Serialize, Default)]
pub enum Value {
    #[default]
    Min,
    Max,
    Center,
    Relative(f64),
    Absolute(u32),
}

impl Value {
    fn get(self, size: u32, own_size: u32) -> u32 {
        match self {
            Value::Relative(v) => (v * size as f64) as u32,
            Value::Absolute(v) => v,
            Value::Min => 0,
            Value::Max => size.saturating_sub(own_size),
            Value::Center => size.saturating_sub(own_size) >> 1,
        }
    }
}

#[derive(Deserialize, Clone, Debug, Serialize, Default)]
pub struct Position {
    pub x: Value,
    pub y: Value,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Line {
    pub p1: Position,
    pub p2: Position,
}
impl Line {
    fn get(&self, area: &Area) -> ((f32, f32), (f32, f32)) {
        let size = Size {
            width: 1,
            height: 1,
        };

        let p1 = self.p1.get(area, &size);
        let p2 = self.p2.get(area, &size);
        ((p1.0 as f32, p1.1 as f32), (p2.0 as f32, p2.1 as f32))
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rectangle {
    #[serde(flatten)]
    pub pos: Position,
    pub width: Value,
    pub height: Value,
    #[serde(default)]
    pub fill: bool,
}
impl Rectangle {
    fn get(&self, area: &Area) -> Rect {
        let width = self.width.get(area.width, 0);
        let height = self.height.get(area.height, 0);
        let (x, y) = self.pos.get(area, &Size { width, height });

        Rect::at(x as i32, y as i32).of_size(width, height)
    }

    pub fn draw<C>(&self, area: &Area, brush: C::Pixel, image: &mut C)
    where
        C: Canvas,
        <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
    {
        let rect = self.get(area);
        if self.fill {
            draw_filled_rect_mut(image, rect, brush);
        } else {
            draw_hollow_rect_mut(image, rect, brush);
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Section {
    #[serde(flatten)]
    pub pos: Position,
    pub height: Value,
    pub width: Value,
    pub items: Vec<Item>,
}

impl Section {
    fn to_area(&self, area: &Area) -> Area {
        let width = self.width.get(area.width, 0);
        let height = self.height.get(area.height, 0);
        let (x, y) = self.pos.get(area, &Size { width, height });
        Area {
            x,
            y,
            height,
            width,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub enum Item {
    Text(Text),
    Rectangle(Rectangle),
    Table(Table),
    TextTable(TextTable),
    Empty,
}

impl Item {
    fn size(&self) -> Size {
        match self {
            Item::Text(text) => text.final_size(),
            Item::Rectangle(rectangle) => todo!(),
            Item::Empty => Size::default(),
            Item::Table(table) => todo!(),
            Item::TextTable(text_table) => todo!(),
        }
    }

    pub fn draw<C>(&self, area: &Area, brush: C::Pixel, image: &mut C)
    where
        C: Canvas,
        <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
    {
        match self {
            Item::Text(text) => text.draw(area, brush, image),
            Item::Rectangle(rectangle) => rectangle.draw(area, brush, image),
            Item::Table(table) => table.draw(area, brush, image),
            Item::Empty => (),
            Item::TextTable(text_table) => text_table.draw(area, brush, image),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Area {
    x: u32,
    y: u32,
    height: u32,
    width: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Ui {
    pub width: u32,
    pub height: u32,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub rectangles: Vec<Rectangle>,
}

impl Position {
    pub fn get(&self, area: &Area, own_size: &Size) -> (u32, u32) {
        (
            self.x.get(area.width, own_size.width) + area.x,
            self.y.get(area.height, own_size.height) + area.y,
        )
    }
}

pub fn call_pre_draw(items: &mut [Item], data: &HashMap<String, String>) {
    for item in items.iter_mut() {
        match item {
            Item::Text(text) => text.pre_draw(data),
            Item::Table(table) => table.pre_draw(data),
            Item::TextTable(table) => table.pre_draw(data),
            _ => (),
        }
    }
}

pub fn call_draw<C>(items: &mut [Item], area: &Area, brush: C::Pixel, image: &mut C)
where
    C: Canvas,
    <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
{
    for item in items.iter() {
        match item {
            Item::Text(text) => text.draw(area, brush, image),
            Item::Rectangle(rectangle) => rectangle.draw(area, brush, image),
            Item::Table(table) => table.draw(area, brush, image),
            Item::Empty => (),
            Item::TextTable(text_table) => text_table.draw(area, brush, image),
        }
    }
}

pub fn render_ui(ui: &mut Ui, data: HashMap<String, String>) -> anyhow::Result<Vec<u8>> {
    let mut image = RgbImage::new(ui.width, ui.height);
    image.fill(255u8);

    let area = Area {
        x: 0,
        y: 0,
        width: ui.width,
        height: ui.height,
    };

    let brush = Rgb([0u8, 0u8, 0u8]);

    for section in ui.sections.iter_mut() {
        let area = section.to_area(&area);
        call_pre_draw(&mut section.items, &data);
        call_draw(&mut section.items, &area, brush, &mut image);
    }

    for rectangle in ui.rectangles.iter() {
        let rect = rectangle.get(&area);
        if rectangle.fill {
            draw_filled_rect_mut(&mut image, rect, brush);
        } else {
            draw_hollow_rect_mut(&mut image, rect, brush);
        }
    }

    // for t in ui.lines {
    //     let (start, end) = t.get(&area);
    //     println!("start {start:?}, end:{end:?}");
    //     draw_line_segment_mut(&mut image, start, end, brush);
    // }

    let img = DynamicImage::ImageRgb8(image).to_luma8();

    let mut buf = Vec::<u8>::new();
    let buffer = Cursor::new(&mut buf);
    let writer = BufWriter::new(buffer);

    let mut encoder = Encoder::new(writer, img.width(), img.height());

    encoder.set_color(ColorType::Grayscale);
    encoder.set_depth(BitDepth::Eight);

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(img.as_bytes())?;
    png_writer.finish()?;
    Ok(buf)
}
