use crate::view::renderer::RenderQueue;

use super::{Bus, Event, Hub, ID_FEEDER, Id, RenderData, View, ViewId};
use crate::colour::{BLACK, WHITE};
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::font::{NORMAL_STYLE, font_from_style};
use crate::framebuffer::UpdateMode;
use crate::geom::Rectangle;
use crate::input::gestures::GestureEvent;
use chrono::{DateTime, Local};

pub struct Clock {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    format: String,
    time: DateTime<Local>,
}

impl Clock {
    pub fn new(rect: &mut Rectangle, context: &mut Context) -> Clock {
        let time = Local::now();
        let format = context.settings.time_format.clone();

        let font = font_from_style(&mut context.fonts, &NORMAL_STYLE, CURRENT_DEVICE.dpi);
        let width = font
            .plan(&time.format(&format).to_string(), None, None)
            .width
            + font.em() as i32;
        rect.min.x = rect.max.x - width;
        Clock {
            id: ID_FEEDER.next(),
            rect: *rect,
            children: Vec::new(),
            format,
            time,
        }
    }

    pub fn update(&mut self, rendering_ctx: &mut Option<RenderQueue>) {
        self.time = Local::now();

        RenderQueue::add_redraw_req(
            rendering_ctx,
            RenderData::new(self.id, self.rect, UpdateMode::Gui),
        );
    }
}

impl View for Clock {
    fn handle_event(
        &mut self,
        evt: &Event,
        _hub: &Hub,
        bus: &mut Bus,
        rendering_ctx: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) -> bool {
        match *evt {
            Event::ClockTick => {
                self.update(rendering_ctx);
                true
            }
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
                bus.push_back(Event::ToggleNear(ViewId::ClockMenu, self.rect));
                true
            }
            _ => false,
        }
    }

    fn render_view(&self, _rect: &Rectangle, ctx: &mut Context) {
        let dpi = CURRENT_DEVICE.dpi;
        let font = font_from_style(&mut ctx.fonts, &NORMAL_STYLE, dpi);
        let plan = font.plan(&self.time.format(&self.format).to_string(), None, None);
        let dx = (self.rect.width() as i32 - plan.width) / 2;
        let dy = (self.rect.height() as i32 - font.x_heights.0 as i32) / 2;
        let pt = pt!(self.rect.min.x + dx, self.rect.max.y - dy);

        ctx.fb.draw_rectangle(&self.rect, WHITE);
        font.render(ctx.fb.as_mut(), BLACK, &plan, pt);
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
