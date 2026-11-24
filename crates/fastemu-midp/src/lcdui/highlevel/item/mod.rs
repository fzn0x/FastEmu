//! `javax.microedition.lcdui.Item`, the base of everything a `Form`
//! holds, its listeners, and what the form asks of each item: its size,
//! its pixels, and what it does with keys, pointer events and text.
//!
//! Every item object carries one [`ItemData`]: what all items share
//! (label, layout, commands, preferred size, owner) and its kind's own
//! state ([`Kind`]). Each kind lives in a module of its own with its
//! class.

pub mod choice_group;
pub mod custom;
pub mod date;
pub mod gauge;
pub mod image;
pub mod spacer;
pub mod string;
pub mod text_field;

use fastemu_core::PointerAction;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::draw::{Fonts, PAD, Painter, wrap};
use super::form;
use super::text::TextEditor;
use crate::lcdui::Rect;
use crate::lcdui::display;
use crate::lcdui::event::callback;
use crate::lcdui::screen::KeyEvent;
use crate::lcdui::state::state;

/// The class.
pub const ITEM: &str = "javax/microedition/lcdui/Item";
const COMMAND_LISTENER: &str = "javax/microedition/lcdui/ItemCommandListener";
const STATE_LISTENER: &str = "javax/microedition/lcdui/ItemStateListener";
const COMMAND_ACTION: &str = "(Ljavax/microedition/lcdui/Command;Ljavax/microedition/lcdui/Item;)V";
const ILLEGAL_STATE: &str = "java/lang/IllegalStateException";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";

/// `Item.LAYOUT_LEFT`.
pub const LAYOUT_LEFT: i32 = 1;
/// `Item.LAYOUT_RIGHT`.
pub const LAYOUT_RIGHT: i32 = 2;
/// `Item.LAYOUT_CENTER`.
pub const LAYOUT_CENTER: i32 = 3;
/// `Item.LAYOUT_TOP`.
pub const LAYOUT_TOP: i32 = 0x10;
/// `Item.LAYOUT_BOTTOM`.
pub const LAYOUT_BOTTOM: i32 = 0x20;
/// `Item.LAYOUT_VCENTER`.
pub const LAYOUT_VCENTER: i32 = 0x30;
/// `Item.LAYOUT_NEWLINE_BEFORE`.
pub const LAYOUT_NEWLINE_BEFORE: i32 = 0x100;
/// `Item.LAYOUT_NEWLINE_AFTER`.
pub const LAYOUT_NEWLINE_AFTER: i32 = 0x200;
/// `Item.LAYOUT_SHRINK`.
pub const LAYOUT_SHRINK: i32 = 0x400;
/// `Item.LAYOUT_EXPAND`.
pub const LAYOUT_EXPAND: i32 = 0x800;
/// `Item.LAYOUT_VSHRINK`.
pub const LAYOUT_VSHRINK: i32 = 0x1000;
/// `Item.LAYOUT_VEXPAND`.
pub const LAYOUT_VEXPAND: i32 = 0x2000;
/// `Item.LAYOUT_2`.
pub const LAYOUT_2: i32 = 0x4000;
/// Every layout bit MIDP defines.
const LAYOUT_BITS: i32 = 0x3 | LAYOUT_VCENTER | 0x7F00;

/// `Item.PLAIN`.
pub const PLAIN: i32 = 0;
/// `Item.HYPERLINK`.
pub const HYPERLINK: i32 = 1;
/// `Item.BUTTON`.
pub const BUTTON: i32 = 2;

/// The space above and below an item's content.
pub const VPAD: i32 = 2;

/// What one kind of item keeps.
pub enum Kind {
    /// The abstract `Item` itself, which no game can make.
    Base,
    String(string::StringState),
    Image(image::ImageState),
    Spacer(spacer::SpacerState),
    Text(TextEditor),
    Choice(choice_group::GroupState),
    Gauge(gauge::GaugeState),
    Date(date::DateState),
    Custom(custom::CustomState),
}

