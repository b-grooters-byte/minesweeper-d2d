mod minefield;

use std::io::{stdin, self, Result};

use crate::minefield::Game;


fn main() -> Result<()>{
    let mut game = Game::new(15, 10);
    let mut buf = String::new();
    loop {
        println!("{}", game);
        let stdin = io::stdin();
        stdin.read_line(&mut buf)?;
        let input = buf.trim();
        match input.chars().nth(0).unwrap() {
            'x' => break,
            'u' => {
                let (x,y) = get_coords(&input[1..input.len()]);
                game.uncover(x, y);
            }
            'f' => {
                let (x,y) = get_coords(&input[1..input.len()]);
                game.flag(x, y);
            },
            _ => {}
        }
        if input == "x" {
            break;
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