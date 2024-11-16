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
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            draw_boxes: vec![(Rc::new(RefCell::new(DrawNode::default())), pos2(0.0, 0.0), 0)],
            last_cursor_pos: None,
            zoom: 1.0,
        }
    }
}

const STANDARD_COORD_BOUNDS: Rect = Rect::from_min_max(pos2(-1.0, -1.0), pos2(1.0, 1.0));

impl Painting {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            if ui.button("Clear Painting").clicked() {
                *self = Self::default();
            }
        })
        .response
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        self.zoom *= zoom_delta;
        if self.zoom > 2.0 {
            self.zoom /= 2.0;
            self.draw_boxes = self
                .draw_boxes
                .drain(..)
                .flat_map(|(node, pos, layers_above)| {
                    if layers_above > 0 {
                        return vec![
                            (node, 2.0 * pos, layers_above + 1)
                        ];
                    }
                    let mut children = node.borrow()
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
                    
                    let still_neccessary = node.borrow()
                        .children
                        .iter()
                        .enumerate()
                        .flat_map(|(y, row)| {
                            row.iter()
                                .enumerate()
                                .filter(|(_, child)| child.is_none())
                                .map(|(x, _)| {
                                    (x, y)
                                })
                                .collect::<Vec<_>>()
                        })
                        .map(|(x, y)| (2.0*pos+vec2(x as f32 - 0.5, y as f32 - 0.5)))
                        .any(|offset| (offset - vec2(0.5, 0.5)).to_vec2().abs().max_elem() <= 2.0);
                    if still_neccessary {
                        children.push((node, 2.0 * pos, 1));
                    }
                    children
                })
                .filter(|(_, offset, layers_above)| (*offset - vec2(0.5, 0.5) * 2.0f32.powi(*layers_above as i32)).to_vec2().abs().max_elem() <= 2.0 * 2.0f32.powi(*layers_above as i32))
                .sorted_by_key(|(_, _, layers_above)| layers_above.clone())
                .collect::<Vec<_>>();
        } else if self.zoom < 0.5 {
            self.zoom *= 2.0;
            self.draw_boxes = self
                .draw_boxes
                .drain(..)
                .map(|(node, pos, layers_above)| {
                    let corner = vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32);
                    let ref_self = node.clone();
                    if layers_above > 0 {
                        (
                            node, 
                            pos / 2.0,
                            layers_above - 1,
                        )
                    } else {
                        (
                            node.borrow_mut().get_or_create_parent(ref_self), 
                            (pos + vec2(0.5, 0.5) - corner)/2.0,
                            layers_above,
                        )
                    }
                })
                .unique_by(|(_, location, _)| (location.x.floor() as i32, location.y.floor() as i32))
                .collect::<Vec<_>>();
        }

        'input_handler: {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
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
                                .scale_from_center(self.zoom * 2.0f32.powi(*layers_above as i32))
                                .translate(self.zoom * offset.to_vec2() * response.rect.size()),
                            STANDARD_COORD_BOUNDS,
                        );
                        let center = from_screen * last_cursor_pos.lerp(canvas_pos, 0.5);
                        if STANDARD_COORD_BOUNDS.contains(center) {
                            println!("above: {layers_above}, offset: {offset}");
                            node.borrow_mut().send_stroke::<Line>(
                                from_screen * last_cursor_pos,
                                from_screen * canvas_pos,
                                0.005 / self.zoom / 2.0f32.powi(*layers_above as i32),
                                node.clone(),
                            );

                            if *layers_above > 0 {
                                let (child, child_offset) = node.borrow().get_child(*layers_above, center, pos2(0.0, 0.0)).unwrap();
                                println!("offset: {child_offset}");
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
        }

        for (node, offset, _) in self.draw_boxes.iter().filter(|(_, _, layers_above)| *layers_above == 0) {
            let to_screen = emath::RectTransform::from_to(
                STANDARD_COORD_BOUNDS,
                response
                    .rect
                    .scale_from_center(self.zoom)
                    .translate(self.zoom * offset.to_vec2() * response.rect.size()),
            );
            node.borrow().draw_grid(&painter, to_screen);
        }
        for (node, offset, _) in self.draw_boxes.iter().filter(|(_, _, layers_above)| *layers_above == 0) {
            let to_screen = emath::RectTransform::from_to(
                STANDARD_COORD_BOUNDS,
                response
                    .rect
                    .scale_from_center(self.zoom)
                    .translate(self.zoom * offset.to_vec2() * response.rect.size()),
            );
            node.borrow().draw(&painter, to_screen);
        }

        response
    }
}
