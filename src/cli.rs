mod game;

use std::io::{self, Result};
use crate::game::Game;

const BOARD_WIDTH: i16 = 10;
const BOARD_HEIGHT: i16 = 5;


fn main() -> Result<()>{
    println!(r#"
Minesweeper CLI
----------------------------------------
The Minesweeper CLI application is a simple testbed
for the game logic.

Commands:
----------------------------------------
x       Exit
r       Restart 
u[x,y]  Uncover a tile at the coordinates
f[x,y]  Flag a mine at the coordinates
?[x,y]  Mark as unknown at the coordinates
    "#);

    let mut game = Game::new(BOARD_WIDTH, BOARD_HEIGHT);
    let mut buf = String::new();
    loop {
        println!("{}", game);
        let stdin = io::stdin();
        stdin.read_line(&mut buf)?;
        let input = buf.trim();
        match input.chars().nth(0).unwrap() {
            'x' => break,
            'r' => game = Game::new(BOARD_WIDTH, BOARD_HEIGHT),
            'u' => {
                let (x,y) = get_coords(&input[1..input.len()]);
                game.uncover(x, y);
            }
            'f' => {
                let (x,y) = get_coords(&input[1..input.len()]);
                game.flag(x, y);
            },
            '?' => {
                let (x,y) = get_coords(&input[1..input.len()]);
                game.question(x,y);
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())

}

fn get_coords(s: &str) -> (i16, i16) {
    let s = &s[1..s.len()-1];
    let mut parts = s.split(',');
    let s: &str = parts.next().unwrap();
    let x = s.parse::<i16>().unwrap();
    let s: &str = parts.next().unwrap();
    let y = s.parse::<i16>().unwrap();
    (x,y)
}