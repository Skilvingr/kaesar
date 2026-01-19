use std::fs::File;
use std::mem;

use kaesar_core::anyhow::{Context, Error};
use kaesar_core::chrono::Local;
use kaesar_core::colour::Colour;
use kaesar_core::framebuffer::{ColourSpace, Framebuffer, UpdateMode};
use kaesar_core::geom::Rectangle;
use kaesar_core::png;
use sdl2::pixels::{Color as SdlColor, PixelFormatEnum};
use sdl2::rect::Point as SdlPoint;
use sdl2::rect::Rect as SdlRect;
use sdl2::render::{BlendMode, WindowCanvas};

use crate::DEFAULT_ROTATION;

pub struct FBCanvas {
    canvas: Option<WindowCanvas>,
    inverted: bool,
    rotation: i8,
}

impl FBCanvas {
    pub fn new(canvas: WindowCanvas) -> Self {
        Self {
            canvas: Some(canvas),
            inverted: false,
            rotation: DEFAULT_ROTATION,
        }
    }
}

unsafe impl Send for FBCanvas {}
impl Framebuffer for FBCanvas {
    fn colour_space(&self) -> ColourSpace {
        ColourSpace::Rgb
    }

    fn set_pixel(&mut self, x: u32, y: u32, mut color: Colour) {
        if self.inverted() {
            color.invert();
        }

        let [red, green, blue] = color.rgb();
        self.canvas
            .as_mut()
            .unwrap()
            .set_draw_color(SdlColor::RGB(red, green, blue));
        self.canvas
            .as_mut()
            .unwrap()
            .draw_point(SdlPoint::new(x as i32, y as i32))
            .unwrap();
    }

    fn set_blended_pixel(&mut self, x: u32, y: u32, mut color: Colour, alpha: f32) {
        if self.inverted() {
            color.invert();
        }

        let [red, green, blue] = color.rgb();
        self.canvas.as_mut().unwrap().set_draw_color(SdlColor::RGBA(
            red,
            green,
            blue,
            (alpha * 255.0) as u8,
        ));
        self.canvas
            .as_mut()
            .unwrap()
            .draw_point(SdlPoint::new(x as i32, y as i32))
            .unwrap();
    }

    fn invert_region(&mut self, rect: &Rectangle) {
        let rect = rect.clone();

        let width = rect.width();
        let s_rect = Some(SdlRect::new(rect.min.x, rect.min.y, width, rect.height()));
        if let Ok(data) = self
            .canvas
            .as_ref()
            .unwrap()
            .read_pixels(s_rect, PixelFormatEnum::RGB24)
        {
            for y in rect.min.y..rect.max.y {
                let v = (y - rect.min.y) as u32;
                for x in rect.min.x..rect.max.x {
                    let u = (x - rect.min.x) as u32;
                    let addr = 3 * (v * width + u);
                    let red = data[addr as usize];
                    let green = data[(addr + 1) as usize];
                    let blue = data[(addr + 2) as usize];
                    let mut color = Colour::Rgb(red, green, blue);
                    color.invert();
                    self.set_pixel(x as u32, y as u32, color);
                }
            }
        }
    }

    fn shift_region(&mut self, rect: &Rectangle, drift: u8) {
        let rect = rect.clone();

        let width = rect.width();
        let s_rect = Some(SdlRect::new(rect.min.x, rect.min.y, width, rect.height()));
        if let Ok(data) = self
            .canvas
            .as_ref()
            .unwrap()
            .read_pixels(s_rect, PixelFormatEnum::RGB24)
        {
            for y in rect.min.y..rect.max.y {
                let v = (y - rect.min.y) as u32;
                for x in rect.min.x..rect.max.x {
                    let u = (x - rect.min.x) as u32;
                    let addr = 3 * (v * width + u);
                    let red = data[addr as usize];
                    let green = data[(addr + 1) as usize];
                    let blue = data[(addr + 2) as usize];
                    let mut color = Colour::Rgb(red, green, blue);
                    color.shift(drift);
                    self.set_pixel(x as u32, y as u32, color);
                }
            }
        }
    }

    fn update(&mut self, _rect: &Rectangle, _mode: UpdateMode) -> Result<u32, Error> {
        self.canvas.as_mut().unwrap().present();
        Ok(Local::now().timestamp_subsec_millis())
    }

    fn wait(&self, _tok: u32) -> Result<i32, Error> {
        Ok(1)
    }

    fn save(&self, path: &str) -> Result<(), Error> {
        let (width, height) = self.dims();
        let file =
            File::create(path).with_context(|| format!("can't create output file {}", path))?;
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder
            .write_header()
            .with_context(|| format!("can't write PNG header for {}", path))?;
        let data = self
            .canvas
            .as_ref()
            .unwrap()
            .read_pixels(
                self.canvas.as_ref().unwrap().viewport(),
                PixelFormatEnum::RGB24,
            )
            .unwrap_or_default();
        writer
            .write_image_data(&data)
            .with_context(|| format!("can't write PNG data to {}", path))?;
        Ok(())
    }

    fn rotation(&self) -> i8 {
        self.rotation
    }

    fn set_rotation(&mut self, n: i8) -> Result<(u32, u32), Error> {
        let (mut width, mut height) = self.dims();
        if (width < height && n % 2 != 0) || (width > height && n % 2 != 1) {
            mem::swap(&mut width, &mut height);
        }

        // The canvas here has to be recreated after a resize event:
        // https://wiki.libsdl.org/SDL2/SDL_GetWindowSurface#remarks

        let mut win = self.canvas.take().unwrap().into_window();
        win.set_size(width, height).unwrap();
        let mut fb = win.into_canvas().software().build().unwrap();
        fb.set_blend_mode(BlendMode::Blend);
        fb.present();
        self.canvas.replace(fb);

        self.rotation = n;

        Ok((width, height))
    }

    fn set_monochrome(&mut self, _enable: bool) {}

    fn set_dithered(&mut self, _enable: bool) {}

    fn set_inverted(&mut self, enable: bool) {
        self.inverted = enable;
    }

    fn monochrome(&self) -> bool {
        false
    }

    fn dithered(&self) -> bool {
        false
    }

    fn inverted(&self) -> bool {
        self.inverted
    }

    fn width(&self) -> u32 {
        self.canvas.as_ref().unwrap().window().size().0
    }

    fn height(&self) -> u32 {
        self.canvas.as_ref().unwrap().window().size().1
    }
}
