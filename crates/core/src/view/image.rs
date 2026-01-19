use crate::view::renderer::RenderQueue;

use crate::colour::WHITE;
use crate::context::Context;
use crate::framebuffer::{Pixmap, UpdateMode};
use crate::geom::Rectangle;
use crate::view::{Bus, Event, Hub, ID_FEEDER, Id, RenderData, View};

pub struct Image {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    pixmap: Pixmap,
}

impl Image {
    pub fn new(rect: Rectangle, pixmap: Pixmap) -> Image {
        Image {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            pixmap,
        }
    }

    pub fn update(&mut self, pixmap: Pixmap, rendering_ctx: &mut Option<RenderQueue>) {
        self.pixmap = pixmap;

        RenderQueue::add_redraw_req(
            rendering_ctx,
            RenderData::new(self.id, self.rect, UpdateMode::Gui),
        );
    }
}

impl View for Image {
    fn handle_event(
        &mut self,
        _evt: &Event,
        _hub: &Hub,
        _bus: &mut Bus,
        _rendering_ctx: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) -> bool {
        false
    }

    fn render_view(&self, rect: &Rectangle, ctx: &mut Context) {
        let x0 = self.rect.min.x + (self.rect.width() - self.pixmap.width) as i32 / 2;
        let y0 = self.rect.min.y + (self.rect.height() - self.pixmap.height) as i32 / 2;
        let x1 = x0 + self.pixmap.width as i32;
        let y1 = y0 + self.pixmap.height as i32;
        if let Some(r) = rect![self.rect.min, pt!(x1, y0)].intersection(&rect) {
            ctx.fb.draw_rectangle(&r, WHITE);
        }
        if let Some(r) = rect![self.rect.min.x, y0, x0, self.rect.max.y].intersection(&rect) {
            ctx.fb.draw_rectangle(&r, WHITE);
        }
        if let Some(r) = rect![pt!(x0, y1), self.rect.max].intersection(&rect) {
            ctx.fb.draw_rectangle(&r, WHITE);
        }
        if let Some(r) = rect![x1, self.rect.min.y, self.rect.max.x, y1].intersection(&rect) {
            ctx.fb.draw_rectangle(&r, WHITE);
        }
        if let Some(r) = rect![x0, y0, x1, y1].intersection(&rect) {
            let frame = r - pt!(x0, y0);

            ctx.fb.draw_framed_pixmap(&self.pixmap, &frame, r.min);
        }
    }

    fn render_rect(&self, rect: &Rectangle) -> Rectangle {
        rect.intersection(&self.rect).unwrap_or(self.rect)
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
