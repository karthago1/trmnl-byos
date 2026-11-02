use std::collections::HashMap;

use image::Pixel;
use imageproc::{
    definitions::Clamp,
    drawing::{Canvas, draw_filled_rect_mut},
    rect::Rect,
};
use serde::Deserialize;

use crate::ui::{Area, Item, Position, Size, Text, Value, call_pre_draw};

#[derive(Deserialize, Clone, Debug)]
pub struct TextTable {
    #[serde(flatten)]
    pub pos: Position,
    pub columns: usize,
    pub rows: usize,
    pub items: Vec<String>,
    pub spacing: u32,
    pub font_size: f32,
    table: Option<Table>,
}
impl TextTable {
    pub(crate) fn draw<C>(&self, area: &Area, brush: <C as Canvas>::Pixel, image: &mut C)
    where
        C: Canvas,
        <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
    {
        self.table.as_ref().unwrap().draw(area, brush, image);
    }

    pub(crate) fn pre_draw(&mut self, data: &HashMap<String, String>) {
        if self.table.is_none() {
            self.table = Some(Table {
                pos: self.pos.clone(),
                columns: self.columns,
                rows: self.rows,
                spacing: self.spacing,
                items: self
                    .items
                    .iter_mut()
                    .map(|text| {
                        if text.is_empty() {
                            Item::Empty
                        } else {
                            Text::new(
                                std::mem::take(text),
                                Position {
                                    x: Value::Center,
                                    y: Value::Center,
                                },
                                self.font_size,
                            )
                            .into()
                        }
                    })
                    .collect(),
            })
        }

        self.table.as_mut().unwrap().pre_draw(data);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Table {
    #[serde(flatten)]
    pub pos: Position,
    // pub height: Value,
    // pub width: Value,
    pub columns: usize,
    pub rows: usize,
    pub items: Vec<Item>,
    pub spacing: u32,
}

impl Table {
    pub fn pre_draw(&mut self, data: &HashMap<String, String>) {
        call_pre_draw(&mut self.items, data);
    }

    pub fn draw<C>(&self, area: &Area, brush: C::Pixel, image: &mut C)
    where
        C: Canvas,
        <C::Pixel as Pixel>::Subpixel: Into<f32> + Clamp<f32>,
    {
        let mut col_widths = vec![0u32; self.columns];
        let mut row_height = vec![0u32; self.rows];

        for (i, item) in self.items.iter().enumerate() {
            let row = i / self.columns;
            let col = i % self.columns;
            let s = item.size();
            col_widths[col] = s.width.max(col_widths[col]);
            row_height[row] = s.height.max(row_height[row]);
        }

        let own_size = Size {
            width: col_widths.iter().sum::<u32>()
                + self.spacing * self.columns.saturating_sub(1) as u32,
            height: row_height.iter().sum::<u32>()
                + self.spacing * self.rows.saturating_sub(1) as u32,
        };

        let (x, y) = self.pos.get(area, &own_size);

        let half_spacing = self.spacing >> 1;
        let mut i = 0;
        let mut item_y = y;
        for row in 0..self.rows {
            let mut item_x = x;
            for col in 0..self.columns {
                let item = &self.items[i];
                let area = Area {
                    x: item_x,
                    y: item_y,
                    width: col_widths[col],
                    height: row_height[row],
                };

                item.draw(&area, brush, image);
                println!("{area:?}");
                item_x += area.width + self.spacing;
                i += 1;
            }
            item_y += row_height[row] + half_spacing;

            let rect = Rect::at(x as i32, item_y as i32).of_size(own_size.width, 2);
            draw_filled_rect_mut(image, rect, brush);
            item_y += half_spacing;
        }

        let mut item_x = x;
        for col in 0..self.columns.saturating_sub(1) {
            item_x += col_widths[col] + half_spacing;
            let rect = Rect::at(item_x as i32, y as i32).of_size(2, own_size.height);
            draw_filled_rect_mut(image, rect, brush);
            item_x += half_spacing;
        }
    }
}
