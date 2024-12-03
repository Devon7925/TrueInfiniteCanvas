use std::{cell::RefCell, rc::Rc};

use crate::structure::{Direction, DrawNode};

pub struct CircularBuffer2D<T, const N: usize> {
    data: [[Option<T>; N]; N],
    offset: (usize, usize),
}

impl<T, const N: usize> Default for CircularBuffer2D<T, N> {
    fn default() -> Self {
        Self {
            data: [(); N].map(|_| [(); N].map(|_| None)),
            offset: (0, 0),
        }
    }
}

impl<T: Cleanupable, const N: usize> CircularBuffer2D<T, N> {
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        if x < -(N as i32) / 2 || x > N as i32 / 2 {
            panic!()
        }
        if y < -(N as i32) / 2 || y > N as i32 / 2 {
            panic!()
        }
        self.data[((x + N as i32 / 2) as usize + self.offset.0) % N]
            [((y + N as i32 / 2) as usize + self.offset.1) % N]
            .as_ref()
    }

    pub fn set(&mut self, x: i32, y: i32, obj: T) {
        self.clear(x, y);
        self.data[((x + N as i32 / 2) as usize + self.offset.0) % N]
            [((y + N as i32 / 2) as usize + self.offset.1) % N] = Some(obj);
    }

    pub fn clear(&mut self, x: i32, y: i32) {
        self.deallocate(
            ((x + N as i32 / 2) as usize + self.offset.0) % N,
            ((y + N as i32 / 2) as usize + self.offset.1) % N,
        );
    }

    pub fn clear_all(&mut self) {
        for x in -(N as i32) / 2..=(N as i32) / 2 {
            for y in -(N as i32) / 2..=(N as i32) / 2 {
                self.clear(x, y);
            }
        }
    }
    pub fn cells(&self) -> Vec<(i32, i32, &T)> {
        let mut cells = vec![];
        for x in -(N as i32) / 2..=(N as i32) / 2 {
            for y in -(N as i32) / 2..=(N as i32) / 2 {
                if let Some(cell) = self.get(x, y) {
                    cells.push((x, y, cell));
                }
            }
        }
        cells
    }

    pub fn shift_pos_x(&mut self) {
        self.offset.0 = (self.offset.0 + 1) % N;
        for y in 0..N {
            self.deallocate((N - 1 + self.offset.0) % N, y);
        }
    }

    pub fn shift_neg_x(&mut self) {
        self.offset.0 = (self.offset.0 + N - 1) % N;
        for y in 0..N {
            self.deallocate(self.offset.0, y);
        }
    }

    pub fn shift_pos_y(&mut self) {
        self.offset.1 = (self.offset.1 + 1) % N;
        for x in 0..N {
            self.deallocate(x, (N - 1 + self.offset.1) % N);
        }
    }

    pub fn shift_neg_y(&mut self) {
        self.offset.1 = (self.offset.1 + N - 1) % N;
        for x in 0..N {
            self.deallocate(x, self.offset.1);
        }
    }

    fn deallocate(&mut self, x: usize, y: usize) {
        if let Some(ref mut data) = self.data[x][y] {
            data.cleanup();
        }
        self.data[x][y] = None;
    }
}

pub trait Cleanupable {
    fn cleanup(&mut self);
}

impl Cleanupable for Rc<RefCell<DrawNode>> {
    fn cleanup(&mut self) {
        self.borrow_mut().try_cleanup();
    }
}

impl<const N: usize> CircularBuffer2D<Rc<RefCell<DrawNode>>, N> {
    pub fn zoom_in(&mut self, corner: (u8, u8)) {
        let mut new_data = [(); N].map(|_| [(); N].map(|_| None));
        for x in -(N as i32) / 2..=(N as i32) / 2 {
            for y in -(N as i32) / 2..=(N as i32) / 2 {
                let zoomed_out_node = self.get(
                    ((x as f32 + corner.0 as f32) / 2.0).floor() as i32,
                    ((y as f32 + corner.1 as f32) / 2.0).floor() as i32,
                );
                let corner = (
                    ((x + 2 * N as i32) as u8 + corner.0) % 2,
                    ((y + 2 * N as i32) as u8 + corner.1) % 2,
                );
                let new_node = zoomed_out_node.map(|node| {
                    node.borrow_mut()
                        .get_or_create_child_from_corner(corner, node.clone())
                });
                new_data[(x + N as i32 / 2) as usize][(y + N as i32 / 2) as usize] = new_node;
            }
        }
        self.clear_all();
        self.data = new_data;
        self.offset = (0, 0);
    }

    pub fn zoom_out(&mut self) {
        let mut new_data = [(); N].map(|_| [(); N].map(|_| None));
        for x in -(N as i32) / 4..=(N as i32) / 4 {
            for y in -(N as i32) / 4..=(N as i32) / 4 {
                let Some(node) = self.get(2 * x, 2 * y) else {
                    continue;
                };
                let parent = node.borrow_mut().get_or_create_parent(node.clone());
                new_data[(x + N as i32 / 2) as usize][(y + N as i32 / 2) as usize] = Some(parent);
            }
        }
        self.clear_all();
        self.data = new_data;
        self.offset = (0, 0);
    }

    pub fn load_all(&mut self) {
        for x in -(N as i32) / 2..=(N as i32) / 2 {
            for y in -(N as i32) / 2..=(N as i32) / 2 {
                if self.get(x, y).is_some() {
                    continue;
                }
                if x > -(N as i32) / 2 {
                    if let Some(left_node) = self.get(x - 1, y).cloned() {
                        let neighbor = left_node
                            .borrow_mut()
                            .get_or_create_neighbor(Direction::PosX, left_node.clone());
                        self.set(x, y, neighbor);
                        continue;
                    }
                }
                if y > -(N as i32) / 2 {
                    if let Some(above_node) = self.get(x, y - 1).cloned() {
                        let neighbor = above_node
                            .borrow_mut()
                            .get_or_create_neighbor(Direction::PosY, above_node.clone());
                        self.set(x, y, neighbor);
                        continue;
                    }
                }
            }
        }
        for x in -(N as i32) / 2..=(N as i32) / 2 {
            for y in -(N as i32) / 2..=(N as i32) / 2 {
                let x = -x;
                let y = -y;
                if self.get(x, y).is_some() {
                    continue;
                }
                if x < (N as i32) / 2 {
                    if let Some(right_node) = self.get(x + 1, y).cloned() {
                        let neighbor = right_node
                            .borrow_mut()
                            .get_or_create_neighbor(Direction::NegX, right_node.clone());
                        self.set(x, y, neighbor);
                        continue;
                    }
                }
                if y < (N as i32) / 2 {
                    if let Some(below_node) = self.get(x, y + 1).cloned() {
                        let neighbor = below_node
                            .borrow_mut()
                            .get_or_create_neighbor(Direction::NegY, below_node.clone());
                        self.set(x, y, neighbor);
                        continue;
                    }
                }
                panic!("{x} {y} not filled in")
            }
        }
    }
}
