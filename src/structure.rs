use std::{cell::RefCell, rc::Rc, sync::Arc};

use egui::{emath::RectTransform, pos2, vec2, Color32, Painter, Pos2, Rect, Stroke, Ui};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DrawNode {
    pub parent: Option<Rc<RefCell<DrawNode>>>,
    pub children: [[Option<Rc<RefCell<DrawNode>>>; 2]; 2],
    strokes: Vec<(Box<dyn CanvasDrawable>, u32)>,
    pub corner: (u8, u8),
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
    pub fn from_corner(corner: (u8, u8)) -> Self {
        Self {
            parent: None,
            children: [(); 2].map(|_| [(); 2].map(|_| None)),
            strokes: vec![],
            corner,
        }
    }

    pub fn get_strokes(&self, painter: &Painter, screen_rect: Rect) -> Vec<(Box<dyn CanvasDrawable>, u32, Rect)> {
        let inner_to_rect = screen_rect.scale_from_center(0.5);
        let mut strokes = self.strokes.iter().map(|(stroke, order)| (stroke.clone(), *order, screen_rect.clone())).collect_vec();
        for y in 0..=1 {
            for x in 0..=1 {
                if self.children[y][x].is_none() {
                    continue;
                };

                strokes.extend(self.children[y][x].as_ref().unwrap().borrow().get_strokes(
                    painter,
                        inner_to_rect.translate(vec2(
                            (x as f32 - 0.5) * 0.5 * screen_rect.width(),
                            (y as f32 - 0.5) * 0.5 * screen_rect.height(),
                        )),
                ));
            }
        }

        strokes
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

    pub fn send_stroke<T: CanvasDrawableGenerator + 'static>(&mut self, p1: Pos2, p2: Pos2, scale: f32, stroke: &Stroke, order: u32, ref_self: Rc<RefCell<DrawNode>>) {
        if (p1-p2).abs().max_elem() >= 0.5 {
            self.strokes.push((T::from_points(p1, p2, scale, stroke), order));
            return;
        }
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
            self.children[y][x].as_mut().unwrap().borrow_mut().parent = Some(ref_self);
            self.children[y][x].as_mut().unwrap().borrow_mut().corner = (x as u8, y as u8);
        }
        let ref_child = self.children[y][x].as_ref().unwrap().clone();
        self.children[y][x].as_mut().unwrap().borrow_mut().send_stroke::<T>(new_p1, new_p2, 2.0 * scale, stroke, order, ref_child);
    }

    pub fn get_or_create_parent(&mut self, ref_self: Rc<RefCell<DrawNode>>) -> Rc<RefCell<DrawNode>> {
        if let Some(parent) = self.parent.as_ref() {
            return parent.clone();
        }
        let mut parent = DrawNode::default();
        parent.corner = self.corner;
        parent.children[self.corner.1 as usize][self.corner.0 as usize] = Some(ref_self);
        self.parent = Some(Rc::new(RefCell::new(parent)));
        return self.parent.as_ref().unwrap().clone();
    }

    pub fn get_child(&self, layers: u32, pos: Pos2, offset: Pos2) -> Option<(Rc<RefCell<DrawNode>>, Pos2)> {
        let x = if pos.x > 0.0 {1} else {0};
        let y = if pos.y > 0.0 {1} else {0};
        let mut new_pos = pos;
        let new_offset = 2.0*offset + vec2(x as f32, y as f32)-vec2(0.5, 0.5);
        if x == 0 {
            new_pos.x = pos.x + 0.5;
        } else {
            new_pos.x = pos.x - 0.5;
        }
        if y == 0 {
            new_pos.y = pos.y + 0.5;
        } else {
            new_pos.y = pos.y - 0.5;
        }
        new_pos = 2.0 * new_pos;
        if self.children[y][x].is_none() {
            return None
        }
        let ref_child = self.children[y][x].as_ref().unwrap().clone();
        if layers == 1 {
            return Some((ref_child, new_offset));
        }
        self.children[y][x].as_ref().unwrap().borrow().get_child(layers - 1, new_pos, new_offset)
    }

    pub fn get_lowest_child(&self, layers: u32, pos: Pos2, offset: Pos2, ref_self: Rc<RefCell<DrawNode>>) -> (Rc<RefCell<DrawNode>>, Pos2, u32) {
        if layers == 0 {
            return (ref_self, offset, layers)
        }
        let x = if pos.x > 0.0 {1} else {0};
        let y = if pos.y > 0.0 {1} else {0};
        let mut new_pos = pos;
        let new_offset = 2.0*offset + vec2(x as f32, y as f32)-vec2(0.5, 0.5);
        if x == 0 {
            new_pos.x = pos.x + 0.5;
        } else {
            new_pos.x = pos.x - 0.5;
        }
        if y == 0 {
            new_pos.y = pos.y + 0.5;
        } else {
            new_pos.y = pos.y - 0.5;
        }
        new_pos = 2.0 * new_pos;
        if self.children[y][x].is_none() {
            return (ref_self, offset * 2.0f32.powi(layers as i32), layers)
        }
        let ref_child = self.children[y][x].as_ref().unwrap().clone();
        self.children[y][x].as_ref().unwrap().borrow().get_lowest_child(layers - 1, new_pos, new_offset, ref_child)
    }
}

#[allow(private_bounds)]
pub trait CanvasDrawableGenerator: CanvasDrawable {
    fn from_points(p1: Pos2, p2: Pos2, scale: f32, stroke: &Stroke) -> Box<Self>;
}

#[typetag::serde(tag = "type")]
pub trait CanvasDrawable {
    fn draw(&self, painter: &Painter, to_screen: RectTransform);
    fn box_clone(&self) -> Box<dyn CanvasDrawable>;
}

impl Clone for Box<dyn CanvasDrawable>
{
    fn clone(&self) -> Box<dyn CanvasDrawable> {
        self.box_clone()
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Line {
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    stroke: Stroke,
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
            Stroke::new(self.stroke.width * scale_factor, self.stroke.color),
        );
    }

    fn box_clone(&self) -> Box<dyn CanvasDrawable> {
        Box::new((*self).clone())
    }
}

impl CanvasDrawableGenerator for Line {
    fn from_points(p1: Pos2, p2: Pos2, scale: f32, stroke: &Stroke) -> Box<Self> {
        Box::new(Line {
            start_x: p1.x,
            start_y: p1.y,
            end_x: p2.x,
            end_y: p2.y,
            stroke: Stroke::new(stroke.width * scale, stroke.color),
        })
    }
}
