use std::{
    mem,
    time::{Duration, Instant},
};

use fxhash::FxHashMap;

use crate::{
    context::Context,
    framebuffer::{Framebuffer, UpdateMode},
    geom::Rectangle,
    view::{Id, View},
};

pub struct UpdateData {
    pub token: u32,
    pub time: Instant,
    pub rect: Rectangle,
}

pub const MAX_UPDATE_DELAY: Duration = Duration::from_millis(600);

impl UpdateData {
    pub fn has_completed(&self) -> bool {
        self.time.elapsed() >= MAX_UPDATE_DELAY
    }
}

pub struct RenderData {
    pub id: Option<Id>,
    pub rect: Rectangle,
    pub mode: UpdateMode,
    pub wait: bool,
}

impl RenderData {
    pub fn new(id: Id, rect: Rectangle, mode: UpdateMode) -> RenderData {
        RenderData {
            id: Some(id),
            rect,
            mode,
            wait: true,
        }
    }

    pub fn no_wait(id: Id, rect: Rectangle, mode: UpdateMode) -> RenderData {
        RenderData {
            id: Some(id),
            rect,
            mode,
            wait: false,
        }
    }

    pub fn expose(rect: Rectangle, mode: UpdateMode) -> RenderData {
        RenderData {
            id: None,
            rect,
            mode,
            wait: true,
        }
    }

    pub fn rect(&self) -> Rectangle {
        self.rect
    }
}

pub struct RenderQueue(Option<FxHashMap<(UpdateMode, bool), Vec<(Option<Id>, Rectangle)>>>);

impl Default for RenderQueue {
    fn default() -> Self {
        Self(Some(Default::default()))
    }
}

impl RenderQueue {
    pub fn add_redraw_req(rq: &mut Option<Self>, data: RenderData) {
        if let Some(rq) = rq {
            rq.0.as_mut()
                .unwrap()
                .entry((data.mode, data.wait))
                .or_insert_with(|| Vec::new())
                .push((data.id, data.rect));
        }
    }
}

// We render from bottom to top. For a view to render it has to either appear in `ids` or intersect
// one of the rectangles in `bgs`. When we're about to render a view, if `wait` is true, we'll wait
// for all the updates in `updating` that intersect with the view.
pub fn render(
    view: &dyn View,
    wait: bool,
    ids: &FxHashMap<Id, Vec<Rectangle>>,
    rects: &mut Vec<Rectangle>,
    bgs: &mut Vec<Rectangle>,
    context: &mut Context,
    updating: &mut Vec<UpdateData>,
) {
    let mut render_rects = Vec::new();

    if view.len() == 0 || view.is_background() {
        for rect in ids
            .get(&view.id())
            .cloned()
            .into_iter()
            .flatten()
            .chain(rects.iter().filter_map(|r| r.intersection(view.rect())))
            .chain(bgs.iter().filter_map(|r| r.intersection(view.rect())))
        {
            let render_rect = view.render_rect(&rect);

            if wait {
                updating.retain(|update| {
                    let overlaps = render_rect.overlaps(&update.rect);
                    if overlaps && !update.has_completed() {
                        context
                            .fb
                            .wait(update.token)
                            .map_err(|e| {
                                eprintln!(
                                    "Can't wait for {}, {}: {:#}",
                                    update.token, update.rect, e
                                )
                            })
                            .ok();
                    }
                    !overlaps
                });
            }

            view.render_view(&rect, context);

            render_rects.push(render_rect);

            // Most views can't render a subrectangle of themselves.
            if *view.rect() == render_rect {
                break;
            }
        }
    } else {
        bgs.extend(ids.get(&view.id()).cloned().into_iter().flatten());
    }

    // Merge the contiguous zones to avoid having to schedule lots of small frambuffer updates.
    for rect in render_rects.into_iter() {
        if rects.is_empty() {
            rects.push(rect);
        } else {
            if let Some(last) = rects.last_mut() {
                if rect.extends(last) {
                    last.absorb(&rect);
                    let mut i = rects.len();
                    while i > 1 && rects[i - 1].extends(&rects[i - 2]) {
                        if let Some(rect) = rects.pop() {
                            if let Some(last) = rects.last_mut() {
                                last.absorb(&rect);
                            }
                        }
                        i -= 1;
                    }
                } else {
                    let mut i = rects.len();
                    while i > 0 && !rects[i - 1].contains(&rect) {
                        i -= 1;
                    }
                    if i == 0 {
                        rects.push(rect);
                    }
                }
            }
        }
    }

    for i in 0..view.len() {
        render(view.child(i), wait, ids, rects, bgs, context, updating);
    }
}

/// Suitable for emergency renderings: renders instantly a view, not worrying about
/// other views in the queue.
#[inline]
pub fn render_instantly(context: &mut Context, view: &dyn View) {
    let mut ids = FxHashMap::default();

    ids.insert(view.id(), vec![*view.rect()]);

    let mut rects = vec![];
    render(
        view,
        false,
        &ids,
        &mut rects,
        &mut vec![],
        context,
        &mut vec![],
    );

    for rect in rects {
        if let Err(err) = context.fb.update(&rect, UpdateMode::Full) {
            eprintln!("Can't update {}: {:#}.", rect, err);
        }
    }
}

#[inline]
pub fn process_render_queue(
    view: &dyn View,
    rq: &mut RenderQueue,
    context: &mut Context,
    updating: &mut Vec<UpdateData>,
) {
    let mut hm = FxHashMap::default();

    mem::swap(&mut hm, rq.0.as_mut().unwrap());

    for ((mode, wait), pairs) in hm.drain() {
        let mut ids = FxHashMap::default();
        let mut rects = Vec::new();
        let mut bgs = Vec::new();

        for (id, rect) in pairs.into_iter().rev() {
            if let Some(id) = id {
                ids.entry(id).or_insert_with(Vec::new).push(rect);
            } else {
                bgs.push(rect);
            }
        }

        render(view, wait, &ids, &mut rects, &mut bgs, context, updating);

        for rect in rects {
            match context.fb.update(&rect, mode) {
                Ok(token) => {
                    updating.push(UpdateData {
                        token,
                        rect,
                        time: Instant::now(),
                    });
                }
                Err(err) => {
                    eprintln!("Can't update {}: {:#}.", rect, err);
                }
            }
        }
    }
}

#[inline]
pub fn wait_for_all(fb: &mut dyn Framebuffer, updating: &mut Vec<UpdateData>) {
    for update in updating.drain(..) {
        if update.has_completed() {
            continue;
        }
        fb.wait(update.token)
            .map_err(|e| eprintln!("Can't wait for {}, {}: {:#}", update.token, update.rect, e))
            .ok();
    }
}
