use std::{cell::RefCell, rc::Rc};

use egui::{emath, pos2, vec2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    circular_buffer::CircularBuffer2D,
    structure::{DrawNode, DrawNodeRef, Line},
};

#[derive(Deserialize, Serialize)]
pub struct Painting {
    #[serde(serialize_with = "structure_serializer")]
    #[serde(deserialize_with = "structure_deserializer")]
    draw_boxes: CircularBuffer2D<Rc<RefCell<DrawNode>>, 5>,
    last_cursor_pos: Option<Pos2>,
    zoom: f32,
    pan: Vec2,
    stroke: Stroke,
    next_stroke_order: u32,
    debug_render: bool,
}

#[derive(Deserialize, Serialize)]
struct CircularBufferSerialization {
    center_path: Vec<(u8, u8)>,
    top_level_parent: DrawNodeRef,
}

fn structure_serializer<S>(
    structure: &CircularBuffer2D<Rc<RefCell<DrawNode>>, 5>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let Some(center_cell) = structure.get(0, 0) else {
        panic!("No center cell to serialize from");
    };
    let (top_level, path) = DrawNode::get_top_level_and_path(vec![], center_cell.clone());
    CircularBufferSerialization {
        center_path: path,
        top_level_parent: DrawNodeRef(top_level),
    }
    .serialize(serializer)
}

fn structure_deserializer<'de, D>(
    deserializer: D,
) -> Result<CircularBuffer2D<Rc<RefCell<DrawNode>>, 5>, D::Error>
where
    D: Deserializer<'de>,
{
    let serialization = CircularBufferSerialization::deserialize(deserializer)?;
    let mut draw_boxes = CircularBuffer2D::<Rc<RefCell<DrawNode>>, 5>::default();
    let mut center_path = serialization.center_path;
    let center = serialization
        .top_level_parent
        .0
        .borrow()
        .follow_path(&mut center_path, serialization.top_level_parent.0.clone());
    unsafe {
        let ptr = Rc::into_raw(serialization.top_level_parent.0.clone());
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr);
    }
    draw_boxes.set(0, 0, center);
    draw_boxes.load_all();
    Ok(draw_boxes)
}

impl Default for Painting {
    fn default() -> Self {
        let mut draw_boxes = CircularBuffer2D::<Rc<RefCell<DrawNode>>, 5>::default();
        draw_boxes.set(0, 0, DrawNode::top_level());
        draw_boxes.load_all();
        Self {
            draw_boxes,
            last_cursor_pos: None,
            zoom: 1.0,
            pan: vec2(0.0, 0.0),
            stroke: Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
            next_stroke_order: 0,
            debug_render: false,
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
            ui.checkbox(&mut self.debug_render, "Debug render");
            if ui.button("Export").clicked() {
                let mut out = Vec::new();
                let mut serializer = ron::ser::Serializer::with_options(
                    &mut out,
                    None,
                    ron::Options::default().without_recursion_limit(),
                )
                .unwrap();
                let serializer = serde_stacker::Serializer::new(&mut serializer);
                let export = match self.serialize(serializer) {
                    Ok(_) => String::from_utf8(out).expect("Ron should be utf-8"),
                    Err(err) => panic!("eframe failed to encode data using ron: {}", err),
                };
                ui.output_mut(|output| output.copied_text = export);
            }
            if ui.button("Import").clicked() {
                let clipboard = get_clipboard();
                let mut deserializer = ron::de::Deserializer::from_str_with_options(
                    &clipboard,
                    ron::Options::default().without_recursion_limit(),
                )
                .unwrap();
                let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
                println!("Trying import");
                match Painting::deserialize(deserializer) {
                    Ok(value) => {
                        println!("Successful import");
                        *self = value;
                    }
                    Err(err) => {
                        // This happens on when we break the format, e.g. when updating egui.
                        log::debug!("Failed to decode RON: {err}");
                        eprintln!("Failed to decode RON: {err}");
                    }
                };
            }
        })
        .response
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

