use std::sync::Once;

use windows::{
    core::{IUnknown, Result, HSTRING, GUID},
    w,
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{D2D1_COLOR_F, D2D_RECT_F},
                ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
                D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS,
                D2D1_RENDER_TARGET_PROPERTIES,
            },
            Gdi::{BeginPaint, CreateSolidBrush, EndPaint, PAINTSTRUCT, InvalidateRect}, DirectWrite::{DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory, IDWriteFactory1},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, GetClientRect, GetWindowLongPtrA, LoadCursorW,
            RegisterClassW, SetWindowLongPtrA, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW, WINDOW_EX_STYLE, WM_CREATE, WM_PAINT,
            WNDCLASSW, WS_CHILDWINDOW, WS_CLIPSIBLINGS, WS_VISIBLE, WM_LBUTTONUP,
        },
    },
};

use crate::{direct2d::create_brush, game::CellState, game::Game};

static REGISTER_GAMEBOARD_WINDOW_CLASS: Once = Once::new();
static GAMEBOARD_WINDOW_CLASS_NAME: &HSTRING = w!("bytetrail.window.bezier-demo");

const CELL_WIDTH: f32 = 5.0 / 25.4;
const CELL_HEIGHT: f32 = 5.0 / 25.4;
const BOARD_COLOR: (f32, f32, f32) = (0.4, 0.4, 0.4);
const CELL_COLOR: (f32, f32, f32) = (0.85, 0.85, 0.85);
const CELL_HIGHLIGHT: (f32, f32, f32) = (0.95, 0.95, 0.95);

pub(crate) enum BoardLevel {
    Easy,
    Medium,
    Difficult,
}

pub(crate) struct GameBoard<'a> {
    handle: HWND,
    factory: &'a ID2D1Factory1,
    write_factory: IDWriteFactory,
    target: Option<ID2D1HwndRenderTarget>,
    cell_brush: Option<ID2D1SolidColorBrush>,
    game: Game,
    cell_width: f32,
    cell_height: f32,
    dpix: f32,
    dpiy: f32,
}

impl<'a> GameBoard<'a> {
    pub(crate) fn new(
        parent: HWND,
        level: BoardLevel,
        factory: &'a ID2D1Factory1,
    ) -> Result<Box<Self>> {
        let instance = unsafe { GetModuleHandleW(None)? };
        let factory_iid = GUID::new()?;
        let write_factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?};
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
            BoardLevel::Easy => (
                dpix * 8.0 * CELL_WIDTH,
                dpiy * 10.0 * CELL_HEIGHT,
                Game::new(8, 10),
            ),
            BoardLevel::Medium => (
                dpix * 12.0 * CELL_WIDTH,
                dpiy * 16.0 * CELL_HEIGHT,
                Game::new(12, 16),
            ),
            BoardLevel::Difficult => (
                dpix * 30.0 * CELL_WIDTH,
                dpiy * 18.0 * CELL_HEIGHT,
                Game::new(30, 18),
            ),
        };

        let mut board = Box::new(GameBoard {
            handle: HWND(0),
            factory,
            write_factory,
            target: None,
            cell_brush: None,
            game,
            cell_width: dpix * CELL_WIDTH as f32,
            cell_height: dpiy * CELL_HEIGHT as f32,
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
        if self.target.is_none() {
            self.create_render_target()?;
            let target = self.target.as_ref().unwrap();
            unsafe { target.SetDpi(self.dpix, self.dpiy) };
            self.cell_brush = Some(create_brush(
                target,
                CELL_COLOR.0,
                CELL_COLOR.1,
                CELL_COLOR.2,
                1.0,
            )?);
        }
        unsafe {
            self.target.as_ref().unwrap().BeginDraw();
            self.draw_board();
            self.target.as_ref().unwrap().EndDraw(None, None)?;
        }
        Ok(())
    }

    fn draw_board(&mut self) -> Result<()> {
        let target = self.target.as_mut().unwrap();
        unsafe {
            target.Clear(Some(&D2D1_COLOR_F {
                r: BOARD_COLOR.0,
                g: BOARD_COLOR.1,
                b: BOARD_COLOR.2,
                a: 1.0,
            }));
        }

        let cell_brush = self.cell_brush.as_ref().unwrap();
        for x in 0..self.game.width() {
            for y in 0..self.game.height() {
                let left = x as f32 * self.cell_width + 1.0;
                let top = y as f32  * self.cell_height + 1.0;
                let right = left + self.cell_width - 2.0;
                let bottom = top + self.cell_height - 2.0;
                match self.game.cell_state(x, y) {
                    CellState::Flagged(_) => {
                        unsafe {
                            target.FillRectangle(
                                &D2D_RECT_F {
                                    left: 1.0,
                                    top: 1.0,
                                    right: 11.0,
                                    bottom: 10.0,
                                },
                                cell_brush,
                            )
                        };
                    }
                    CellState::Unknown(_) => {
                        unsafe {
                            target.FillRectangle(
                                &D2D_RECT_F {
                                    left,
                                    top,
                                    right,
                                    bottom
                                },
                                cell_brush,
                            )
                        };
                    }
                    CellState::Known(mined) => {}
                    CellState::Counted(count) => {

                    }
                    CellState::Questioned(_) => {}
                }
            }
        }
        Ok(())
    }

    fn create_render_target(&mut self) -> Result<()> {
        unsafe {
            let mut rect: RECT = RECT::default();
            GetClientRect(self.handle, &mut rect);
            let props = D2D1_RENDER_TARGET_PROPERTIES::default();
            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.handle,
                pixelSize: windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS::default(),
            };
            let target = self.factory.CreateHwndRenderTarget(&props, &hwnd_props)?;
            self.target = Some(target);
        }
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
            WM_LBUTTONUP =>  {
                let (x, y) = mouse_position(lparam);
                let x_cell = (x / self.cell_width) as i16;
                let y_cell = (y / self.cell_height) as i16;
                self.game.uncover(x_cell, y_cell);
                // TODO manage the results of uncover to control clip
                unsafe { InvalidateRect(self.handle, None, false) };
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


fn mouse_position(lparam: LPARAM) -> (f32, f32) {
    (
        (lparam.0 & 0x0000_FFFF) as f32,
        ((lparam.0 & 0xFFFF_0000) >> 16) as f32,
    )
}
