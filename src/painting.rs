use std::{cell::RefCell, rc::Rc};

use egui::{
    emath, pos2, vec2, Color32, Context, Frame, Pos2, Rect, Sense, Stroke, Ui, Vec2, Window,
};

use crate::structure::{DrawNode, Line};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Painting {
    /// in 0-1 normalized coordinates
    draw_boxes: Vec<(Rc<RefCell<DrawNode>>, Pos2)>,
    last_cursor_pos: Option<Pos2>,
    zoom: f32,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            draw_boxes: vec![(Rc::new(RefCell::new(DrawNode::default())), pos2(0.0, 0.0))],
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
                .flat_map(|(node, pos)| {
                    node.borrow()
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
                        .map(|(x, y, child)| (child, 2.0*pos+vec2(x as f32 - 0.5, y as f32 - 0.5)))
                        .collect::<Vec<_>>()
                })
                .filter(|(_, offset)| offset.x >= -1.5 && offset.x <= 2.5 && offset.y >= -1.5 && offset.y <= 2.5)
                .collect::<Vec<_>>();
        } else if self.zoom < 0.5 {
            self.zoom = 0.5;
        }

        'input_handler: {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let canvas_pos = pointer_pos;
                let Some(last_cursor_pos) = self.last_cursor_pos else {
                    self.last_cursor_pos = Some(canvas_pos);
                    break 'input_handler;
                };
                if last_cursor_pos != canvas_pos {
                    for (node, offset) in self.draw_boxes.iter_mut() {
                        let from_screen = emath::RectTransform::from_to(
                            response
                                .rect
                                .scale_from_center(self.zoom)
                                .translate(self.zoom * offset.to_vec2() * response.rect.size()),
                            STANDARD_COORD_BOUNDS,
                        );
                        let center = from_screen * last_cursor_pos.lerp(canvas_pos, 0.5);
                        if STANDARD_COORD_BOUNDS.contains(center) {
                            node.borrow_mut().send_stroke::<Line>(
                                from_screen * last_cursor_pos,
                                from_screen * canvas_pos,
                                0.005 / self.zoom,
                                node.clone(),
                            );
                        }
                    }
                    self.last_cursor_pos = Some(canvas_pos);
                    response.mark_changed();
                }
            } else {
                self.last_cursor_pos = None
            }
        }

        for (node, offset) in self.draw_boxes.iter() {
            let to_screen = emath::RectTransform::from_to(
                STANDARD_COORD_BOUNDS,
                response
                    .rect
                    .scale_from_center(self.zoom)
                    .translate(self.zoom * offset.to_vec2() * response.rect.size()),
            );
            node.borrow().draw_grid(&painter, to_screen);
            node.borrow().draw(&painter, to_screen);
        }

        response
    }
}
