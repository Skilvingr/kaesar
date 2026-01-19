//! Views are organized as a tree. A view might receive / send events and render itself.
//!
//! The z-level of the n-th child of a view is less or equal to the z-level of its n+1-th child.
//!
//! Events travel from the root to the leaves, only the leaf views will handle the root events, but
//! any view can send events to its parent. From the events it receives from its children, a view
//! resends the ones it doesn't handle to its own parent. Hence an event sent from a child might
//! bubble up to the root. If it reaches the root without being captured by any view, then it will
//! be written to the main event channel and will be sent to every leaf in one of the next loop
//! iterations.

pub mod battery;
pub mod button;
pub mod calculator;
pub mod clock;
pub mod common;
pub mod dialog;
pub mod dictionary;
pub mod filler;
pub mod frontlight;
pub mod home;
pub mod icon;
pub mod image;
pub mod input_field;
pub mod intermission;
pub mod key;
pub mod keyboard;
pub mod label;
pub mod labeled_icon;
pub mod menu;
pub mod menu_entry;
pub mod named_input;
pub mod notification;
pub mod page_label;
pub mod preset;
pub mod presets_list;
pub mod reader;
pub mod renderer;
pub mod rotation_values;
pub mod rounded_button;
pub mod search_bar;
pub mod sketch;
pub mod slider;
pub mod top_bar;
pub mod touch_events;

use self::calculator::LineOrigin;
use self::key::KeyKind;
use crate::colour::Colour;
use crate::context::Context;
use crate::document::{Location, TextLocation};
use crate::framebuffer::UpdateMode;
use crate::geom::{Boundary, CycleDir, LinearDir, Rectangle};
use crate::input::gestures::GestureEvent;
use crate::input::{DeviceEvent, FingerStatus};
use crate::metadata::{
    Info, Margin, PageScheme, ScrollMode, SimpleStatus, SortMethod, TextAlign, ZoomMode,
};
use crate::settings::{ButtonScheme, FirstColumn, RotationLock, SecondColumn};
use crate::view::renderer::RenderData;
use crate::view::renderer::RenderQueue;
use downcast_rs::{Downcast, impl_downcast};
use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

// Border thicknesses in pixels, at 300 DPI.
pub const THICKNESS_SMALL: f32 = 1.0;
pub const THICKNESS_MEDIUM: f32 = 2.0;
pub const THICKNESS_LARGE: f32 = 3.0;

// Border radii in pixels, at 300 DPI.
pub const BORDER_RADIUS_SMALL: f32 = 6.0;
pub const BORDER_RADIUS_MEDIUM: f32 = 9.0;
pub const BORDER_RADIUS_LARGE: f32 = 12.0;

// Big and small bar heights in pixels, at 300 DPI.
// On the *Aura ONE*, the height is exactly `2 * sb + 10 * bb`.
pub const SMALL_BAR_HEIGHT: f32 = 121.0;
pub const BIG_BAR_HEIGHT: f32 = 163.0;

pub const CLOSE_IGNITION_DELAY: Duration = Duration::from_millis(150);

pub type Bus = VecDeque<Event>;
pub type Hub = Sender<Event>;

pub trait View: Downcast + Send + Sync {
    /// Return true in order to capture the event.
    fn handle_event(
        &mut self,
        evt: &Event,
        hub: &Hub,
        bus: &mut Bus,
        rendering_rendering_ctx: &mut Option<RenderQueue>,
        context: &mut Context,
    ) -> bool;
    //fn render(&self, fb: &mut dyn Framebuffer, rect: Rectangle, fonts: &mut Fonts) {}
    fn render_view(&self, _rect: &Rectangle, _ctx: &mut Context) {}

    fn rect(&self) -> &Rectangle;
    fn rect_mut(&mut self) -> &mut Rectangle;
    fn children(&self) -> &Vec<Box<dyn View>>;
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>>;
    fn id(&self) -> Id;

    fn render_rect(&self, _rect: &Rectangle) -> Rectangle {
        *self.rect()
    }

    fn resize(
        &mut self,
        rect: Rectangle,
        _hub: &Hub,
        _rq: &mut Option<RenderQueue>,
        _context: &mut Context,
    ) {
        *self.rect_mut() = rect;
    }

    fn child(&self, index: usize) -> &dyn View {
        self.children()[index].as_ref()
    }

    fn child_mut(&mut self, index: usize) -> &mut dyn View {
        self.children_mut()[index].as_mut()
    }

    fn len(&self) -> usize {
        self.children().len()
    }

    fn might_skip(&self, _evt: &Event) -> bool {
        false
    }

    fn might_rotate(&self) -> bool {
        true
    }

    fn is_background(&self) -> bool {
        false
    }

    fn view_id(&self) -> Option<ViewId> {
        None
    }
}

impl_downcast!(View);

impl Debug for Box<dyn View> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Box<dyn View>")
    }
}

