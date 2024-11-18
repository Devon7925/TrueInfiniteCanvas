use std::{cell::RefCell, rc::Rc};

use egui::{
    emath, pos2, vec2, Color32, Context, Frame, Pos2, Rect, Sense, Stroke, Ui, Vec2, Window,
};
use itertools::Itertools;

use crate::structure::{DrawNode, Line};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Painting {
    /// in 0-1 normalized coordinates
    draw_boxes: Vec<(Rc<RefCell<DrawNode>>, Pos2, u32)>,
    last_cursor_pos: Option<Pos2>,
    zoom: f32,
    pan: Vec2,
    stroke: Stroke,
    next_stroke_order: u32,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            draw_boxes: vec![(
                Rc::new(RefCell::new(DrawNode::from_corner((0, 0)))),
                pos2(0.5, 0.5),
                0,
            ),(
                Rc::new(RefCell::new(DrawNode::from_corner((1, 0)))),
                pos2(-0.5, 0.5),
                0,
            ),(
                Rc::new(RefCell::new(DrawNode::from_corner((0, 1)))),
                pos2(0.5, -0.5),
                0,
            ),(
                Rc::new(RefCell::new(DrawNode::from_corner((1, 1)))),
                pos2(-0.5, -0.5),
                0,
            )],
            last_cursor_pos: None,
            zoom: 1.0,
            pan: vec2(0.0, 0.0),
            stroke: Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
            next_stroke_order: 0,
        }
    }
}

const STANDARD_COORD_BOUNDS: Rect = Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0));

