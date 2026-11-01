use std::{
    cell::OnceCell,
    collections::HashMap,
    io::{BufWriter, Cursor},
};

use ::png::{BitDepth, ColorType, Encoder};
use ab_glyph::{FontRef, PxScale};
use image::{Pixel, Rgb, RgbImage};
use imageproc::{
    definitions::Clamp,
    drawing::{Canvas, draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut, text_size},
    rect::Rect,
};
use serde::{Deserialize, Serialize};

const FONT_BYTE: &[u8] = include_bytes!("../data/0xProtoNerdFont-Regular.ttf");

#[derive(clap::Args, Deserialize, Clone, Debug)]
pub struct Size {
    /// width of the image
    #[arg(short, long, default_value_t = 800)]
    width: u32,
    /// heigth of the image
    #[arg(short('x'), long, default_value_t = 480)]
    height: u32,
}

#[derive(Deserialize, Clone, Copy, Debug, Serialize)]
pub enum Value {
    Relative(f64),
    Absolute(u32),
    Min,
    Max,
    Center,
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

#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct Position {
    pub x: Value,
    pub y: Value,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Text {
    pub text: String,
    #[serde(flatten)]
    pub pos: Position,
    pub size: f32,
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

fn draw_text<C>(
    item: &Text,
    area: &Area,
    brush: C::Pixel,
    data: &HashMap<String, String>,
    image: &mut C,
    font: &FontRef<'_>,
) where
    C: Canvas,
    <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
{
    let scale = PxScale {
        x: item.size,
        y: item.size,
    };

    let mut text = item.text.clone();
    for (key, value) in data.iter() {
        let key = format!("{{{key}}}");
        text = text.replace(&key, value);
    }
    let (w, h) = text_size(scale, &font, &text);
    let (x, y) = item.pos.get(
        area,
        &Size {
            width: w,
            height: h,
        },
    );
    println!("text to draw at ({x}, {y})");
    draw_text_mut(image, brush, x as i32, y as i32, scale, &font, &text);
}

pub fn render_ui(ui: &Ui, data: HashMap<String, String>) -> anyhow::Result<Vec<u8>> {
    let font_cell = OnceCell::new();
    let font = font_cell.get_or_init(|| {
        println!("font initialized");
        FontRef::try_from_slice(FONT_BYTE).expect("failed to parse font")
    });

    let mut image = RgbImage::new(ui.width, ui.height);
    image.fill(255u8);

    let area = Area {
        x: 0,
        y: 0,
        width: ui.width,
        height: ui.height,
    };

    let brush = Rgb([0u8, 0u8, 0u8]);

    for section in ui.sections.iter() {
        let area = section.to_area(&area);
        println!("area to draw {area:?}");
        for item in section.items.iter() {
            match item {
                Item::Text(text) => draw_text(text, &area, brush, &data, &mut image, font),
                Item::Rectangle(rectangle) => {
                    let rect = rectangle.get(&area);
                    if rectangle.fill {
                        draw_filled_rect_mut(&mut image, rect, brush);
                    } else {
                        draw_hollow_rect_mut(&mut image, rect, brush);
                    }
                }
            }
        }
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

    save_monochrome_png(&image)
}

fn save_monochrome_png(img: &RgbImage) -> anyhow::Result<Vec<u8>> {
    let width = img.width();
    let height = img.height();

    // Each row contains 1 bit per pixel.
    // PNG rows are padded to a whole number of bytes.
    let row_bytes = (width as usize + 7) / 8;

    let mut packed = vec![0u8; row_bytes * height as usize];

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);

            let byte_index = y as usize * row_bytes + (x / 8) as usize;

            let bit_index = 7 - (x % 8);

            // PNG 1-bit grayscale:
            // 0 = black
            // 1 = white
            //
            // Set the bit for white pixels.

            let sum: usize = pixel.0.iter().map(|v| *v as usize).sum();
            if sum >= (128 * 3) {
                packed[byte_index] |= 1 << bit_index;
            }
        }
    }

    let mut buf = Vec::<u8>::new();
    let buffer = Cursor::new(&mut buf);
    let writer = BufWriter::new(buffer);

    let mut encoder = Encoder::new(writer, width, height);

    encoder.set_color(ColorType::Grayscale);
    encoder.set_depth(BitDepth::One);

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(&packed)?;
    png_writer.finish()?;

    Ok(buf)
}
