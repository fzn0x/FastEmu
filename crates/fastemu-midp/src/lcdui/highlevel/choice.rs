//! `javax.microedition.lcdui.Choice`, the interface `List` and
//! `ChoiceGroup` share, and the elements both keep: strings, images,
//! fonts, which are selected, and which one has the focus.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value};

use super::snapshot;
use crate::lcdui::font::{default_font, font_object};

/// The interface.
pub const CHOICE: &str = "javax/microedition/lcdui/Choice";

/// `Choice.EXCLUSIVE`.
pub const EXCLUSIVE: i32 = 1;
/// `Choice.MULTIPLE`.
pub const MULTIPLE: i32 = 2;
/// `Choice.IMPLICIT`.
pub const IMPLICIT: i32 = 3;
/// `Choice.POPUP`.
pub const POPUP: i32 = 4;
/// `Choice.TEXT_WRAP_DEFAULT`.
pub const TEXT_WRAP_DEFAULT: i32 = 0;
/// `Choice.TEXT_WRAP_ON`.
pub const TEXT_WRAP_ON: i32 = 1;
/// `Choice.TEXT_WRAP_OFF`.
pub const TEXT_WRAP_OFF: i32 = 2;

/// One element: its string, image and font, and whether it is selected.
#[derive(Clone, Copy, Debug)]
pub struct Element {
    pub text: ObjRef,
    pub image: Option<ObjRef>,
    pub font: Option<ObjRef>,
    pub selected: bool,
}

/// The elements of a `List` or `ChoiceGroup`.
///
/// In the exclusive kinds (`EXCLUSIVE`, `IMPLICIT`, `POPUP`) exactly one
/// element is selected while there are any; in `IMPLICIT` lists the
/// selection follows the focus, so a command fired from a soft key sees
/// the element the user is on.
#[derive(Debug)]
pub struct ChoiceModel {
    pub kind: i32,
    pub elements: Vec<Element>,
    pub fit_policy: i32,
    /// The element with the focus, kept inside the elements.
    pub focus: usize,
}

impl ChoiceModel {
    /// An empty choice of `kind`.
    pub fn new(kind: i32) -> Self {
        Self {
            kind,
            elements: Vec::new(),
            fit_policy: TEXT_WRAP_DEFAULT,
            focus: 0,
        }
    }

    /// Marks the references the elements hold.
    pub fn trace(&self, tracer: &mut Tracer<'_>) {
        for e in &self.elements {
            tracer.mark(e.text);
            tracer.mark_opt(e.image);
            tracer.mark_opt(e.font);
        }
    }