// We start delivering events from the highest z-level to prevent views from capturing
// gestures that occurred in higher views.
// The consistency must also be ensured by the views: popups, for example, need to
// capture any tap gesture with a touch point inside their rectangle, by returning `true`.
// A child can send events to the main channel through the *hub* or communicate with its parent through the *bus*.
// A view that wants to render can write to the rendering queue.
pub fn handle_event(
    view: &mut dyn View,
    evt: &Event,
    hub: &Hub,
    parent_bus: &mut Bus,
    rendering_ctx: &mut Option<RenderQueue>,
    context: &mut Context,
) -> bool {
    if view.len() > 0 {
        let mut captured = false;

        if view.might_skip(evt) {
            return captured;
        }

        let mut child_bus: Bus = VecDeque::with_capacity(1);

        for i in (0..view.len()).rev() {
            if handle_event(
                view.child_mut(i),
                evt,
                hub,
                &mut child_bus,
                rendering_ctx,
                context,
            ) {
                captured = true;
                break;
            }
        }

        let mut temp_bus: Bus = VecDeque::with_capacity(1);

        child_bus.retain(|child_evt| {
            !view.handle_event(child_evt, hub, &mut temp_bus, rendering_ctx, context)
        });

        parent_bus.append(&mut child_bus);
        parent_bus.append(&mut temp_bus);

        captured || view.handle_event(evt, hub, parent_bus, rendering_ctx, context)
    } else {
        view.handle_event(evt, hub, parent_bus, rendering_ctx, context)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Device(DeviceEvent),
    Gesture(GestureEvent),
    Keyboard(KeyboardEvent),
    Key(KeyKind),
    Open(Box<Info>),
    OpenHtml(String, Option<String>),
    LoadPixmap(usize),
    Update(UpdateMode),
    RefreshBookPreview(PathBuf, Option<PathBuf>),
    Invalid(PathBuf),
    Notify(String),
    Page(CycleDir),
    ResultsPage(CycleDir),
    GoTo(usize),
    GoToLocation(Location),
    ResultsGoTo(usize),
    CropMargins(Box<Margin>),
    Chapter(CycleDir),
    SelectDirectory(PathBuf),
    ToggleSelectDirectory(PathBuf),
    NavigationBarResized(i32),
    Focus(Option<ViewId>),
    Select(EntryId),
    PropagateSelect(EntryId),
    EditLanguages,
    Define(String),
    Submit(ViewId, String),
    Slider(SliderId, f32, FingerStatus),
    ToggleNear(ViewId, Rectangle),
    ToggleInputHistoryMenu(ViewId, Rectangle),
    ToggleBookMenu(Rectangle, usize),
    TogglePresetMenu(Rectangle, usize),
    SubMenu(Rectangle, Vec<EntryKind>),
    ProcessLine(LineOrigin, String),
    History(CycleDir, bool),
    Toggle(ViewId),
    Show(ViewId),
    Close(ViewId),
    CloseSub(ViewId),
    Search(String),
    SearchResult(usize, Vec<Boundary>),
    FetcherAddDocument(u32, Box<Info>),
    FetcherRemoveDocument(u32, PathBuf),
    FetcherSearch {
        id: u32,
        path: Option<PathBuf>,
        query: Option<String>,
        sort_by: Option<(SortMethod, bool)>,
    },
    CheckFetcher(u32),
    EndOfSearch,
    Finished,
    ClockTick,
    BatteryTick,
    ToggleFrontlight,
    Load(PathBuf),
    LoadPreset(usize),
    Scroll(i32),
    Save,
    Guess,
    SetWifi(bool),
    MightSuspend,
    PrepareSuspend,
    Suspend,
    Share,
    PrepareShare,
    Validate,
    Cancel,
    Reseed,
    SshUp(&'static str),
    Back,
    Quit,
    WakeUp,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AppCmd {
    Sketch,
    Calculator,
    Dictionary { query: String, language: String },
    TouchEvents,
    RotationValues,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum ViewId {
    Home,
    Reader,
    SortMenu,
    MainMenu,
    TitleMenu,
    SelectionMenu,
    AnnotationMenu,
    BatteryMenu,
    ClockMenu,
    SearchTargetMenu,
    InputHistoryMenu,
    KeyboardLayoutMenu,
    Frontlight,
    Dictionary,
    FontSizeMenu,
    TextAlignMenu,
    FontFamilyMenu,
    MarginWidthMenu,
    ContrastExponentMenu,
    ContrastGreyMenu,
    LineHeightMenu,
    DirectoryMenu,
    BookMenu,
    LibraryMenu,
    PageMenu,
    PresetMenu,
    MarginCropperMenu,
    SearchMenu,
    SketchMenu,
    RenameDocument,
    RenameDocumentInput,
    GoToPage,
    GoToPageInput,
    GoToResultsPage,
    GoToResultsPageInput,
    NamePage,
    NamePageInput,
    EditNote,
    EditNoteInput,
    EditLanguages,
    EditLanguagesInput,
    HomeSearchInput,
    ReaderSearchInput,
    DictionarySearchInput,
    CalculatorInput,
    SearchBar,
    AddressBar,
    AddressBarInput,
    Keyboard,
    AboutDialog,
    ShareDialog,
    MarginCropper,
    TopBottomBars,
    TableOfContents,
    MessageNotif(Id),
    SubMenu(u8),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SliderId {
    FontSize,
    LightIntensity,
    LightWarmth,
    ContrastExponent,
    ContrastGrey,
}

impl SliderId {
    pub fn label(self) -> String {
        match self {
            SliderId::LightIntensity => "Intensity".to_string(),
            SliderId::LightWarmth => "Warmth".to_string(),
            SliderId::FontSize => "Font Size".to_string(),
            SliderId::ContrastExponent => "Contrast Exponent".to_string(),
            SliderId::ContrastGrey => "Contrast Grey".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Align {
    Left(i32),
    Right(i32),
    Center,
}

impl Align {
    #[inline]
    pub fn offset(&self, width: i32, container_width: i32) -> i32 {
        match *self {
            Align::Left(dx) => dx,
            Align::Right(dx) => container_width - width - dx,
            Align::Center => (container_width - width) / 2,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum KeyboardEvent {
    Append(char),
    Partial(char),
    Move { target: TextKind, dir: LinearDir },
    Delete { target: TextKind, dir: LinearDir },
    Submit,
}

#[derive(Debug, Copy, Clone)]
pub enum TextKind {
    Char,
    Word,
    Extremum,
}

#[derive(Debug, Clone)]
pub enum EntryKind {
    Message(String, Option<String>),
    Command(String, EntryId),
    CheckBox(String, EntryId, bool),
    RadioButton(String, EntryId, bool),
    SubMenu(String, Vec<EntryKind>),
    More(Vec<EntryKind>),
    Separator,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EntryId {
    About,
    SystemInfo,
    LoadLibrary(usize),
    Load(PathBuf),
    Flush,
    Save,
    Import,
    CleanUp,
    Sort(SortMethod),
    ReverseOrder,
    EmptyTrash,
    Rename(PathBuf),
    Remove(PathBuf),
    CopyTo(PathBuf, usize),
    MoveTo(PathBuf, usize),
    AddDirectory(PathBuf),
    SelectDirectory(PathBuf),
    ToggleSelectDirectory(PathBuf),
    SetStatus(PathBuf, SimpleStatus),
    SearchAuthor(String),
    RemovePreset(usize),
    FirstColumn(FirstColumn),
    SecondColumn(SecondColumn),
    ThumbnailPreviews,
    ApplyCroppings(usize, PageScheme),
    RemoveCroppings,
    SetZoomMode(ZoomMode),
    SetScrollMode(ScrollMode),
    SetPageName,
    RemovePageName,
    HighlightSelection,
    AnnotateSelection,
    DefineSelection,
    SearchForSelection,
    AdjustSelection,
    Annotations,
    Bookmarks,
    RemoveAnnotation([TextLocation; 2]),
    EditAnnotationNote([TextLocation; 2]),
    RemoveAnnotationNote([TextLocation; 2]),
    GoTo(usize),
    GoToSelectedPageName,
    SearchDirection(LinearDir),
    SetButtonScheme(ButtonScheme),
    SetFontFamily(String),
    SetFontSize(i32),
    SetTextAlign(TextAlign),
    SetMarginWidth(i32),
    SetLineHeight(i32),
    SetContrastExponent(i32),
    SetContrastGrey(i32),
    SetRotationLock(Option<RotationLock>),
    SetSearchTarget(Option<String>),
    SetInputText(ViewId, String),
    SetKeyboardLayout(String),
    ToggleShowHidden,
    ToggleFuzzy,
    ToggleInverted,
    ToggleDocumentInverted,
    ToggleDithered,
    ToggleWifi,
    ToggleSSH,
    Rotate(i8),
    Launch(AppCmd),
    SetPenSize(i32),
    SetPenColor(Colour),
    TogglePenDynamism,
    ReloadDictionaries,
    New,
    Refresh,
    TakeScreenshot,
    RestartApp,
    Reboot,
    Quit,
}

impl EntryKind {
    pub fn is_separator(&self) -> bool {
        matches!(*self, EntryKind::Separator)
    }

    pub fn text(&self) -> &str {
        match *self {
            EntryKind::Message(ref s, ..)
            | EntryKind::Command(ref s, ..)
            | EntryKind::CheckBox(ref s, ..)
            | EntryKind::RadioButton(ref s, ..)
            | EntryKind::SubMenu(ref s, ..) => s,
            EntryKind::More(..) => "More",
            _ => "",
        }
    }

    pub fn get(&self) -> Option<bool> {
        match *self {
            EntryKind::CheckBox(_, _, v) | EntryKind::RadioButton(_, _, v) => Some(v),
            _ => None,
        }
    }

    pub fn set(&mut self, value: bool) {
        match *self {
            EntryKind::CheckBox(_, _, ref mut v) | EntryKind::RadioButton(_, _, ref mut v) => {
                *v = value
            }
            _ => (),
        }
    }
}

pub static ID_FEEDER: IdFeeder = IdFeeder::new(1);
pub struct IdFeeder(AtomicU64);
pub type Id = u64;

impl IdFeeder {
    pub const fn new(id: Id) -> Self {
        IdFeeder(AtomicU64::new(id))
    }

    pub fn next(&self) -> Id {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}
