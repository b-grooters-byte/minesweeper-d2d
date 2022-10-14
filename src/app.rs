mod direct2d;
mod game;
mod gameboard;

use gameboard::GameBoard;
use std::sync::Once;
use windows::{
    core::Result,
    core::HSTRING,
    w,
    Win32::{
        Foundation::HWND,
        Graphics::Direct2D::ID2D1Factory1,
        UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG},
    },
};

static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &HSTRING = w!("bytetrail.window.minesweeper");

fn main() -> windows::core::Result<()> {
    let factory = direct2d::create_factory()?;
    let _m = AppWindow::new("MineSweeper", &factory);
    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, HWND(0), 0, 0).into() {
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

pub(crate) struct AppWindow<'a> {
    handle: HWND,
    game_board: Option<Box<GameBoard<'a>>>,
    factory: &'a ID2D1Factory1,
}

impl<'a> AppWindow<'a> {
    pub(crate) fn new(title: &'static str, factory: &'a ID2D1Factory1) -> Result<Box<Self>> {
        let window = Box::new(AppWindow {
            handle: HWND(0),
            game_board: None,
            factory,
        });

        Ok(window)
    }
}