    /// Whether one element is always selected.
    pub fn exclusive(&self) -> bool {
        self.kind != MULTIPLE
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// `getSelectedIndex()`: the selected element of an exclusive choice,
    /// -1 when empty and always for `MULTIPLE`.
    pub fn selected_index(&self) -> i32 {
        if !self.exclusive() {
            return -1;
        }
        self.elements
            .iter()
            .position(|e| e.selected)
            .map_or(-1, |i| i as i32)
    }

    /// Keeps an exclusive choice with one selection and the focus inside.
    fn settle(&mut self) {
        self.focus = self.focus.min(self.elements.len().saturating_sub(1));
        if self.exclusive() && !self.elements.is_empty() && self.selected_index() < 0 {
            let first = if self.kind == IMPLICIT { self.focus } else { 0 };
            self.elements[first].selected = true;
        }
    }

    /// Inserts an element at `at`, which must be at most the length.
    pub fn insert(&mut self, at: usize, text: ObjRef, image: Option<ObjRef>) {
        self.elements.insert(
            at,
            Element {
                text,
                image,
                font: None,
                selected: false,
            },
        );
        if at <= self.focus && self.elements.len() > 1 {
            self.focus += 1;
        }
        self.settle();
    }

    /// Removes the element at `at`, an index inside. A selected element's
    /// selection passes to the one that takes its place.
    pub fn delete(&mut self, at: usize) {
        let removed = self.elements.remove(at);
        if at < self.focus {
            self.focus -= 1;
        }
        if removed.selected && self.exclusive() && !self.elements.is_empty() {
            let next = at.min(self.elements.len() - 1);
            self.elements[next].selected = true;
        }
        self.settle();
    }

    /// Removes every element.
    pub fn clear(&mut self) {
        self.elements.clear();
        self.focus = 0;
    }

    /// `setSelectedIndex(at, selected)`. Deselecting in an exclusive
    /// choice does nothing.
    pub fn set_selected(&mut self, at: usize, selected: bool) {
        if self.exclusive() {
            if !selected {
                return;
            }
            for (i, e) in self.elements.iter_mut().enumerate() {
                e.selected = i == at;
            }
            if self.kind == IMPLICIT {
                self.focus = at;
            }
        } else {
            self.elements[at].selected = selected;
        }
    }

    /// `setSelectedFlags(flags)`: an exclusive choice selects the first
    /// set flag, or its first element when none is set.
    pub fn set_flags(&mut self, flags: &[bool]) {
        if self.exclusive() {
            let at = flags.iter().position(|&f| f).unwrap_or(0);
            if at < self.elements.len() {
                self.set_selected(at, true);
            }
        } else {
            for (e, &f) in self.elements.iter_mut().zip(flags) {
                e.selected = f;
            }
        }
    }

    /// Moves the focus to `at`; an `IMPLICIT` list's selection follows.
    pub fn focus_on(&mut self, at: usize) {
        if at < self.elements.len() {
            self.focus = at;
            if self.kind == IMPLICIT {
                self.set_selected(at, true);
            }
        }
    }

    /// What the select key does on the focused element: selects it in an
    /// exclusive choice, flips it in a `MULTIPLE` one. Returns whether
    /// the selection changed.
    pub fn choose_focused(&mut self) -> bool {
        let at = self.focus;
        let Some(e) = self.elements.get(at) else {
            return false;
        };
        if self.exclusive() {
            let changed = !e.selected;
            self.set_selected(at, true);
            changed
        } else {
            self.elements[at].selected = !e.selected;
            true
        }
    }
}

/// How the shared methods reach an object's elements.
#[derive(Clone, Copy)]
pub(crate) struct Access {
    pub model: for<'a, 'b> fn(&'a mut Ctx<'b>, ObjRef) -> JResult<&'a mut ChoiceModel>,
    /// Runs after a change: repaints the list, or lays out the form.
    pub changed: fn(&mut Ctx<'_>, ObjRef) -> JResult<()>,
}

fn out_of_bounds<T>(ctx: &mut Ctx<'_>, index: i32) -> JResult<T> {
    ctx.throw(
        "java/lang/IndexOutOfBoundsException",
        &format!("element {index}"),
    )
}

/// The index `index` if it names an element (or, with `end`, the place
/// after the last one).
fn index(ctx: &mut Ctx<'_>, this: ObjRef, access: Access, index: i32, end: bool) -> JResult<usize> {
    let len = (access.model)(ctx, this)?.len() as i64;
    let limit = if end { len + 1 } else { len };
    if !(0..limit).contains(&i64::from(index)) {
        return out_of_bounds(ctx, index);
    }
    Ok(index as usize)
}

/// The flags array argument: `NullPointerException` for null, and
/// `IllegalArgumentException` when shorter than the choice.
fn flags_array(ctx: &mut Ctx<'_>, this: ObjRef, access: Access, v: Value) -> JResult<ObjRef> {
    let array = ctx.non_null(v)?;
    let len = (access.model)(ctx, this)?.len();
    if ctx.array_len(array)? < len {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "the array is shorter than the choice",
        );
    }
    Ok(array)
}

/// Checks that `choiceType` is one `allowed` lists.
pub(crate) fn check_type(ctx: &mut Ctx<'_>, kind: i32, allowed: &[i32]) -> JResult<()> {
    if allowed.contains(&kind) {
        return Ok(());
    }
    ctx.throw(
        "java/lang/IllegalArgumentException",
        &format!("invalid choice type {kind}"),
    )
}

/// Fills a new choice from the constructor's `String[]` and `Image[]`.
pub(crate) fn fill(
    ctx: &mut Ctx<'_>,
    this: ObjRef,
    access: Access,
    strings: Value,
    images: Value,
) -> JResult<()> {
    let strings = ctx.non_null(strings)?;
    let texts = ctx.array::<Option<ObjRef>>(strings)?.to_vec();
    let pictures = match images.obj() {
        Some(images) => {
            let pictures = ctx.array::<Option<ObjRef>>(images)?.to_vec();
            if pictures.len() != texts.len() {
                return ctx.throw(
                    "java/lang/IllegalArgumentException",
                    "the image array's length differs from the string array's",
                );
            }
            pictures
        }
        None => vec![None; texts.len()],
    };
    if texts.iter().any(Option::is_none) {
        return ctx.throw("java/lang/NullPointerException", "a null string element");
    }
    for (text, image) in texts.into_iter().zip(pictures) {
        let image = snapshot(ctx, image)?;
        let model = (access.model)(ctx, this)?;
        let at = model.len();
        model.insert(at, text.expect("checked above"), image);
    }
    Ok(())
}

/// Adds the `Choice` methods to `class`.
pub(crate) fn methods(class: NativeClass, access: Access) -> NativeClass {
    let class = class
        .method("size", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.model)(ctx, this)?.len() as i32))
        })
        .method("getString", "(I)Ljava/lang/String;", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let at = index(ctx, this, access, a[1].int(), false)?;
            Ok((access.model)(ctx, this)?.elements[at].text.into())
        })
        .method(
            "getImage",
            "(I)Ljavax/microedition/lcdui/Image;",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let at = index(ctx, this, access, a[1].int(), false)?;
                Ok((access.model)(ctx, this)?.elements[at].image.into())
            },
        )
        .method(
            "append",
            "(Ljava/lang/String;Ljavax/microedition/lcdui/Image;)I",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let text = ctx.non_null(a[1])?;
                let image = snapshot(ctx, a[2].obj())?;
                let model = (access.model)(ctx, this)?;
                let at = model.len();
                model.insert(at, text, image);
                (access.changed)(ctx, this)?;
                Ok(Value::Int(at as i32))
            },
        )
        .method(
            "insert",
            "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let at = index(ctx, this, access, a[1].int(), true)?;
                let text = ctx.non_null(a[2])?;
                let image = snapshot(ctx, a[3].obj())?;
                (access.model)(ctx, this)?.insert(at, text, image);
                (access.changed)(ctx, this)?;
                Ok(Value::Void)
            },
        )
        .method("delete", "(I)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let at = index(ctx, this, access, a[1].int(), false)?;
            (access.model)(ctx, this)?.delete(at);
            (access.changed)(ctx, this)?;
            Ok(Value::Void)
        })
        .method("deleteAll", "()V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            (access.model)(ctx, this)?.clear();
            (access.changed)(ctx, this)?;
            Ok(Value::Void)
        })
        .method(
            "set",
            "(ILjava/lang/String;Ljavax/microedition/lcdui/Image;)V",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let at = index(ctx, this, access, a[1].int(), false)?;
                let text = ctx.non_null(a[2])?;
                let image = snapshot(ctx, a[3].obj())?;
                let e = &mut (access.model)(ctx, this)?.elements[at];
                e.text = text;
                e.image = image;
                (access.changed)(ctx, this)?;
                Ok(Value::Void)
            },
        );
    selection_methods(class, access)
}