        let drag_input = response.dragged_by(egui::PointerButton::Middle)
            || response.drag_started_by(egui::PointerButton::Middle);
        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            let from_screen = emath::RectTransform::from_to(
                response
                    .rect
                    .scale_from_center(5.0 * self.zoom)
                    .translate(self.zoom * -self.pan * response.rect.size()),
                5.0 / 2.0 * STANDARD_COORD_BOUNDS,
            );
            let transformed_pointer_pos = from_screen * pointer;
            let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
            self.pan += (zoom_delta - 1.0) * (transformed_pointer_pos - self.pan).to_vec2();
            self.zoom *= zoom_delta;
        }
        if response.dragged() && drag_input {
            self.pan -= response.drag_delta() / self.zoom / response.rect.size();
        }
        let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);
        self.pan -= pan_delta / self.zoom / response.rect.size();
        self.handle_pan_zoom();

        'input_handler: {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if response.drag_started_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary)
                {
                    let canvas_pos = pointer_pos;
                    let Some(last_cursor_pos) = self.last_cursor_pos else {
                        self.last_cursor_pos = Some(canvas_pos);
                        break 'input_handler;
                    };
                    if last_cursor_pos != canvas_pos {
                        let from_screen = emath::RectTransform::from_to(
                            response
                                .rect
                                .scale_from_center(5.0 * self.zoom)
                                .translate(self.zoom * -self.pan * response.rect.size()),
                            5.0 / 2.0 * STANDARD_COORD_BOUNDS,
                        );
                        let center = from_screen * last_cursor_pos.lerp(canvas_pos, 0.5);
                        let x = center.x.round() as i32;
                        let y = center.y.round() as i32;
                        let Some(node) = self.draw_boxes.get(x, y) else {
                            break 'input_handler;
                        };
                        let p1 = 2.0 * (from_screen * last_cursor_pos - vec2(x as f32, y as f32));
                        let p2 = 2.0 * (from_screen * canvas_pos - vec2(x as f32, y as f32));
                        let p1 = p1 / 2.0
                            + vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32)
                            - vec2(0.5, 0.5);
                        let p2 = p2 / 2.0
                            + vec2(node.borrow().corner.0 as f32, node.borrow().corner.1 as f32)
                            - vec2(0.5, 0.5);
                        let parent = node.borrow_mut().get_or_create_parent(node.clone());
                        parent.borrow_mut().send_stroke::<Line>(
                            p1,
                            p2,
                            0.005 / self.zoom,
                            &self.stroke,
                            self.next_stroke_order,
                            node.clone(),
                        );
                        self.next_stroke_order += 1;
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

        if self.debug_render {
            for (x, y, node) in self.draw_boxes.cells() {
                let offset = vec2(x as f32, y as f32);
                let to_screen = emath::RectTransform::from_to(
                    STANDARD_COORD_BOUNDS,
                    response
                        .rect
                        .scale_from_center(self.zoom)
                        .translate(self.zoom * (offset - self.pan) * response.rect.size()),
                );
                node.borrow().draw_grid(&painter, to_screen);
            }
        }
        let mut strokes = vec![];
        for (x, y, node) in self.draw_boxes.cells() {
            let offset = vec2(x as f32, y as f32);
            strokes.extend(
                node.borrow().get_strokes(
                    response
                        .rect
                        .scale_from_center(self.zoom)
                        .translate(self.zoom * (offset - self.pan) * response.rect.size()),
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

    fn handle_pan_zoom(&mut self) {
        let mut changed = false;

        if self.zoom > 2.0 {
            self.zoom /= 2.0;
            self.pan *= 2.0;
            let corner = (
                if self.pan.x > 0.0 { 1 } else { 0 },
                if self.pan.y > 0.0 { 1 } else { 0 },
            );
            self.pan.x -= corner.0 as f32 - 0.5;
            self.pan.y -= corner.1 as f32 - 0.5;
            self.draw_boxes.zoom_in(corner);
            changed = true;
        } else if self.zoom < 0.5 {
            self.zoom *= 2.0;
            let center_corner = self.draw_boxes.get(0, 0).unwrap().borrow().corner;
            self.pan.x += center_corner.0 as f32 - 0.5;
            self.pan.y += center_corner.1 as f32 - 0.5;
            self.pan /= 2.0;
            self.draw_boxes.zoom_out();
            changed = true;
        }
        if self.pan.x >= 1.0 {
            self.pan.x -= 1.0;
            self.draw_boxes.shift_pos_x();
            changed = true;
        }
        if self.pan.x <= -1.0 {
            self.pan.x += 1.0;
            self.draw_boxes.shift_neg_x();
            changed = true;
        }
        if self.pan.y >= 1.0 {
            self.pan.y -= 1.0;
            self.draw_boxes.shift_pos_y();
            changed = true;
        }
        if self.pan.y <= -1.0 {
            self.pan.y += 1.0;
            self.draw_boxes.shift_neg_y();
            changed = true;
        }
        if changed {
            self.draw_boxes.load_all();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_clipboard() -> String {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let ctx = ClipboardContext::new().unwrap();
    ctx.get_text().unwrap_or("".to_string())
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn get_clipboard() -> String {
    "".to_string()
}
