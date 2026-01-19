use crate::view::renderer::RenderQueue;

use crate::colour::{BLACK, WHITE};
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::font::{NORMAL_STYLE, font_from_style};
use crate::framebuffer::UpdateMode;
use crate::geom::Rectangle;
use crate::input::gestures::GestureEvent;
use crate::view::{Bus, Event, Hub, ID_FEEDER, Id, RenderData, View, ViewId};

pub struct LibraryLabel {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    name: String,
    count: usize,
    filter: bool,
}

impl LibraryLabel {
    pub fn new(rect: Rectangle, name: &str, count: usize, filter: bool) -> LibraryLabel {
        LibraryLabel {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            name: name.to_string(),
            count,
            filter,
        }
    }

    pub fn update(
        &mut self,
        name: &str,
        count: usize,
        filter: bool,
        rendering_ctx: &mut Option<RenderQueue>,
    ) {
        let mut render = false;
        if self.name != name {
            self.name = name.to_string();
            render = true;
        }
        if self.count != count {
            self.count = count;
            render = true;
        }
        if self.filter != filter {
            self.filter = filter;
            render = true;
        }
        if render {
            RenderQueue::add_redraw_req(
                rendering_ctx,
                RenderData::new(self.id, self.rect, UpdateMode::Gui),
            );
        }
    }

    fn text(&self) -> String {
        let subject = if self.filter {
            if self.count != 1 { "matches" } else { "match" }
        } else {
            if self.count != 1 { "books" } else { "book" }
        };

        if self.count == 0 {
            format!("{} (No {})", self.name, subject)
        } else {
            format!("{} ({} {})", self.name, self.count, subject)
        }
    }
}

impl View for LibraryLabel {
    fn handle_event(
        &mut self,
        evt: &Event,
        _hub: &Hub,
        bus: &mut Bus,
        _rendering_ctx: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) -> bool {
        match *evt {
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
                bus.push_back(Event::ToggleNear(ViewId::LibraryMenu, self.rect));
                true
            }
            _ => false,
        }
    }

    fn render_view(&self, _rect: &Rectangle, ctx: &mut Context) {
        let dpi = CURRENT_DEVICE.dpi;
        let font = font_from_style(&mut ctx.fonts, &NORMAL_STYLE, dpi);
        let padding = font.em() as i32 / 2;
        let max_width = self.rect.width().saturating_sub(2 * padding as u32) as i32;
        let plan = font.plan(&self.text(), Some(max_width), None);
        let dx = padding + (max_width - plan.width) / 2;
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