fn selection_methods(class: NativeClass, access: Access) -> NativeClass {
    class
        .method("isSelected", "(I)Z", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let at = index(ctx, this, access, a[1].int(), false)?;
            Ok(Value::from(
                (access.model)(ctx, this)?.elements[at].selected,
            ))
        })
        .method("getSelectedIndex", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.model)(ctx, this)?.selected_index()))
        })
        .method("getSelectedFlags", "([Z)I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let array = flags_array(ctx, this, access, a[1])?;
            let flags: Vec<bool> = (access.model)(ctx, this)?
                .elements
                .iter()
                .map(|e| e.selected)
                .collect();
            let out = ctx.array_mut::<i8>(array)?;
            out.fill(0);
            for (slot, &f) in out.iter_mut().zip(&flags) {
                *slot = i8::from(f);
            }
            Ok(Value::Int(flags.iter().filter(|&&f| f).count() as i32))
        })
        .method("setSelectedIndex", "(IZ)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let at = index(ctx, this, access, a[1].int(), false)?;
            (access.model)(ctx, this)?.set_selected(at, a[2].boolean());
            (access.changed)(ctx, this)?;
            Ok(Value::Void)
        })
        .method("setSelectedFlags", "([Z)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let array = flags_array(ctx, this, access, a[1])?;
            let flags: Vec<bool> = ctx.array::<i8>(array)?.iter().map(|&f| f != 0).collect();
            (access.model)(ctx, this)?.set_flags(&flags);
            (access.changed)(ctx, this)?;
            Ok(Value::Void)
        })
        .method("setFitPolicy", "(I)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let policy = a[1].int();
            if !(TEXT_WRAP_DEFAULT..=TEXT_WRAP_OFF).contains(&policy) {
                return ctx.throw("java/lang/IllegalArgumentException", "invalid fit policy");
            }
            (access.model)(ctx, this)?.fit_policy = policy;
            (access.changed)(ctx, this)?;
            Ok(Value::Void)
        })
        .method("getFitPolicy", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.model)(ctx, this)?.fit_policy))
        })
        .method(
            "setFont",
            "(ILjavax/microedition/lcdui/Font;)V",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let at = index(ctx, this, access, a[1].int(), false)?;
                (access.model)(ctx, this)?.elements[at].font = a[2].obj();
                (access.changed)(ctx, this)?;
                Ok(Value::Void)
            },
        )
        .method(
            "getFont",
            "(I)Ljavax/microedition/lcdui/Font;",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let at = index(ctx, this, access, a[1].int(), false)?;
                match (access.model)(ctx, this)?.elements[at].font {
                    Some(font) => Ok(font.into()),
                    None => {
                        let font = default_font(ctx);
                        Ok(font_object(ctx, &font)?.into())
                    }
                }
            },
        )
}

