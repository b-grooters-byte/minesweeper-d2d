use std::{cell::Cell, fmt::Display};

use rand::{rngs::StdRng, Rng, SeedableRng};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    Graphics::Direct2D::ID2D1Factory1,
};

enum GameState {
    Initial,
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum CellState {
    Unknown(bool),
    Known(bool),
    Flagged(bool),
    Counted(u8),
    Questioned(bool),
}

const DENSITY_FACTOR_A: f32 = 0.0002;
const DENSITY_FACTOR_B: f32 = 0.0938;
const DENSITY_FACTOR_C: f32 = 0.8937;

pub(crate) struct Game {
    width: i16,
    height: i16,
    state: GameState,
    field_state: Vec<CellState>,
    remaining: u16,
}

impl Game {
    pub(crate) fn new(width: i16, height: i16) -> Self {
        let mut rng = StdRng::from_entropy();
        let size = width as usize * height as usize;
        let mut minefield = Vec::<CellState>::with_capacity(size);
        let density = ((width as f32 * height as f32).powi(2) * DENSITY_FACTOR_A
            + (width as f32 * height as f32) * DENSITY_FACTOR_B
            + DENSITY_FACTOR_C) as u16;

        for _ in 0..size {
            minefield.push(CellState::Unknown(false));
        }

        for _ in 0..density {
            let mut cell = rng.gen_range(0..size);
            while let CellState::Unknown(true) = minefield[cell] {
                cell = rng.gen_range(0..size);
            }
            minefield[cell] = CellState::Unknown(true);
        }
        Game {
            width,
            height,
            state: GameState::Initial,
            field_state: minefield,
            remaining: density,
        }
    }

    pub(crate) fn clear(&mut self) {
        for i in 0..self.field_state.len() {
            self.field_state[i] = CellState::Unknown(false);
        }
    }

    pub(crate) fn remaining(&self) -> u16 {
        self.remaining
    }

    pub(crate) fn flag(&mut self, x: i16, y: i16) {
        let index = (y * self.width + x) as usize;
        match self.field_state[index] {
            CellState::Unknown(mined) |
            CellState::Questioned(mined) => self.field_state[index] = CellState::Flagged(mined),
            _ => {}
        }
    }

    pub(crate) fn is_mined(&self, x: i16, y: i16) -> bool {
        self.field_state[(y * self.width + x) as usize] == CellState::Unknown(true)
            || self.field_state[(y * self.width + x) as usize] == CellState::Known(true)
    }

    pub(crate) fn uncover(&mut self, x: i16, y: i16) {
        let index = (y * self.width + x) as usize;
        match self.field_state[index] {
            CellState::Questioned(false)
            | CellState::Flagged(false)
            | CellState::Unknown(false) => {
                let count = self.neighbor_count(x, y);
                if count != 0 {
                    self.field_state[index] = CellState::Counted(count);
                } else {
                    let mut stack = Vec::<(i16, i16)>::new();
                    stack.push((x, y));
                    while stack.len() > 0 {
                        let (x, y) = stack.pop().unwrap();
                        let index = (y * self.width + x) as usize;
                        let count = self.neighbor_count(x, y);
                        if count == 0 {
                            self.field_state[index] = CellState::Known(false);
                            for y_idx in y - 1..=y + 1 {
                                if y_idx < 0 || y_idx == self.height {
                                    continue;
                                }
                                let row_idx = (y_idx * self.width) as usize;
                                for x_idx in x - 1..=x + 1 {
                                    if x_idx < 0 || x_idx == self.width {
                                        continue;
                                    }
                                    let index = row_idx + x_idx as usize;
                                    // do not check self
                                    if index == (y * self.width + x) as usize {
                                        continue;
                                    }
                                    if self.field_state[index] == CellState::Unknown(false) {
                                        stack.push((x_idx, y_idx));
                                    }
                                }
                            }
                        } else {
                            self.field_state[index] = CellState::Counted(count);
                        }
                    }
                }
            }
            CellState::Questioned(true) | CellState::Flagged(true) | CellState::Unknown(true) => {
                // uncovered a mined cell
                self.field_state[index] = CellState::Known(true);
            }
            _ => {
                // do nothing in the known states
            }
        }
    }

