use crate::view::renderer::RenderQueue;

use super::{BORDER_RADIUS_LARGE, THICKNESS_MEDIUM};
use super::{Bus, Event, Hub, ID_FEEDER, Id, RenderData, View};
use crate::colour::{TEXT_INVERTED_HARD, TEXT_NORMAL};
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::font::{NORMAL_STYLE, font_from_style};
use crate::framebuffer::UpdateMode;
use crate::geom::{BorderSpec, CornerSpec, Rectangle};
use crate::input::gestures::GestureEvent;
use crate::input::{DeviceEvent, FingerStatus};
use crate::unit::scale_by_dpi;

pub struct Button {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    event: Event,
    text: String,
    active: bool,
    pub disabled: bool,
}

impl Button {
    pub fn new(rect: Rectangle, event: Event, text: String) -> Button {
        Button {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            event,
            text,
            active: false,
            disabled: false,
        }
    }

    pub fn disabled(mut self, value: bool) -> Button {
        self.disabled = value;
        self
    }
}

impl View for Button {
    fn handle_event(
        &mut self,
        evt: &Event,
        _hub: &Hub,
        bus: &mut Bus,
        rendering_ctx: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) -> bool {
        match *evt {
            Event::Device(DeviceEvent::Finger {
                status, position, ..
            }) if !self.disabled => match status {
                FingerStatus::Down if self.rect.includes(position) => {
                    self.active = true;

                    RenderQueue::add_redraw_req(
                        rendering_ctx,
                        RenderData::new(self.id, self.rect, UpdateMode::Fast),
                    );
                    true
                }
                FingerStatus::Up if self.active => {
                    self.active = false;

                    RenderQueue::add_redraw_req(
                        rendering_ctx,
                        RenderData::new(self.id, self.rect, UpdateMode::Gui),
                    );
                    true
                }
                _ => false,
            },
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
                if !self.disabled {
                    bus.push_back(self.event.clone());
                }
                true
            }
            _ => false,
        }
    }

    fn render_view(&self, _rect: &Rectangle, ctx: &mut Context) {
        let dpi = CURRENT_DEVICE.dpi;

        let scheme = if self.active {
            TEXT_INVERTED_HARD
        } else {
            TEXT_NORMAL
        };
        let foreground = if self.disabled { scheme[2] } else { scheme[1] };

        let border_radius = scale_by_dpi(BORDER_RADIUS_LARGE, dpi) as i32;
        let border_thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as u16;

        ctx.fb.draw_rounded_rectangle_with_border(
            &self.rect,
            &CornerSpec::Uniform(border_radius),
            &BorderSpec {
                thickness: border_thickness,
                color: foreground,
            },
            &scheme[0],
        );

        let font = font_from_style(&mut ctx.fonts, &NORMAL_STYLE, dpi);
        let x_height = font.x_heights.0 as i32;
        let padding = font.em() as i32;
        let max_width = self.rect.width() as i32 - padding;

        let plan = font.plan(&self.text, Some(max_width), None);

        let dx = (self.rect.width() as i32 - plan.width) / 2;
        let dy = (self.rect.height() as i32 - x_height) / 2;
        let pt = pt!(self.rect.min.x + dx, self.rect.max.y - dy);

        font.render(ctx.fb.as_mut(), foreground, &plan, pt);
    }

    fn rect(&self) -> &Rectangle {
        &self.rect
    }

    fn rect_mut(&mut self) -> &mut Rectangle {
        &mut self.rect
    }

    fn children(&self) -> &Vec<Box<dyn View>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        &mut self.children
    }

    fn id(&self) -> Id {
        self.id
    }
}