impl Painting {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label("Stroke:");
            ui.add(&mut self.stroke);
            ui.separator();
            if ui.button("Clear Painting").clicked() {
                *self = Self::default();
            }
        })
        .response
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        self.zoom *= zoom_delta;
        self.handle_zoom();
        // let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);
        // self.pan += pan_delta;
        if response.dragged() && response.dragged_by(egui::PointerButton::Middle) {
            self.pan -= response.drag_delta() / self.zoom / response.rect.size();
        }
        println!("pan: {}", self.pan);
        self.handle_pan();

        'input_handler: {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if !response.dragged_by(egui::PointerButton::Middle) {
                    let canvas_pos = pointer_pos;
                    let Some(last_cursor_pos) = self.last_cursor_pos else {
                        self.last_cursor_pos = Some(canvas_pos);
                        break 'input_handler;
                    };
                    if last_cursor_pos != canvas_pos {
                        let mut new_boxes = vec![];
                        for (node, offset, layers_above) in self.draw_boxes.iter_mut() {
                            let from_screen = emath::RectTransform::from_to(
                                response
                                    .rect
                                    .scale_from_center(
                                        self.zoom * 2.0f32.powi(*layers_above as i32),
                                    )
                                    .translate(
                                        self.zoom
                                            * (offset.to_vec2() - self.pan)
                                            * response.rect.size(),
                                    ),
                                STANDARD_COORD_BOUNDS,
                            );
                            let center = from_screen * last_cursor_pos.lerp(canvas_pos, 0.5);
                            if STANDARD_COORD_BOUNDS.contains(center) {
                                node.borrow_mut().send_stroke::<Line>(
                                    from_screen * last_cursor_pos,
                                    from_screen * canvas_pos,
                                    0.005 / self.zoom / 2.0f32.powi(*layers_above as i32),
                                    &self.stroke,
                                    self.next_stroke_order,
                                    node.clone(),
                                );
                                self.next_stroke_order += 1;

                                if *layers_above > 0 {
                                    let (child, child_offset) = node
                                        .borrow()
                                        .get_child(*layers_above, center, pos2(0.0, 0.0))
                                        .unwrap();
                                    new_boxes.push((child, *offset + child_offset.to_vec2(), 0))
                                }
                                break;
                            }
                        }
                        self.draw_boxes.extend(new_boxes);
                        self.draw_boxes.sort_by_key(|k| k.2);
                        self.last_cursor_pos = Some(canvas_pos);
                        response.mark_changed();
                    }
                } else {
                    self.last_cursor_pos = None
                }
            } else {
                self.last_cursor_pos = None
            }
        }

        for (node, offset, _) in self
            .draw_boxes
            .iter()
            .filter(|(_, _, layers_above)| *layers_above == 0)
        {
            let to_screen = emath::RectTransform::from_to(
                STANDARD_COORD_BOUNDS,
                response
                    .rect
                    .scale_from_center(self.zoom)
                    .translate(self.zoom * (offset.to_vec2() - self.pan) * response.rect.size()),
            );
            node.borrow().draw_grid(&painter, to_screen);
        }
        let mut strokes = vec![];
        for (node, offset, _) in self
            .draw_boxes
            .iter()
            .filter(|(_, _, layers_above)| *layers_above == 0)
        {
            strokes.extend(
                node.borrow().get_strokes(
                    &painter,
                    response.rect.scale_from_center(self.zoom).translate(
                        self.zoom * (offset.to_vec2() - self.pan) * response.rect.size(),
                    ),
                ),
            );
        }
        strokes.sort_by_key(|(_, order, _)| *order);
        for (stroke, _, screen_rect) in strokes {
            let to_screen = emath::RectTransform::from_to(STANDARD_COORD_BOUNDS, screen_rect);
            stroke.draw(&painter, to_screen);
        }

        response
    }

    fn handle_zoom(&mut self) {
        if self.zoom > 2.0 {
            self.zoom /= 2.0;
            self.pan *= 2.0;
            self.draw_boxes = self
                .draw_boxes
                .drain(..)
                .flat_map(|(node, pos, layers_above)| {
                    if layers_above > 0 {
                        return vec![(node, 2.0 * pos, layers_above + 1)];
                    }
                    let mut children = node
                        .borrow()
                        .children
                        .iter()
                        .enumerate()
                        .flat_map(|(y, row)| {
                            row.iter()
                                .enumerate()
                                .flat_map(|(x, child)| {
                                    child.clone().map(|child| (x, y, child.clone()))
                                })
                                .collect::<Vec<_>>()
                        })
                        .map(|(x, y, child)| (child, 2.0*pos+vec2(x as f32 - 0.5, y as f32 - 0.5), 0))
                        .collect::<Vec<_>>();

                    let still_neccessary = node
                        .borrow()
                        .children
                        .iter()
                        .enumerate()
                        .flat_map(|(y, row)| {
                            row.iter()
                                .enumerate()
                                .filter(|(_, child)| child.is_none())
                                .map(|(x, _)| (x, y))
                                .collect::<Vec<_>>()
                        })
                        .map(|(x, y)| (2.0 * pos + vec2(x as f32, y as f32)))
                        .any(|offset| (offset - vec2(0.5, 0.5)).to_vec2().abs().max_elem() <= 2.0);
                    if still_neccessary {
                        children.push((node, 2.0 * pos, 1));
                    }
                    children
                })
                .filter(|(_, offset, layers_above)| {
                    (*offset - vec2(0.5, 0.5) * 2.0f32.powi(*layers_above as i32))
                        .to_vec2()
                        .abs()
                        .max_elem()
                        <= 2.0 * 2.0f32.powi(*layers_above as i32)
                })
                .sorted_by_key(|(_, _, layers_above)| layers_above.clone())
                .collect::<Vec<_>>();
        } else if self.zoom < 0.5 {
            self.zoom *= 2.0;
            self.pan /= 2.0;
            self.draw_boxes = self
                .draw_boxes
                .drain(..)
                .map(|(node, pos, layers_above)| {
                    let corner = vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32);
                    let ref_self = node.clone();
                    if layers_above > 0 {
                        (node, pos / 2.0, layers_above - 1)
                    } else {
                        (
                            node.borrow_mut().get_or_create_parent(ref_self),
                            (pos + vec2(0.5, 0.5) - corner) / 2.0,
                            layers_above,
                        )
                    }
                })
                .unique_by(|(_, location, _)| {
                    (location.x.floor() as i32, location.y.floor() as i32)
                })
                .collect::<Vec<_>>();
        }
    }

    fn handle_pan(&mut self) {
        let mut center_changed = false;
        if self.pan.x >= 1.0 {
            self.pan.x -= 1.0;
            self.draw_boxes
                .iter_mut()
                .for_each(|(_, offset, _)| offset.x -= 1.0);
            center_changed = true;
        }
        if self.pan.x <= -1.0 {
            self.pan.x += 1.0;
            self.draw_boxes
                .iter_mut()
                .for_each(|(_, offset, _)| offset.x += 1.0);
            center_changed = true;
        }
        if self.pan.y >= 1.0 {
            self.pan.y -= 1.0;
            self.draw_boxes
                .iter_mut()
                .for_each(|(_, offset, _)| offset.y -= 1.0);
            center_changed = true;
        }
        if self.pan.y <= -1.0 {
            self.pan.y += 1.0;
            self.draw_boxes
                .iter_mut()
                .for_each(|(_, offset, _)| offset.y += 1.0);
            center_changed = true;
        }
        if center_changed {
            self.draw_boxes.retain(|(_, offset, layers_above)| {
                (*offset * 2.0f32.powi(*layers_above as i32))
                    .to_vec2()
                    .abs()
                    .max_elem()
                    <= 2.0 * 2.0f32.powi(*layers_above as i32)
            });
            self.ensure_all_loaded();
        }
    }

    fn ensure_all_loaded(&mut self) {
        for x in -2..=2 {
            let true_x = x as f32;
            for y in -2..=2 {
                let true_y = y as f32;
                self.ensure_loaded(pos2(true_x, true_y));
            }
        }
    }

    fn ensure_loaded(&mut self, query_pos: Pos2) {
        if let Some((draw_box, offset, layers_above)) = self.draw_boxes.iter().find(|(_, offset, layers_above)| (query_pos - *offset).abs().max_elem() <= 0.5 * 2.0f32.powi(*layers_above as i32)) {
            if *layers_above == 0 {
                return;
            }
            let (new_draw_box, new_pos, new_layers_above) = draw_box
                    .borrow()
                    .get_lowest_child(
                        *layers_above,
                        ((query_pos - *offset) / (0.5 * 2.0f32.powi(*layers_above as i32))).to_pos2(),
                        pos2(0.0, 0.0),
                        draw_box.clone(),
                    );
            if *layers_above == new_layers_above {
                return;
            }
            self.draw_boxes.push((new_draw_box, new_pos + offset.to_vec2(), new_layers_above));
            return;
        }

        println!("forcing child load at {}", query_pos);
        let mut current_above = 0;
        let mut parents = self
            .draw_boxes
            .iter()
            .map(|(node, pos, layers_above)| {
                let corner = vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32);
                let ref_self = node.clone();
                if *layers_above > current_above {
                    (node.clone(), *pos, *layers_above)
                } else {
                    (
                        node.borrow_mut().get_or_create_parent(ref_self),
                        (*pos + 2.0f32.powi(*layers_above as i32) * (vec2(0.5, 0.5) - corner)),
                        *layers_above + 1,
                    )
                }
            })
            .unique_by(|(_, location, _)| {
                (location.x.floor() as i32, location.y.floor() as i32)
            })
            .collect::<Vec<_>>();
        current_above += 1;

        while !(parents.iter().any(|(_, offset, layers_above)| (*offset - query_pos).abs().max_elem() <= 0.5 * 2.0f32.powi(*layers_above as i32))) {
            println!("ca: {}", current_above);
            for parent in parents.iter() {
                println!("parent: {}", parent.1)
            }
            if current_above > 20 {
                panic!()
            }
            parents = parents
                .into_iter()
                .map(|(node, pos, layers_above)| {
                    let corner = vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32);
                    let ref_self = node.clone();
                    if layers_above > current_above {
                        (node.clone(), pos, layers_above)
                    } else {
                        (
                            node.borrow_mut().get_or_create_parent(ref_self),
                            (pos + 2.0f32.powi(layers_above as i32) * (vec2(0.5, 0.5) - corner)),
                            layers_above + 1,
                        )
                    }
                })
                .unique_by(|(_, location, _)| {
                    (location.x.floor() as i32, location.y.floor() as i32)
                })
                .collect::<Vec<_>>();
            current_above += 1;
        }
        let parent = parents.iter().find(|(_, offset, layers_above)| (query_pos - *offset).abs().max_elem() <= 0.5 * 2.0f32.powi(*layers_above as i32));
        
        let (parent_box, parent_offset, parent_layers_above) = parent.unwrap();

        let (new_draw_box, new_pos, new_layers_above) = parent_box.borrow().get_lowest_child(
            *parent_layers_above,
            ((query_pos - *parent_offset) / (0.5 * 2.0f32.powi(*parent_layers_above as i32))).to_pos2(),
            pos2(0.0, 0.0),
            parent_box.clone(),
        );
        self.draw_boxes.push((new_draw_box, new_pos + parent_offset.to_vec2(), new_layers_above));
    }
}