/// The `Choice` interface.
pub fn interface() -> NativeClass {
    let int = |v| Value::Int(v);
    let image = "Ljavax/microedition/lcdui/Image;";
    NativeClass::interface(CHOICE)
        .constant("EXCLUSIVE", "I", int(EXCLUSIVE))
        .constant("MULTIPLE", "I", int(MULTIPLE))
        .constant("IMPLICIT", "I", int(IMPLICIT))
        .constant("POPUP", "I", int(POPUP))
        .constant("TEXT_WRAP_DEFAULT", "I", int(TEXT_WRAP_DEFAULT))
        .constant("TEXT_WRAP_ON", "I", int(TEXT_WRAP_ON))
        .constant("TEXT_WRAP_OFF", "I", int(TEXT_WRAP_OFF))
        .abstract_method("size", "()I")
        .abstract_method("getString", "(I)Ljava/lang/String;")
        .abstract_method("getImage", &format!("(I){image}"))
        .abstract_method("append", &format!("(Ljava/lang/String;{image})I"))
        .abstract_method("insert", &format!("(ILjava/lang/String;{image})V"))
        .abstract_method("delete", "(I)V")
        .abstract_method("deleteAll", "()V")
        .abstract_method("set", &format!("(ILjava/lang/String;{image})V"))
        .abstract_method("isSelected", "(I)Z")
        .abstract_method("getSelectedIndex", "()I")
        .abstract_method("getSelectedFlags", "([Z)I")
        .abstract_method("setSelectedIndex", "(IZ)V")
        .abstract_method("setSelectedFlags", "([Z)V")
        .abstract_method("setFitPolicy", "(I)V")
        .abstract_method("getFitPolicy", "()I")
        .abstract_method("setFont", "(ILjavax/microedition/lcdui/Font;)V")
        .abstract_method("getFont", "(I)Ljavax/microedition/lcdui/Font;")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;

    /// Runs `f` with `n` distinct strings standing for element texts; the
    /// model never looks inside them.
    fn with_texts(n: usize, f: impl FnOnce(&[ObjRef]) + Send + 'static) {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            Vec::new(),
        );
        vm.attach("test", move |ctx| {
            let texts = (0..n)
                .map(|i| ctx.new_string(&i.to_string()))
                .collect::<JResult<Vec<_>>>()?;
            f(&texts);
            JResult::Ok(())
        })
        .expect("the test ran");
    }

    fn model(kind: i32, texts: &[ObjRef]) -> ChoiceModel {
        let mut m = ChoiceModel::new(kind);
        for &t in texts {
            let at = m.len();
            m.insert(at, t, None);
        }
        m
    }

    #[test]
    fn exclusive_choices_keep_one_selection() {
        with_texts(3, |t| {
            let mut m = model(EXCLUSIVE, t);
            assert_eq!(m.selected_index(), 0);
            m.set_selected(2, true);
            assert_eq!(m.selected_index(), 2);
            m.set_selected(2, false);
            assert_eq!(m.selected_index(), 2, "deselecting does nothing");
            m.delete(2);
            assert_eq!(m.selected_index(), 1, "the selection moves on");
            m.clear();
            assert_eq!(m.selected_index(), -1);
        });
    }

    #[test]
    fn multiple_choices_select_freely() {
        with_texts(3, |t| {
            let mut m = model(MULTIPLE, t);
            assert_eq!(m.selected_index(), -1);
            m.set_flags(&[true, false, true]);
            assert!(m.elements[0].selected && m.elements[2].selected);
            m.focus_on(1);
            assert!(m.choose_focused());
            assert!(m.elements[1].selected);
            assert!(m.choose_focused());
            assert!(!m.elements[1].selected);
        });
    }

    #[test]
    fn implicit_selection_follows_the_focus() {
        with_texts(4, |t| {
            let mut m = model(IMPLICIT, t);
            m.focus_on(3);
            assert_eq!(m.selected_index(), 3);
            m.set_selected(1, true);
            assert_eq!(m.focus, 1);
            m.delete(0);
            assert_eq!((m.focus, m.selected_index()), (0, 0));
        });
    }

    #[test]
    fn flags_pick_the_first_set_one_or_the_first_element() {
        with_texts(3, |t| {
            let mut m = model(EXCLUSIVE, t);
            m.set_flags(&[false, true, true]);
            assert_eq!(m.selected_index(), 1);
            m.set_flags(&[false, false, false]);
            assert_eq!(m.selected_index(), 0);
        });
    }

    #[test]
    fn inserting_before_the_focus_keeps_it_on_its_element() {
        with_texts(3, |t| {
            let mut m = model(EXCLUSIVE, &t[..2]);
            m.focus_on(1);
            m.insert(0, t[2], None);
            assert_eq!(m.focus, 2);
        });
    }
}
