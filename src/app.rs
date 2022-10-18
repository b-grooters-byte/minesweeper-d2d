mod direct2d;
mod game;
mod gameboard;

use gameboard::{BoardLevel, GameBoard};
use std::sync::Once;
use windows::{
    core::Result,
    core::HSTRING,
    w,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct2D::ID2D1Factory1,
            Gdi::{COLOR_WINDOW, HBRUSH},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
            GetWindowLongPtrA, GetWindowRect, LoadCursorW, PostQuitMessage, RegisterClassW,
            SetWindowLongPtrA, SetWindowPos, ShowWindow, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW, MSG, SWP_NOMOVE, SW_SHOW,
            WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
        },
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
        let instance = unsafe { GetModuleHandleW(None)? };
        // synchronization for a one time initialization of FFI call
        REGISTER_WINDOW_CLASS.call_once(|| {
            // use defaults for all other fields
            let class = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
                hInstance: instance,
                style: CS_HREDRAW | CS_VREDRAW,
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                lpszClassName: WINDOW_CLASS_NAME.into(),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });
        let mut app_window = Box::new(AppWindow {
            handle: HWND(0),
            game_board: None,
            factory,
        });
        // create the window using Self reference
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS_NAME,
                &HSTRING::from(title),
                WS_VISIBLE | WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                400,
                300,
                HWND(0),
                HMENU(0),
                instance,
                Some(app_window.as_mut() as *mut _ as _),
            )
        };
        unsafe { ShowWindow(window, SW_SHOW) };
        Ok(app_window)
    }

    fn message_loop(
        &mut self,
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => {
                match GameBoard::new(self.handle, BoardLevel::Medium, self.factory) {
                    Ok(board) => {
                        self.game_board = Some(board);
                        let mut rect = RECT::default();
                        let mut child_rect = RECT::default();
                        unsafe {
                            GetWindowRect(self.handle, &mut rect);
                            GetWindowRect(
                                self.game_board.as_ref().unwrap().hwnd(),
                                &mut child_rect,
                            );
                            AdjustWindowRect(
                                &mut child_rect,
                                WS_VISIBLE | WS_OVERLAPPEDWINDOW,
                                false,
                            );
                            SetWindowPos(
                                self.handle,
                                None,
                                rect.left,
                                rect.top,
                                child_rect.right - child_rect.left,
                                child_rect.bottom - child_rect.top,
                                SWP_NOMOVE,
                            );
                        }
                    }
                    Err(_e) => {
                        // TODO determine correct LRESULT
                        return LRESULT(-1);
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
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
        }
        let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Self;

        if !this.is_null() {
            return (*this).message_loop(window, message, wparam, lparam);
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
