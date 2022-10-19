use rand::{rngs::StdRng, Rng, SeedableRng};
use std::fmt::Display;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum GameState {
    Initial,
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    total: u16,
    remaining: u16,
}

impl Game {
    pub(crate) fn new(width: i16, height: i16) -> Self {
        let size = width as usize * height as usize;
        let minefield = Vec::<CellState>::with_capacity(size);
        let mut game = Game {
            width,
            height,
            state: GameState::Initial,
            field_state: minefield,
            total: 0,
            remaining: 0,
        };
        game.reset();
        game
    }

    pub(crate) fn width(&self) -> i16 {
        self.width
    }

    pub(crate) fn height(&self) -> i16 {
        self.height
    }

    pub(crate) fn state(&self) -> GameState {
        self.state
    }

    pub(crate) fn cell_state(&self, x: i16, y: i16) -> CellState {
        let index = (y * self.width + x) as usize;
        self.field_state[index]
    }

    pub(crate) fn reset(&mut self) {
        let mut rng = StdRng::from_entropy();
        let density = ((self.width as f32 * self.height as f32).powi(2) * DENSITY_FACTOR_A
            + (self.width as f32 * self.height as f32) * DENSITY_FACTOR_B
            + DENSITY_FACTOR_C) as u16;
        let size = (self.width * self.height) as usize;
        self.clear();
        for _ in 0..density {
            let mut cell = rng.gen_range(0..size);
            while let CellState::Unknown(true) = self.field_state[cell] {
                cell = rng.gen_range(0..size);
            }
            self.field_state[cell] = CellState::Unknown(true);
        }
        self.remaining = density;
        self.total = density;
        self.state = GameState::Initial;
    }

    pub(crate) fn clear(&mut self) {
        // wipe the board and push new values
        self.field_state.clear();
        for _i in 0..(self.width as usize * self.height as usize) {
            self.field_state.push(CellState::Unknown(false));
        }
        self.state = GameState::Initial;
    }

    pub(crate) fn remaining(&self) -> u16 {
        self.remaining
    }

    pub(crate) fn flag(&mut self, x: i16, y: i16) {
        let index = (y * self.width + x) as usize;
        match self.field_state[index] {
            CellState::Unknown(mined) | CellState::Questioned(mined) => {
                self.field_state[index] = CellState::Flagged(mined);
                if self.remaining > 0 {
                    self.remaining -= 1;
                }
            }
            _ => {}
        }
        self.state = GameState::Playing;
    }

    pub(crate) fn question(&mut self, x: i16, y: i16) {
        let index = (y * self.width + x) as usize;
        match self.field_state[index] {
            CellState::Unknown(mined) => self.field_state[index] = CellState::Questioned(mined),
            CellState::Flagged(mined) => {
                self.field_state[index] = CellState::Questioned(mined);
                // todo correct for over flagged
                self.remaining += 1;
            }
            _ => {}
        }
        self.state = GameState::Playing;
    }

    pub(crate) fn set_unknown(&mut self, x: i16, y: i16) {
        let index = (y * self.width + x) as usize;
        match self.field_state[index] {
            CellState::Flagged(mined) => {
                self.field_state[index] = CellState::Unknown(mined);
                // todo correct for over flagged
                self.remaining += 1;
            }
            CellState::Known(mined) | CellState::Questioned(mined) => {
                self.field_state[index] = CellState::Unknown(mined)
            }

            CellState::Counted(_) => self.field_state[index] = CellState::Unknown(false),
            _ => {}
        }
    }

    pub(crate) fn is_mined(&self, x: i16, y: i16) -> bool {
        self.field_state[(y * self.width + x) as usize] == CellState::Unknown(true)
            || self.field_state[(y * self.width + x) as usize] == CellState::Known(true)
    }

    pub(crate) fn show_mined(&mut self) {
        for i in 0..self.field_state.len() {
            if self.field_state[i] == CellState::Unknown(true) {
                self.field_state[i] = CellState::Known(true);
            }
        }
    }

    pub(crate) fn uncover(&mut self, x: i16, y: i16) -> GameState {
        if self.state == GameState::Lost {
            return self.state;
        }
        self.state = GameState::Playing;
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
                self.state = GameState::Lost;
            }
            _ => {
                // do nothing in the known states
            }
        }
        self.state
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
                if self.field_state[index] == CellState::Unknown(true)
                    || self.field_state[index] == CellState::Questioned(true)
                    || self.field_state[index] == CellState::Flagged(true)
                {
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
        game.uncover(3, 3);
        assert_eq!(CellState::Known(true), game.field_state[18]);
    }

    #[test]
    pub fn test_uncover_edge() {
        // 1 1 1 0 0
        // 2 * 1 0 0
        // * 3 1 0 0
        // * 2 0 0 0
        let mut game = Game::new(5, 5);
        game.clear();
        game.field_state[6] = CellState::Unknown(true);
        game.field_state[10] = CellState::Unknown(true);
        game.field_state[15] = CellState::Unknown(true);
        game.uncover(2, 3);
        assert_eq!(CellState::Counted(2), game.field_state[16]);
        assert_eq!(CellState::Counted(3), game.field_state[11]);
        assert_eq!(CellState::Counted(1), game.field_state[12]);
        assert_eq!(CellState::Counted(1), game.field_state[7]);
    }

    #[test]
    fn test_game_state() {
        let mut game = Game::new(5, 5);
        assert_eq!(GameState::Initial, game.state);
        game.clear();
        assert_eq!(GameState::Initial, game.state);
        let state = game.uncover(1, 1);
        assert_eq!(GameState::Playing, state);
        game.field_state[0] = CellState::Unknown(true);
        let state = game.uncover(0, 0);
        assert_eq!(GameState::Lost, state);
        game.reset();
        assert_eq!(GameState::Initial, game.state);
    }
}