/// Which [`Kind`] an item is, without borrowing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Base,
    String,
    Image,
    Spacer,
    Text,
    Choice,
    Gauge,
    Date,
    Custom,
}

/// What every `Item` object holds.
pub struct ItemData {
    pub label: Option<ObjRef>,
    pub layout: i32,
    pub commands: Vec<ObjRef>,
    pub default_command: Option<ObjRef>,
    /// The `ItemCommandListener`.
    pub listener: Option<ObjRef>,
    /// `setPreferredSize`: -1 where unlocked.
    pub preferred: (i32, i32),
    /// The `Form` or `Alert` that holds the item.
    pub owner: Option<ObjRef>,
    /// Whether the owner is an `Alert` (the item is its indicator).
    pub in_alert: bool,
    pub kind: Kind,
}

impl ItemData {
    /// A fresh item of `kind`.
    pub fn new(kind: Kind) -> Self {
        Self {
            label: None,
            layout: 0,
            commands: Vec::new(),
            default_command: None,
            listener: None,
            preferred: (-1, -1),
            owner: None,
            in_alert: false,
            kind,
        }
    }

    /// Which kind it is.
    pub fn tag(&self) -> Tag {
        match self.kind {
            Kind::Base => Tag::Base,
            Kind::String(_) => Tag::String,
            Kind::Image(_) => Tag::Image,
            Kind::Spacer(_) => Tag::Spacer,
            Kind::Text(_) => Tag::Text,
            Kind::Choice(_) => Tag::Choice,
            Kind::Gauge(_) => Tag::Gauge,
            Kind::Date(_) => Tag::Date,
            Kind::Custom(_) => Tag::Custom,
        }
    }
}

impl Payload for ItemData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.label);
        tracer.mark_opt(self.default_command);
        tracer.mark_opt(self.listener);
        tracer.mark_opt(self.owner);
        for &c in &self.commands {
            tracer.mark(c);
        }
        match &self.kind {
            Kind::Base | Kind::Spacer(_) | Kind::Text(_) | Kind::Gauge(_) | Kind::Date(_) => {}
            Kind::String(s) => s.trace(tracer),
            Kind::Image(s) => s.trace(tracer),
            Kind::Choice(s) => s.model.trace(tracer),
            Kind::Custom(s) => s.trace(tracer),
        }
    }
}

/// The payload of an item.
pub fn data<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a ItemData> {
    ctx.payload::<ItemData>(item)
}

/// The payload of an item, mutably.
pub fn data_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut ItemData> {
    ctx.payload_mut::<ItemData>(item)
}

/// The kind of an item.
pub fn tag(ctx: &Ctx<'_>, item: ObjRef) -> JResult<Tag> {
    Ok(data(ctx, item)?.tag())
}

/// The text of a Java string, empty for `None`.
pub(crate) fn units(ctx: &Ctx<'_>, s: Option<ObjRef>) -> JResult<Vec<u16>> {
    match s {
        Some(s) => Ok(ctx.string_utf16(s)?.to_vec()),
        None => Ok(Vec::new()),
    }
}

/// The layout a form gives the item. Without `LAYOUT_2` an item has a
/// row of its own, as MIDP 1.0 forms looked and as J2ME Loader and
/// freej2me lay every item out; with it, items share rows, and a
/// `StringItem`'s text that starts or ends with a line feed breaks the
/// row there.
pub(crate) fn effective_layout(ctx: &Ctx<'_>, item: ObjRef) -> JResult<i32> {
    let d = data(ctx, item)?;
    let mut layout = d.layout;
    if layout & LAYOUT_2 == 0 {
        return Ok(layout | LAYOUT_NEWLINE_BEFORE | LAYOUT_NEWLINE_AFTER);
    }
    if let Kind::String(s) = &d.kind {
        let text = units(ctx, s.text)?;
        let line_feed = u16::from(b'\n');
        if text.first() == Some(&line_feed) {
            layout |= LAYOUT_NEWLINE_BEFORE;
        }
        if text.last() == Some(&line_feed) {
            layout |= LAYOUT_NEWLINE_AFTER;
        }
    }
    Ok(layout)
}

