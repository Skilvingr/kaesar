use kaesar_core::anyhow::Error;
use kaesar_core::battery::{Battery, FakeBattery};
use kaesar_core::context::Context;
use kaesar_core::device::CURRENT_DEVICE;
use kaesar_core::font::Fonts;
use kaesar_core::framebuffer::Framebuffer;
use kaesar_core::frontlight::{Frontlight, LightLevels};
use kaesar_core::helpers::load_toml;
use kaesar_core::library::Library;
use kaesar_core::lightsensor::LightSensor;
use kaesar_core::settings::Settings;
use sdl2::event::{Event as SdlEvent, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::render::BlendMode;
use std::env;
use std::fs::File;
use std::process::exit;
use std::sync::mpsc;
use std::thread;

use crate::events_sim::{power_btn_up_down, snow};
use crate::fb::FBCanvas;
use crate::unsafe_sync_cell::UnsafeSyncCell;

mod events_sim;
mod fb;
mod unsafe_sync_cell;

pub const APP_NAME: &str = "Kaesar";
const DEFAULT_ROTATION: i8 = 0;

pub fn build_context<F: Framebuffer + 'static>(fb: F) -> Result<Context, Error> {
    let settings = load_toml::<Settings, _>("emulator_sysroot/Settings.toml")?;
    let library_settings = &settings.libraries[settings.selected_library];
    let library = Library::new(&library_settings.path, library_settings.mode)?;

    let battery = Box::new(FakeBattery::new("emulator_sysroot/battery_sysfs")?) as Box<dyn Battery>;
    let frontlight = Box::new(LightLevels::default()) as Box<dyn Frontlight>;
    let lightsensor = Box::new(0u16) as Box<dyn LightSensor>;
    let fonts = Fonts::load()?;

    Ok(Context::new(
        fb,
        None,
        library,
        settings,
        fonts,
        battery,
        frontlight,
        lightsensor,
    ))
}

fn main() -> Result<(), Error> {
    // Will be searched by the core library in order to set
    // the right device settings
    unsafe {
        env::set_var("PRODUCT", "kaesar_simulator");
    }

    //crate::input::start_input();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let (width, height) = CURRENT_DEVICE.dims;
    let window = video_subsystem
        .window("Kaesar Emulator", width, height)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut fb = window.into_canvas().software().build().unwrap();
    fb.set_blend_mode(BlendMode::Blend);

    let (tx, rx) = mpsc::channel();

    // Little hack to send the SDL context in another thread.
    // Hope nothing will catch fire...
    let sdl_ctx = UnsafeSyncCell::new(sdl_context);
    thread::spawn(move || {
        let ctx = sdl_ctx.inner();

        // File used to mock input events. In an ordinary system would be /dev/input/event*
        let mut evt_file = File::create("emulator_sysroot/sim-touch-evts").unwrap();

        let mut power_key_down = false;
        let mut double_fingers_mod = false;
        loop {
            let mut event_pump = ctx.event_pump().unwrap();
            while let Some(sdl_evt) = event_pump.poll_event() {
                //println!("EVT: {:#?}", sdl_evt);

                match sdl_evt {
                    SdlEvent::Quit { .. } => {
                        exit(0);
                    }
                    SdlEvent::KeyDown {
                        timestamp, keycode, ..
                    } => {
                        if let Some(keycode) = keycode {
                            match keycode {
                                Keycode::LCTRL => {
                                    double_fingers_mod = true;
                                }
                                Keycode::P if !power_key_down => {
                                    power_key_down = true;
                                    power_btn_up_down(&mut evt_file, timestamp, false);
                                }
                                _ => {}
                            }
                        }
                    }

                    SdlEvent::KeyUp {
                        timestamp, keycode, ..
                    } => {
                        if let Some(keycode) = keycode {
                            match keycode {
                                Keycode::LCTRL => {
                                    double_fingers_mod = false;
                                }
                                Keycode::P => {
                                    power_key_down = false;
                                    power_btn_up_down(&mut evt_file, timestamp, true);
                                }
                                _ => {}
                            }
                        }
                    }
                    SdlEvent::MouseMotion {
                        timestamp,
                        mousestate,
                        x,
                        y,
                        ..
                    } if mousestate.is_mouse_button_pressed(MouseButton::Left) => {
                        snow::_finger_move(&mut evt_file, timestamp, x, y, double_fingers_mod);
                    }
                    SdlEvent::MouseButtonDown {
                        timestamp,
                        mouse_btn,
                        x,
                        y,
                        clicks,
                        ..
                    } if mouse_btn == MouseButton::Left => {
                        snow::_finger_up_down(
                            &mut evt_file,
                            timestamp,
                            x,
                            y,
                            false,
                            double_fingers_mod,
                        );
                    }
                    SdlEvent::MouseButtonUp {
                        timestamp,
                        mouse_btn,
                        x,
                        y,
                        clicks,
                        ..
                    } if mouse_btn == MouseButton::Left => {
                        snow::_finger_up_down(
                            &mut evt_file,
                            timestamp,
                            x,
                            y,
                            true,
                            double_fingers_mod,
                        );
                    }
                    SdlEvent::Window { win_event, .. } => {
                        if let WindowEvent::Resized(_, _) = win_event {
                            tx.send(()).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    let context = build_context(FBCanvas::new(fb)).unwrap();
    kaesar::run(context, 0, rx).unwrap();

    Ok(())
}
