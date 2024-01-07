use std::sync::Once;

use windows::{
    core::{Result, HSTRING},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Direct2D::{
                Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F},
                ID2D1Bitmap, ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
                ID2D1StrokeStyle, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_HWND_RENDER_TARGET_PROPERTIES,
                D2D1_PRESENT_OPTIONS, D2D1_RENDER_TARGET_PROPERTIES,
            },
            DirectWrite::{
                DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
                DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            },
            Gdi::{BeginPaint, CreateSolidBrush, EndPaint, InvalidateRect, PAINTSTRUCT},
            Imaging::IWICImagingFactory,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, GetClientRect, GetWindowLongPtrA, LoadCursorW,
            RegisterClassW, SetWindowLongPtrA, CREATESTRUCTA, CS_HREDRAW, CS_VREDRAW,
            CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW, WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY,
            WM_LBUTTONUP, WM_PAINT, WM_RBUTTONUP, WNDCLASSW, WS_CHILDWINDOW, WS_CLIPSIBLINGS,
            WS_VISIBLE,
        },
    },
};

use crate::{
    direct2d::{create_brush, create_image_factory, create_style, load_bitmap},
    game::{CellState, Game, GameState},
};

static REGISTER_GAMEBOARD_WINDOW_CLASS: Once = Once::new();

const CELL_WIDTH: f32 = 6.0 / 25.4;
const CELL_HEIGHT: f32 = 6.0 / 25.4;
const BOARD_COLOR: (f32, f32, f32) = (0.4, 0.4, 0.4);
const CELL_COLOR: (f32, f32, f32) = (0.75, 0.75, 0.75);
const CELL_HIGHLIGHT: (f32, f32, f32) = (1.0, 1.0, 1.0);
const DEFAULT_COLOR: (f32, f32, f32) = (0.0, 0.0, 0.0);
const NUM_BRUSH: [(f32, f32, f32); 7] = [
    (0.0, 0.0, 0.5),
    (0.0, 0.5, 0.0),
    (0.5, 0.0, 0.0),
    (0.35, 0.0, 0.7),
    (0.25, 0.0, 0.0),
    (0.0, 0.65, 1.0),
    (0.0, 0.0, 0.0),
];
const MINE_FILE: &HSTRING = windows::core::h!("mine.png");
const FLAG_FILE: &HSTRING = windows::core::h!("flag.png");

pub(crate) enum BoardLevel {
    Easy,
    Medium,
    Difficult,
}