/// Tells the item's owner it changed: a form lays itself out again, an
/// alert repaints.
pub(crate) fn changed(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    let d = data(ctx, item)?;
    match (d.owner, d.in_alert) {
        (Some(owner), true) => display::repaint(ctx, owner),
        (Some(owner), false) => form::item_changed(ctx, owner),
        (None, _) => Ok(()),
    }
}

/// Throws `IllegalStateException` for an item an `Alert` holds, which
/// may not change in the ways its own methods ask.
fn not_in_alert(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    if data(ctx, item)?.in_alert {
        return ctx.throw(ILLEGAL_STATE, "the item belongs to an Alert");
    }
    Ok(())
}

/// What an item needs, label and padding included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Needs {
    pub min_width: i32,
    pub width: i32,
    pub height: i32,
}

/// The fonts and widest width items are measured with outside a paint:
/// the screen less a form's margins.
pub(crate) fn measure_fonts(ctx: &mut Ctx<'_>) -> (Fonts, i32) {
    let st = state(ctx);
    (Fonts::new(&st.fonts), form::content_width(st.screen_width))
}

/// The label's lines and their height at `width`.
fn label_height(ctx: &Ctx<'_>, item: ObjRef, fonts: &Fonts, width: i32) -> JResult<i32> {
    let label = units(ctx, data(ctx, item)?.label)?;
    if label.is_empty() {
        return Ok(0);
    }
    Ok(wrap(&fonts.label, &label, width).len() as i32 * fonts.label.height() + 1)
}

/// The content's needs, by kind, at most `width` wide.
fn content(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, width: i32) -> JResult<Needs> {
    match tag(ctx, item)? {
        Tag::Base => Ok(Needs {
            min_width: 0,
            width: 0,
            height: 0,
        }),
        Tag::String => string::needs(ctx, item, fonts, width),
        Tag::Image => image::needs(ctx, item, fonts, width),
        Tag::Spacer => spacer::needs(ctx, item),
        Tag::Text => text_field::needs(ctx, item, fonts, width),
        Tag::Choice => choice_group::needs(ctx, item, fonts, width),
        Tag::Gauge => Ok(gauge::needs(fonts, width)),
        Tag::Date => date::needs(ctx, item, fonts, width),
        Tag::Custom => custom::needs(ctx, item, width),
    }
}

/// What the item needs when it may be up to `width` wide: its label above
/// its content, padded, unless `setPreferredSize` locked a side. A
/// `Spacer` is exactly its size.
pub(crate) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, width: i32) -> JResult<Needs> {
    if tag(ctx, item)? == Tag::Spacer {
        return spacer::needs(ctx, item);
    }
    let inner = (width - 2 * PAD).max(1);
    let body = content(ctx, item, fonts, inner)?;
    let label = units(ctx, data(ctx, item)?.label)?;
    let label_width = super::draw::width(&fonts.label, &label).min(inner);
    let label_h = label_height(ctx, item, fonts, inner)?;
    let min_width = (body.min_width + 2 * PAD).min(width);
    let natural = (body.width.max(label_width) + 2 * PAD).clamp(min_width, width.max(min_width));
    let (lock_w, lock_h) = data(ctx, item)?.preferred;
    let chosen = if lock_w >= 0 {
        lock_w.clamp(min_width, width.max(min_width))
    } else {
        natural
    };
    let height = if lock_h >= 0 {
        lock_h.max(label_h + 2 * VPAD)
    } else {
        label_h + body.height + 2 * VPAD
    };
    Ok(Needs {
        min_width,
        width: chosen,
        height,
    })
}

