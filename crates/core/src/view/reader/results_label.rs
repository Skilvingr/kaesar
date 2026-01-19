use crate::view::renderer::RenderQueue;

use crate::colour::TEXT_NORMAL;
use crate::context::Context;
use crate::device::CURRENT_DEVICE;
use crate::font::{NORMAL_STYLE, font_from_style};
use crate::framebuffer::UpdateMode;
use crate::geom::Rectangle;
use crate::view::{Bus, Event, Hub, ID_FEEDER, Id, RenderData, View};

pub struct ResultsLabel {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    count: usize,
    completed: bool,
}

impl ResultsLabel {
    pub fn new(rect: Rectangle, count: usize, completed: bool) -> ResultsLabel {
        ResultsLabel {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            count,
            completed,
        }
    }

    pub fn update(&mut self, count: usize, rendering_ctx: &mut Option<RenderQueue>) {
        self.count = count;
        RenderQueue::add_redraw_req(
            rendering_ctx,
            RenderData::new(self.id, self.rect, UpdateMode::Gui),
        );
    }

    fn text(&self) -> String {
        let qualifier = if self.count != 1 { "results" } else { "result" };

        if self.count == 0 {
            format!("No {}", qualifier)
        } else {
            format!("{} {}", self.count, qualifier)
        }
    }
}

impl View for ResultsLabel {
    fn handle_event(
        &mut self,
        evt: &Event,
        _hub: &Hub,
        _bus: &mut Bus,
        rendering_ctx: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) -> bool {
        match *evt {
            Event::EndOfSearch => {
                self.completed = true;
                RenderQueue::add_redraw_req(
                    rendering_ctx,
                    RenderData::new(self.id, self.rect, UpdateMode::Gui),
                );
                false
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

        ctx.fb.draw_rectangle(&self.rect, TEXT_NORMAL[0]);

        let color = if self.completed {
            TEXT_NORMAL[1]
        } else {
            TEXT_NORMAL[2]
        };
        font.render(ctx.fb.as_mut(), color, &plan, pt);
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