pub(crate) struct GameBoard<'a> {
    handle: HWND,
    factory: &'a ID2D1Factory1,
    image_factory: IWICImagingFactory,
    text_format: IDWriteTextFormat,
    target: Option<ID2D1HwndRenderTarget>,
    line_style: ID2D1StrokeStyle,
    default_brush: Option<ID2D1SolidColorBrush>,
    cell_brush: Option<ID2D1SolidColorBrush>,
    cell_highlight: Option<ID2D1SolidColorBrush>,
    num_brush: [Option<ID2D1SolidColorBrush>; 7],
    flag: Option<ID2D1Bitmap>,
    mine: Option<ID2D1Bitmap>,
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
        let write_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
        let image_factory = create_image_factory()?;
        let line_style = create_style(factory, None)?;
        let text_format = unsafe {
            write_factory.CreateTextFormat(
                &HSTRING::from("San Serif"),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                14.0,
                &HSTRING::from("en-US"),
            )?
        };
        unsafe {
            text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        }
        REGISTER_GAMEBOARD_WINDOW_CLASS.call_once(|| {
            // use defaults for all other fields
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wnd_proc),
                hInstance: instance.into(),
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                hbrBackground: unsafe { CreateSolidBrush(COLORREF(0)) },
                lpszClassName: windows::core::w!("bytetrail.window.bezier-demo"),
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
            image_factory,
            text_format,
            target: None,
            line_style,
            default_brush: None,
            cell_brush: None,
            cell_highlight: None,
            num_brush: [None, None, None, None, None, None, None],
            flag: None,
            mine: None,
            game,
            cell_width: dpix * CELL_WIDTH,
            cell_height: dpiy * CELL_HEIGHT,
            dpix,
            dpiy,
        });

        let _window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                windows::core::w!("bytetrail.window.bezier-demo"),
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

    pub(crate) fn hwnd(&self) -> HWND {
        self.handle
    }

    fn release_device(&mut self) {
        self.target = None;
        self.release_device_resources();
    }

    fn release_device_resources(&mut self) {
        self.default_brush = None;
        self.cell_brush = None;
        self.cell_highlight = None;
        for i in 0..7 {
            self.num_brush[i] = None;
        }
        self.flag = None;
        self.mine = None;
        self.target = None;
    }

    fn render(&mut self) -> Result<()> {
        if self.target.is_none() {
            self.create_render_target()?;
            let target = self.target.as_ref().unwrap();
            self.flag = Some(load_bitmap(FLAG_FILE, target, &self.image_factory)?);
            self.mine = Some(load_bitmap(MINE_FILE, target, &self.image_factory)?);
            unsafe { target.SetDpi(self.dpix, self.dpiy) };
            self.default_brush = Some(create_brush(
                target,
                DEFAULT_COLOR.0,
                DEFAULT_COLOR.1,
                DEFAULT_COLOR.2,
                1.0,
            )?);
            self.cell_highlight = Some(create_brush(
                target,
                CELL_HIGHLIGHT.0,
                CELL_HIGHLIGHT.1,
                CELL_HIGHLIGHT.2,
                1.0,
            )?);
            self.cell_brush = Some(create_brush(
                target,
                CELL_COLOR.0,
                CELL_COLOR.1,
                CELL_COLOR.2,
                1.0,
            )?);
            for (i, brush) in NUM_BRUSH.iter().enumerate() {
                self.num_brush[i] = Some(create_brush(target, brush.0, brush.1, brush.2, 1.0)?);
            }
        }
        unsafe {
            self.target.as_ref().unwrap().BeginDraw();
            self.draw_board()?;
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

        let default_brush = self.default_brush.as_ref().unwrap();
        let cell_brush = self.cell_brush.as_ref().unwrap();
        let cell_highlight = self.cell_highlight.as_ref().unwrap();
        let mut num_brush: Vec<&ID2D1SolidColorBrush> = Vec::new();
        for brush_ref in &self.num_brush {
            num_brush.push(brush_ref.as_ref().unwrap());
        }
        let flag = self.flag.as_ref().unwrap();
        let mine = self.mine.as_ref().unwrap();

        for x in 0..self.game.width() {
            for y in 0..self.game.height() {
                let left = x as f32 * self.cell_width + 1.0;
                let top = y as f32 * self.cell_height + 1.0;
                let right = left + self.cell_width - 2.0;
                let bottom = top + self.cell_height - 2.0;
                let rect = D2D_RECT_F {
                    left,
                    top,
                    right,
                    bottom,
                };
                match self.game.cell_state(x, y) {
                    CellState::Flagged(_) | CellState::Questioned(_) | CellState::Unknown(_) => {
                        unsafe {
                            target.FillRectangle(&rect, cell_brush);
                            target.DrawLine(
                                D2D_POINT_2F { x: left, y: top },
                                D2D_POINT_2F { x: left, y: bottom },
                                cell_highlight,
                                1.5,
                                &self.line_style,
                            );
                            target.DrawLine(
                                D2D_POINT_2F { x: left, y: top },
                                D2D_POINT_2F { x: right, y: top },
                                cell_highlight,
                                1.5,
                                &self.line_style,
                            );
                        }
                        match self.game.cell_state(x, y) {
                            CellState::Flagged(_) => unsafe {
                                target.DrawBitmap(
                                    flag,
                                    Some(&rect),
                                    1.0,
                                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                                    None,
                                );
                            },
                            CellState::Questioned(_) => unsafe {
                                target.DrawText(
                                    &("?".encode_utf16().collect::<Vec<u16>>()),
                                    &self.text_format,
                                    &rect,
                                    default_brush,
                                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                                    DWRITE_MEASURING_MODE_NATURAL,
                                );
                            },
                            _ => {}
                        }
                    }
                    CellState::Known(mined) => {
                        unsafe {
                            target.FillRectangle(&rect, cell_brush);
                        }
                        if mined {
                            unsafe {
                                target.DrawBitmap(
                                    mine,
                                    Some(&rect),
                                    1.0,
                                    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                                    None,
                                );
                            }
                        }
                    }
                    CellState::Counted(count) => unsafe {
                        let mut mine_count = count;
                        target.FillRectangle(&rect, cell_brush);
                        let num: Vec<u16> = count.to_string().encode_utf16().collect();
                        if count > 7 {
                            mine_count = 7;
                        }
                        target.DrawText(
                            &num,
                            &self.text_format,
                            &rect,
                            num_brush[(mine_count - 1) as usize],
                            D2D1_DRAW_TEXT_OPTIONS_NONE,
                            DWRITE_MEASURING_MODE_NATURAL,
                        );
                    },
                }
            }
        }
        Ok(())
    }

    fn create_render_target(&mut self) -> Result<()> {
        unsafe {
            let mut rect: RECT = RECT::default();
            let _ = GetClientRect(self.handle, &mut rect);
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
            WM_RBUTTONUP => {
                let (x, y) = mouse_position(lparam);
                let x_cell = (x / self.cell_width) as i16;
                let y_cell = (y / self.cell_height) as i16;

                match self.game.cell_state(x_cell, y_cell) {
                    CellState::Unknown(_) => self.game.flag(x_cell, y_cell),
                    CellState::Flagged(_) => self.game.question(x_cell, y_cell),
                    CellState::Questioned(_) => self.game.set_unknown(x_cell, y_cell),
                    _ => {}
                }
                unsafe { InvalidateRect(self.handle, None, false) };

                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if self.game.state() == GameState::Lost {
                    self.game.reset();
                } else {
                    let (x, y) = mouse_position(lparam);
                    let x_cell = (x / self.cell_width) as i16;
                    let y_cell = (y / self.cell_height) as i16;
                    let state = self.game.uncover(x_cell, y_cell);
                    // todo animate lost sequence
                    if state == GameState::Lost {
                        self.game.show_mined();
                    }
                    // TODO animate won sequence
                    if state == GameState::Won {
                        self.game.reset();
                    }
                }
                // TODO manage the results of uncover to control clip
                unsafe { InvalidateRect(self.handle, None, false) };
                LRESULT(0)
            }
            WM_DESTROY => {
                self.release_device();
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