/// The height the item takes when given `width`.
pub(crate) fn height_at(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    fonts: &Fonts,
    width: i32,
) -> JResult<i32> {
    if tag(ctx, item)? == Tag::Spacer {
        return Ok(spacer::needs(ctx, item)?.height);
    }
    let inner = (width - 2 * PAD).max(1);
    let (_, lock_h) = data(ctx, item)?.preferred;
    let label_h = label_height(ctx, item, fonts, inner)?;
    if lock_h >= 0 {
        return Ok(lock_h.max(label_h + 2 * VPAD));
    }
    let body = content(ctx, item, fonts, inner)?;
    Ok(label_h + body.height + 2 * VPAD)
}

/// Where the content of an item laid out in `area` goes, below its label.
pub(crate) fn content_area(
    ctx: &Ctx<'_>,
    item: ObjRef,
    fonts: &Fonts,
    area: Rect,
) -> JResult<Rect> {
    if tag(ctx, item)? == Tag::Spacer {
        return Ok(area);
    }
    let inner = (area.width - 2 * PAD).max(1);
    let label_h = label_height(ctx, item, fonts, inner)?;
    Ok(Rect::new(
        area.x + PAD,
        area.y + VPAD + label_h,
        inner,
        (area.height - label_h - 2 * VPAD).max(0),
    ))
}

/// Paints the item in `area`, its label first and a frame when it has the
/// focus.
pub(crate) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let tag = tag(ctx, item)?;
    if tag == Tag::Spacer {
        return Ok(());
    }
    if focused {
        let theme = p.theme;
        p.frame(area, theme.highlighted_border, 1);
    }
    let label = units(ctx, data(ctx, item)?.label)?;
    let inner = (area.width - 2 * PAD).max(1);
    if !label.is_empty() {
        let font = p.fonts.label.clone();
        let color = p.theme.foreground;
        p.paragraph(&font, &label, area.x + PAD, area.y + VPAD, inner, color);
    }
    let content = content_area(ctx, item, &p.fonts, area)?;
    p.clip(content.intersect(area));
    let painted = match tag {
        Tag::Base | Tag::Spacer => Ok(()),
        Tag::String => string::paint(ctx, item, p, content, focused),
        Tag::Image => image::paint(ctx, item, p, content, focused),
        Tag::Text => text_field::paint(ctx, item, p, content, focused),
        Tag::Choice => choice_group::paint(ctx, item, p, content, focused),
        Tag::Gauge => gauge::paint(ctx, item, p, content, focused),
        Tag::Date => date::paint(ctx, item, p, content, focused),
        Tag::Custom => custom::paint(ctx, item, p, content),
    };
    p.unclip();
    painted
}

/// Whether the focus may rest on the item.
pub(crate) fn focusable(ctx: &Ctx<'_>, item: ObjRef) -> JResult<bool> {
    let d = data(ctx, item)?;
    Ok(match &d.kind {
        Kind::Base | Kind::Spacer(_) => false,
        Kind::Text(_) | Kind::Choice(_) | Kind::Date(_) | Kind::Custom(_) => true,
        Kind::Gauge(g) => g.interactive || !d.commands.is_empty(),
        Kind::String(_) | Kind::Image(_) => !d.commands.is_empty(),
    })
}

/// Whether the item takes typed text while it has the focus, so the app
/// should send characters rather than keypad keys.
pub(crate) fn takes_text(ctx: &Ctx<'_>, item: ObjRef) -> JResult<bool> {
    Ok(match &data(ctx, item)?.kind {
        Kind::Text(editor) => editor.editable(),
        Kind::Date(_) => true,
        _ => false,
    })
}

/// What an item did with an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handled {
    /// Not for the item.
    No,
    /// Used, nothing the listener hears about changed.
    Yes,
    /// Used, and the item's state changed: the form's
    /// `ItemStateListener` hears of it.
    Changed,
}

