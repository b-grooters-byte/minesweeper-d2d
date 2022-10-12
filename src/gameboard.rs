use std::sync::Once;

use windows::{
    core::{Result, HSTRING},
    w,
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Direct2D::ID2D1Factory1,
            Gdi::{BeginPaint, CreateSolidBrush, EndPaint, PAINTSTRUCT},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            DefWindowProcW, GetWindowLongPtrA, LoadCursorW, RegisterClassW, SetWindowLongPtrA,
            CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW, WM_CREATE, WNDCLASSW, CreateWindowExW, WINDOW_EX_STYLE, WS_VISIBLE, WS_CLIPSIBLINGS, WS_CHILDWINDOW, CW_USEDEFAULT, HMENU, WM_PAINT,
        },
    },
};

use crate::game::Game;

static REGISTER_GAMEBOARD_WINDOW_CLASS: Once = Once::new();
static GAMEBOARD_WINDOW_CLASS_NAME: &HSTRING = w!("bytetrail.window.bezier-demo");

const CELL_WIDTH: f32 = 5.0 / 25.4;
const CELL_HEIGHT: f32 = 5.0 / 25.4;

pub(crate) enum BoardLevel {
    Easy,
    Medium,
    Difficult,
}

pub(crate) struct GameBoard<'a> {
    handle: HWND,
    factory: &'a ID2D1Factory1,
    game: Game,
    cell_width: i32,
    cell_height: i32,
    dpix: f32,
    dpiy: f32,
}

impl<'a> GameBoard<'a> {
    pub(crate) fn new(parent: HWND, level: BoardLevel, factory: &'a ID2D1Factory1) -> Result<Box<Self>> {
        let instance = unsafe { GetModuleHandleW(None)? };
        REGISTER_GAMEBOARD_WINDOW_CLASS.call_once(|| {
            // use defaults for all other fields
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wnd_proc),
                hInstance: instance,
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                hbrBackground: unsafe { CreateSolidBrush(COLORREF(0)) },
                lpszClassName: GAMEBOARD_WINDOW_CLASS_NAME.into(),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });

        let mut dpix = 0.0;
        let mut dpiy = 0.0;
        unsafe { factory.GetDesktopDpi(&mut dpix, &mut dpiy) };

        let (width, height, game) = match level {
            BoardLevel::Easy => 
                (
                    dpix * 8.0 * CELL_WIDTH,
                    dpiy * 10.0 * CELL_HEIGHT,
                    Game::new(8, 10)
                ),
            BoardLevel::Medium => 
            (
                dpix * 12.0 * CELL_WIDTH,
                dpiy * 16.0 * CELL_HEIGHT,
                Game::new(12, 16)
            ),
            BoardLevel::Difficult => 
            (
                dpix * 30.0 * CELL_WIDTH,
                dpiy * 18.0 * CELL_HEIGHT,
                Game::new(30, 18)
            ),
        };

        let mut board = Box::new(GameBoard { 
            handle: HWND(0),
            factory,
            game,
            cell_width: (dpix * CELL_WIDTH as f32) as i32,
            cell_height: (dpiy * CELL_HEIGHT as f32) as i32,
            dpix,
            dpiy,
         });
         
         let _window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                GAMEBOARD_WINDOW_CLASS_NAME,
                &HSTRING::from(""),
                WS_VISIBLE | WS_CLIPSIBLINGS | WS_CHILDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                parent,
                HMENU(0),
                instance,
                Some(board.as_mut() as *mut _ as _),
            )
        };

        Ok(board)

    }

    fn render(&mut self) -> Result<()> {
        Ok(())
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                unsafe {
                    BeginPaint(self.handle, &mut ps);
                    self.render().expect("unable to render");
                    EndPaint(self.handle, &ps);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(self.handle, message, wparam, lparam) },
        }
    }

    unsafe extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_CREATE {
            let create_struct = lparam.0 as *const CREATESTRUCTA;
            let this = (*create_struct).lpCreateParams as *mut Self;
            (*this).handle = window;

            SetWindowLongPtrA(window, GWLP_USERDATA, this as _);
        } else {
            let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Self;

            if !this.is_null() {
                return (*this).message_handler(message, wparam, lparam);
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
