use std::ptr::null;

use windows::{
    core::*,
    Foundation::Numerics::Matrix3x2,
    Win32::Graphics::Direct2D::Common::*,
    Win32::{
        Graphics::{
            Direct2D::*,
            Imaging::{
                CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmapDecoder,
                IWICImagingFactory, WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut,
                WICDecodeMetadataCacheOnLoad,
            },
        },
        System::{
            Com::{CoCreateInstance, CLSCTX_ALL},
            SystemServices::GENERIC_READ,
        },
    },
};

/// Creates a single threaded Direct2D factory with default options.
pub fn create_factory() -> Result<ID2D1Factory1> {
    let mut options = D2D1_FACTORY_OPTIONS::default();

    if cfg!(debug_assertions) {
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
    }

    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options)) }
}

pub fn create_image_factory() -> Result<IWICImagingFactory> {
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL) }
}

/// Create a stroke style with the specified dash pattern
pub fn create_style(factory: &ID2D1Factory1, dashes: Option<&[f32]>) -> Result<ID2D1StrokeStyle> {
    let mut props = D2D1_STROKE_STYLE_PROPERTIES {
        startCap: D2D1_CAP_STYLE_ROUND,
        endCap: D2D1_CAP_STYLE_ROUND,
        ..Default::default()
    };
    if dashes.is_some() {
        props.dashStyle = D2D1_DASH_STYLE_CUSTOM;
    }
    unsafe { factory.CreateStrokeStyle(&props, dashes) }
}

pub fn create_brush(
    target: &ID2D1HwndRenderTarget,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) -> Result<ID2D1SolidColorBrush> {
    let color = D2D1_COLOR_F { r, g, b, a };
    let properties = D2D1_BRUSH_PROPERTIES {
        opacity: 0.8,
        transform: Matrix3x2::identity(),
    };
    unsafe { target.CreateSolidColorBrush(&color, Some(&properties)) }
}

pub fn load_bitmap(
    filename: &HSTRING,
    target: &ID2D1HwndRenderTarget,
    factory: &IWICImagingFactory,
) -> Result<ID2D1Bitmap> {
    unsafe {
        let decoder = factory.CreateDecoderFromFilename(
            filename,
            null(),
            GENERIC_READ,
            WICDecodeMetadataCacheOnLoad,
        )?;
        let frame = decoder.GetFrame(0)?;
        let converter = factory.CreateFormatConverter()?;
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )?;
        target.CreateBitmapFromWicBitmap(&converter, None)
    }
}
