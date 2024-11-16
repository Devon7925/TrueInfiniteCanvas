use std::{cell::RefCell, rc::Rc, sync::Arc};

use egui::{emath::RectTransform, pos2, vec2, Color32, Painter, Pos2, Rect, Stroke, Ui};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DrawNode {
    parent: Option<Rc<RefCell<DrawNode>>>,
    pub children: [[Option<Rc<RefCell<DrawNode>>>; 2]; 2],
    strokes: Vec<Box<dyn CanvasDrawable>>,
    corner: (u8, u8),
}

impl Default for DrawNode {
    fn default() -> Self {
        Self {
            parent: None,
            children: [(); 2].map(|_| [(); 2].map(|_| None)),
            strokes: vec![],
            corner: (0, 0),
        }
    }
}

impl DrawNode {
    pub fn draw(&self, painter: &Painter, to_screen: RectTransform) {
        let inner_to_rect = to_screen.to().scale_from_center(0.5);
        for y in 0..=1 {
            for x in 0..=1 {
                if self.children[y][x].is_none() {
                    continue;
                };

                self.children[y][x].as_ref().unwrap().borrow().draw(
                    painter,
                    RectTransform::from_to(
                        to_screen.from().clone(),
                        inner_to_rect.translate(vec2(
                            (x as f32 - 0.5) * 0.5 * to_screen.to().width(),
                            (y as f32 - 0.5) * 0.5 * to_screen.to().height(),
                        )),
                    ),
                );
            }
        }

        for stroke in self.strokes.iter() {
            stroke.draw(painter, to_screen);
        }
    }

    pub fn draw_grid(&self, painter: &Painter, to_screen: RectTransform) {
        let inner_to_rect = to_screen.to().scale_from_center(0.5);
        for y in 0..=1 {
            for x in 0..=1 {
                if self.children[y][x].is_none() {
                    continue;
                };

                self.children[y][x].as_ref().unwrap().borrow().draw_grid(
                    painter,
                    RectTransform::from_to(
                        to_screen.from().clone(),
                        inner_to_rect.translate(vec2(
                            (x as f32 - 0.5) * 0.5 * to_screen.to().width(),
                            (y as f32 - 0.5) * 0.5 * to_screen.to().height(),
                        )),
                    ),
                );
            }
        }

        painter.rect_stroke(Rect::from_min_max(to_screen * pos2(-1.0, -1.0), to_screen * pos2(1.0, 1.0)), 0.0, Stroke::new(2.0, Color32::BLACK));
    }

    pub fn send_stroke<T: CanvasDrawableGenerator + 'static>(&mut self, p1: Pos2, p2: Pos2, scale: f32, ref_self: Rc<RefCell<DrawNode>>) {
        if (p1-p2).abs().max_elem() < 0.5 {
            let center = p1.lerp(p2, 0.5);
            let x = if center.x > 0.0 {1} else {0};
            let y = if center.y > 0.0 {1} else {0};
            let mut new_p1 = p1;
            let mut new_p2 = p2;
            if x == 0 {
                new_p1.x = p1.x + 0.5;
                new_p2.x = p2.x + 0.5;
            } else {
                new_p1.x = p1.x - 0.5;
                new_p2.x = p2.x - 0.5;
            }
            if y == 0 {
                new_p1.y = p1.y + 0.5;
                new_p2.y = p2.y + 0.5;
            } else {
                new_p1.y = p1.y - 0.5;
                new_p2.y = p2.y - 0.5;
            }
            new_p1 = 2.0 * new_p1;
            new_p2 = 2.0 * new_p2;
            if self.children[y][x].is_none() {
                self.children[y][x] = Some(Rc::new(RefCell::new(DrawNode::default())));
            }
            self.children[y][x].as_mut().unwrap().borrow_mut().parent = Some(ref_self);
            let ref_child = self.children[y][x].as_ref().unwrap().clone();
            self.children[y][x].as_mut().unwrap().borrow_mut().send_stroke::<T>(new_p1, new_p2, 2.0 * scale, ref_child);
            return;
        }

        self.strokes.push(T::from_points(p1, p2, scale));
    }
}

#[allow(private_bounds)]
pub trait CanvasDrawableGenerator: CanvasDrawable {
    fn from_points(p1: Pos2, p2: Pos2, scale: f32) -> Box<Self>;
}

#[typetag::serde(tag = "type")]
trait CanvasDrawable {
    fn draw(&self, painter: &Painter, to_screen: RectTransform);
}

#[derive(Deserialize, Serialize)]
pub struct Line {
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    stroke: f32,
}

#[typetag::serde]
impl CanvasDrawable for Line {
    fn draw(&self, painter: &Painter, to_screen: RectTransform) {
        let scale_factor = to_screen.scale().max_elem();
        painter.line_segment(
            [
                to_screen * pos2(self.start_x, self.start_y),
                to_screen * pos2(self.end_x, self.end_y),
            ],
            Stroke::new(self.stroke * scale_factor, Color32::GRAY),
        );
    }
}

impl CanvasDrawableGenerator for Line {
    fn from_points(p1: Pos2, p2: Pos2, scale: f32) -> Box<Self> {
        Box::new(Line {
            start_x: p1.x,
            start_y: p1.y,
            end_x: p2.x,
            end_y: p2.y,
            stroke: scale,
        })
    }
}
