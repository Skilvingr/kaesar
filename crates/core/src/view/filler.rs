use crate::view::renderer::RenderQueue;

use super::{Bus, Event, Hub, ID_FEEDER, Id, View};
use crate::colour::Colour;
use crate::context::Context;
use crate::geom::Rectangle;

pub struct Filler {
    id: Id,
    pub rect: Rectangle,
    children: Vec<Box<dyn View>>,
    color: Colour,
}

impl Filler {
    pub fn new(rect: Rectangle, color: Colour) -> Filler {
        Filler {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            color,
        }
    }
}

impl View for Filler {
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
        if let Some(r) = self.rect.intersection(rect) {
            ctx.fb.draw_rectangle(&r, self.color);
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
