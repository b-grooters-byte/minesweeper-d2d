use std::cell::Cell;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum CellState {
    Unknown(bool),
    Known(bool),
    Flagged,
    Counted(u8),
    Questioned,
}

const DENSITY_FACTOR_A: f32 = 0.0002;
const DENSITY_FACTOR_B: f32 = 0.0938;
const DENSITY_FACTOR_C: f32 = 0.8937;

pub(crate) struct Game {
    width: u8,
    height: u8,
    state: GameState,
    field_state: Vec<CellState>,
    remaining: u16,
}

impl Game {
    pub(crate) fn new(width: u8, height: u8) -> Self {
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
            while minefield[cell] == CellState::Unknown(true) {
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

    pub(crate) fn remaining(&self) -> u16 {
        self.remaining
    }

    pub(crate) fn is_mined(&self, x: u8, y: u8) -> bool {
        self.field_state[(y*self.width+x) as usize] == CellState::Unknown(true) ||
            self.field_state[(y*self.width+x) as usize] == CellState::Known(true) 
    }

    pub(crate) fn uncover(&mut self, x: u8, y: u8) -> CellState{
        let index = (y * self.width + x) as usize;
        self.field_state[index]


    }

    fn neighbor_count(&self, x: u8, y: u8) -> u8 {
        let mut count: u8 = 0;
        for y_off in -1_i16..=1 {
            let cell_idx = ((y as i16 + y_off)  * self.width as i16) as usize;
            for x_idx in x-1..=x+1 {
                let index = cell_idx + x_idx as usize;
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
}