/// A key for the focused item. `form` is its owner.
pub(crate) fn key(
    ctx: &mut Ctx<'_>,
    form: ObjRef,
    item: ObjRef,
    event: KeyEvent,
) -> JResult<Handled> {
    let handled = match tag(ctx, item)? {
        Tag::Text => text_field::key(ctx, item, event)?,
        Tag::Choice => choice_group::key(ctx, form, item, event)?,
        Tag::Gauge => gauge::key(ctx, item, event)?,
        Tag::Date => date::key(ctx, item, event)?,
        Tag::Custom => custom::key(ctx, item, event)?,
        Tag::Base | Tag::String | Tag::Image | Tag::Spacer => Handled::No,
    };
    if handled != Handled::No {
        return Ok(handled);
    }
    let fire = event.game_action == crate::lcdui::keys::FIRE
        && event.phase == crate::lcdui::screen::KeyPhase::Pressed;
    if fire && activate(ctx, item)? {
        return Ok(Handled::Yes);
    }
    Ok(Handled::No)
}

/// The select key on an item with commands: its default command, or its
/// only command, goes to its `ItemCommandListener`. Returns whether one
/// did.
fn activate(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<bool> {
    let d = data(ctx, item)?;
    let command = d
        .default_command
        .or_else(|| (d.commands.len() == 1).then(|| d.commands[0]));
    match command {
        Some(command) => {
            let command = ctx.root(command);
            fire_command(ctx, item, command)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Text typed while the item has the focus.
pub(crate) fn text(ctx: &mut Ctx<'_>, item: ObjRef, typed: &str) -> JResult<Handled> {
    match tag(ctx, item)? {
        Tag::Text => text_field::text(ctx, item, typed),
        Tag::Date => date::text(ctx, item, typed),
        _ => Ok(Handled::No),
    }
}

/// A pointer event on the item, at `(x, y)` in its content area of
/// `size`.
pub(crate) fn pointer(
    ctx: &mut Ctx<'_>,
    form: ObjRef,
    item: ObjRef,
    action: PointerAction,
    (x, y): (i32, i32),
    size: (i32, i32),
) -> JResult<Handled> {
    match tag(ctx, item)? {
        Tag::Choice => choice_group::pointer(ctx, form, item, action, y, size.0),
        Tag::Gauge => gauge::pointer(ctx, item, action, x, size),
        Tag::Custom => custom::pointer(ctx, item, action, x, y),
        Tag::String | Tag::Image if action == PointerAction::Release => {
            Ok(if activate(ctx, item)? {
                Handled::Yes
            } else {
                Handled::No
            })
        }
        _ => Ok(Handled::No),
    }
}

/// Sends `command` to the item's `ItemCommandListener`, as a callback.
pub(crate) fn fire_command(ctx: &mut Ctx<'_>, item: ObjRef, command: ObjRef) -> JResult<()> {
    let Some(listener) = data(ctx, item)?.listener else {
        return Ok(());
    };
    let listener = ctx.root(listener);
    callback(ctx, "an item's commandAction", |ctx| {
        ctx.call_interface(
            listener,
            "commandAction",
            COMMAND_ACTION,
            &[command.into(), item.into()],
        )
        .map(drop)
    })
}

/// The commands the soft keys offer while the item has the focus.
pub(crate) fn commands(ctx: &Ctx<'_>, item: ObjRef) -> JResult<Vec<ObjRef>> {
    Ok(data(ctx, item)?.commands.clone())
}

fn this(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    ctx.non_null(v)
}

/// `setLabel`, which `Spacer` and the items of an `Alert` refuse.
pub(crate) fn set_label(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    not_in_alert(ctx, item)?;
    data_mut(ctx, item)?.label = a[1].obj();
    changed(ctx, item)?;
    Ok(Value::Void)
}

/// `setLayout`, checking the directives.
pub(crate) fn set_layout(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let layout = a[1].int();
    if layout & !LAYOUT_BITS != 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, "invalid layout directives");
    }
    not_in_alert(ctx, item)?;
    data_mut(ctx, item)?.layout = layout;
    changed(ctx, item)?;
    Ok(Value::Void)
}

/// Whether `layout` is a combination of MIDP's layout directives.
pub(crate) fn valid_layout(layout: i32) -> bool {
    layout & !LAYOUT_BITS == 0
}

/// `getLayout`.
pub(crate) fn get_layout(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    Ok(Value::Int(data(ctx, item)?.layout))
}

/// `addCommand`.
pub(crate) fn add_command(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let command = ctx.non_null(a[1])?;
    not_in_alert(ctx, item)?;
    let d = data_mut(ctx, item)?;
    if !d.commands.contains(&command) {
        d.commands.push(command);
    }
    commands_changed(ctx, item)
}

/// `setDefaultCommand`, which also adds the command.
pub(crate) fn set_default_command(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    not_in_alert(ctx, item)?;
    let command = a[1].obj();
    let d = data_mut(ctx, item)?;
    d.default_command = command;
    if let Some(command) = command
        && !d.commands.contains(&command)
    {
        d.commands.push(command);
    }
    commands_changed(ctx, item)
}

/// `setItemCommandListener`.
pub(crate) fn set_listener(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    not_in_alert(ctx, item)?;
    data_mut(ctx, item)?.listener = a[1].obj();
    Ok(Value::Void)
}

/// `setPreferredSize`.
pub(crate) fn set_preferred_size(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let (w, h) = (a[1].int(), a[2].int());
    if w < -1 || h < -1 {
        return ctx.throw(ILLEGAL_ARGUMENT, "a preferred size below -1");
    }
    not_in_alert(ctx, item)?;
    data_mut(ctx, item)?.preferred = (w, h);
    changed(ctx, item)?;
    Ok(Value::Void)
}

/// The item's commands changed: the soft keys may show others, and the
/// item may now take or lose the focus.
fn commands_changed(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<Value> {
    if let Some(owner) = data(ctx, item)?.owner {
        form::commands_changed(ctx, owner)?;
    }
    changed(ctx, item)?;
    Ok(Value::Void)
}

/// `getPreferredWidth` (`width`) or `getPreferredHeight`.
fn preferred(ctx: &mut Ctx<'_>, a: &[Value], width: bool) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let (fonts, max) = measure_fonts(ctx);
    let needs = needs(ctx, item, &fonts, max)?;
    Ok(Value::Int(if width {
        needs.width
    } else {
        height_at(ctx, item, &fonts, needs.width)?
    }))
}

/// `getMinimumWidth` (`width`) or `getMinimumHeight`.
fn minimum(ctx: &mut Ctx<'_>, a: &[Value], width: bool) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let (fonts, max) = measure_fonts(ctx);
    let needs = needs(ctx, item, &fonts, max)?;
    Ok(Value::Int(if width {
        needs.min_width
    } else {
        height_at(ctx, item, &fonts, max)?
    }))
}

/// `notifyStateChanged()`: the form's `ItemStateListener` hears of the
/// item on the event thread, after the call returns.
fn notify_state_changed(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = this(ctx, a[0])?;
    let d = data(ctx, item)?;
    let Some(form) = d.owner.filter(|_| !d.in_alert) else {
        return ctx.throw(ILLEGAL_STATE, "the item is not in a Form");
    };
    let (item, form) = (ctx.new_global(item), ctx.new_global(form));
    ctx.post(move |ctx| {
        let (item, form) = (item.get(ctx), form.get(ctx));
        form::notify_state(ctx, form, item)
    });
    Ok(Value::Void)
}

/// The class `Item`.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(ITEM)
        .abstract_class()
        .payload_with(|| Box::new(ItemData::new(Kind::Base)))
        .constant("LAYOUT_DEFAULT", "I", int(0))
        .constant("LAYOUT_LEFT", "I", int(LAYOUT_LEFT))
        .constant("LAYOUT_RIGHT", "I", int(LAYOUT_RIGHT))
        .constant("LAYOUT_CENTER", "I", int(LAYOUT_CENTER))
        .constant("LAYOUT_TOP", "I", int(LAYOUT_TOP))
        .constant("LAYOUT_BOTTOM", "I", int(LAYOUT_BOTTOM))
        .constant("LAYOUT_VCENTER", "I", int(LAYOUT_VCENTER))
        .constant("LAYOUT_NEWLINE_BEFORE", "I", int(LAYOUT_NEWLINE_BEFORE))
        .constant("LAYOUT_NEWLINE_AFTER", "I", int(LAYOUT_NEWLINE_AFTER))
        .constant("LAYOUT_SHRINK", "I", int(LAYOUT_SHRINK))
        .constant("LAYOUT_EXPAND", "I", int(LAYOUT_EXPAND))
        .constant("LAYOUT_VSHRINK", "I", int(LAYOUT_VSHRINK))
        .constant("LAYOUT_VEXPAND", "I", int(LAYOUT_VEXPAND))
        .constant("LAYOUT_2", "I", int(LAYOUT_2))
        .constant("PLAIN", "I", int(PLAIN))
        .constant("HYPERLINK", "I", int(HYPERLINK))
        .constant("BUTTON", "I", int(BUTTON))
        .method_with("<init>", "()V", 0, |_, _| Ok(Value::Void))
        .method("setLabel", "(Ljava/lang/String;)V", set_label)
        .method("getLabel", "()Ljava/lang/String;", |ctx, a| {
            let item = this(ctx, a[0])?;
            Ok(data(ctx, item)?.label.into())
        })
        .method("getLayout", "()I", get_layout)
        .method("setLayout", "(I)V", set_layout)
        .method(
            "addCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            add_command,
        )
        .method(
            "removeCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, a| {
                let item = this(ctx, a[0])?;
                let Some(command) = a[1].obj() else {
                    return Ok(Value::Void);
                };
                let d = data_mut(ctx, item)?;
                d.commands.retain(|&c| c != command);
                if d.default_command == Some(command) {
                    d.default_command = None;
                }
                commands_changed(ctx, item)
            },
        )
        .method(
            "setItemCommandListener",
            "(Ljavax/microedition/lcdui/ItemCommandListener;)V",
            set_listener,
        )
        .method("getPreferredWidth", "()I", |ctx, a| preferred(ctx, a, true))
        .method("getPreferredHeight", "()I", |ctx, a| {
            preferred(ctx, a, false)
        })
        .method("setPreferredSize", "(II)V", set_preferred_size)
        .method("getMinimumWidth", "()I", |ctx, a| minimum(ctx, a, true))
        .method("getMinimumHeight", "()I", |ctx, a| minimum(ctx, a, false))
        .method(
            "setDefaultCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            set_default_command,
        )
        .method("notifyStateChanged", "()V", notify_state_changed)
}

/// The classes of this module: `Item`, its listeners and every kind.
pub fn classes() -> Vec<NativeClass> {
    vec![
        class(),
        NativeClass::interface(COMMAND_LISTENER).abstract_method("commandAction", COMMAND_ACTION),
        NativeClass::interface(STATE_LISTENER)
            .abstract_method("itemStateChanged", "(Ljavax/microedition/lcdui/Item;)V"),
        string::class(),
        image::class(),
        spacer::class(),
        text_field::class(),
        choice_group::class(),
        gauge::class(),
        date::class(),
        custom::class(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_directives_combine() {
        assert!(valid_layout(0));
        assert!(valid_layout(
            LAYOUT_CENTER | LAYOUT_VCENTER | LAYOUT_NEWLINE_AFTER | LAYOUT_EXPAND | LAYOUT_2
        ));
        assert!(!valid_layout(0x8000));
        assert!(!valid_layout(4));
    }
}