    fn neighbor_count(&self, x: i16, y: i16) -> u8 {
        let mut count: u8 = 0;
        for y_idx in y - 1..=y + 1 {
            if y_idx < 0 || y_idx == self.height {
                continue;
            }
            let row_idx = (y_idx * self.width) as usize;
            for x_idx in x - 1..=x + 1 {
                if x_idx < 0 || x_idx == self.width {
                    continue;
                }
                let index = row_idx + x_idx as usize;
                // do not check self
                if index == (y * self.width + x) as usize {
                    continue;
                }
                if self.field_state[index] == CellState::Unknown(true) {
                    count += 1
                }
            }
        }
        count
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut field = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                match self.field_state[idx] {
                    CellState::Unknown(_) => field.push('\u{25A0}'),
                    CellState::Known(false) => field.push('\u{25A1}'),
                    CellState::Known(true) => field.push('*'),
                    CellState::Counted(count) => field.push_str(count.to_string().as_str()),
                    CellState::Flagged(_) => field.push('\u{1F3F3}'),
                    CellState::Questioned(_) => field.push('?'),
                }
                field.push(' ');
            }
            field.push('\n');
        }
        f.write_str(field.as_str())
    }
}

pub(crate) struct MineFieldWindow<'a> {
    factory: &'a ID2D1Factory1,
}

impl<'a> MineFieldWindow<'a> {
    pub(crate) fn new(factory: &'a ID2D1Factory1) -> Self {
        MineFieldWindow { factory }
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_game_new() {
        let game = Game::new(10, 10);
        assert_eq!(12, game.remaining());
        let mut remaining = 0_u16;
        for cell in game.field_state {
            if cell == CellState::Unknown(true) {
                remaining += 1;
            }
        }
        assert_eq!(12, remaining);
    }

    #[test]
    pub fn test_neighbor_count() {
        let mut game = Game::new(10, 10);
        // clear the mine field
        for i in 0..100 {
            game.field_state[i] = CellState::Unknown(false);
        }
        // set a specific mine
        game.field_state[32] = CellState::Unknown(true);
        let count = game.neighbor_count(3, 4);
        assert_eq!(1, count);
        game.field_state[54] = CellState::Unknown(true);
        let count = game.neighbor_count(3, 4);
        assert_eq!(2, count);
        game.field_state[42] = CellState::Unknown(true);
        let count = game.neighbor_count(3, 4);
        assert_eq!(3, count);
        game.field_state[44] = CellState::Unknown(true);
        let count = game.neighbor_count(3, 4);
        assert_eq!(4, count);
        game.field_state[43] = CellState::Unknown(true);
        let count = game.neighbor_count(3, 4);
        assert_eq!(4, count);
    }

    #[test]
    pub fn test_uncover_simple() {
        //   * 2 0 1 *
        //   * 2 0 1 1
        //   1 1 1 1 1
        //   0 0 1 * 1
        //   0 0 1 1 1
        let mut game = Game::new(5, 5);
        game.clear();
        game.field_state[0] = CellState::Unknown(true);
        game.field_state[4] = CellState::Unknown(true);
        game.field_state[5] = CellState::Unknown(true);
        game.field_state[18] = CellState::Unknown(true);
        assert_eq!(0, game.neighbor_count(2, 0));
        game.uncover(2, 0);
        assert_eq!(CellState::Known(false), game.field_state[7]);
        assert_eq!(CellState::Counted(2), game.field_state[1]);
        assert_eq!(CellState::Counted(2), game.field_state[6]);
        assert_eq!(CellState::Counted(1), game.field_state[3]);
        assert_eq!(CellState::Counted(1), game.field_state[8]);
        assert_eq!(CellState::Unknown(false), game.field_state[10]);
        assert_eq!(CellState::Counted(1), game.field_state[11]);
        assert_eq!(CellState::Counted(1), game.field_state[12]);
        assert_eq!(CellState::Counted(1), game.field_state[13]);
        assert_eq!(CellState::Unknown(false), game.field_state[14]);
        game.uncover(3,3);
        assert_eq!(CellState::Known(true), game.field_state[18]);

    }
}
