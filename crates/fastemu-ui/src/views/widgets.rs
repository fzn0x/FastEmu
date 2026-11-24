//! Small hand-styled controls the views share: icon buttons, text
//! buttons, option chips, switches and tooltips.

use gpui::{
    AnyView, App, Context, Div, ElementId, Hsla, IntoElement, ParentElement, Render, SharedString,
    Stateful, Styled, Svg, Window, div, prelude::*, px, svg,
};

use crate::assets::Icon;
use crate::theme::Theme;

/// An icon in one colour.
pub fn icon(icon: Icon, size: f32, color: Hsla) -> Svg {
    svg()
        .path(icon.path())
        .size(px(size))
        .flex_none()
        .text_color(color)
}

/// A square button showing only an icon, with a tooltip naming it.
pub fn icon_button(
    id: impl Into<ElementId>,
    which: Icon,
    label: &'static str,
    theme: &Theme,
) -> Stateful<Div> {
    toggle_icon_button(id, which, label, false, theme)
}

/// An icon button that shows its state in the accent colour while on.
pub fn toggle_icon_button(
    id: impl Into<ElementId>,
    which: Icon,
    label: &'static str,
    on: bool,
    theme: &Theme,
) -> Stateful<Div> {
    let group: SharedString = "icon-button".into();
    let (rest, lit, hover) = if on {
        (theme.accent, theme.accent, theme.hover)
    } else {
        (theme.muted, theme.text, theme.hover)
    };
    div()
        .id(id)
        .group(group.clone())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(26.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .when(on, |d| d.bg(theme.raised))
        .hover(move |s| s.bg(hover))
        .tooltip(move |_, cx| tooltip(label, cx))
        .child(icon(which, 15.0, rest).group_hover(group, move |s| s.text_color(lit)))
}

/// A button with a text label on the raised colour.
pub fn text_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: &Theme,
) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(28.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .bg(theme.raised)
        .text_color(theme.text)
        .text_sm()
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(label.into())
}

/// The one call to action of a view, in the accent colour.
pub fn primary_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: &Theme,
) -> Stateful<Div> {
    let accent = theme.accent;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(30.0))
        .px(px(12.0))
        .rounded(px(6.0))
        .bg(accent)
        .text_color(theme.on_accent)
        .text_sm()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(move |s| s.bg(accent.opacity(0.85)))
        .child(label.into())
}

/// One option of a group, filled when selected.
pub fn chip(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
) -> Stateful<Div> {
    let hover = theme.hover;
    let base = div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(10.0))
        .rounded(px(13.0))
        .border_1()
        .text_sm()
        .cursor_pointer()
        .child(label.into());
    if selected {
        base.bg(theme.accent)
            .border_color(theme.accent)
            .text_color(theme.on_accent)
    } else {
        base.border_color(theme.border)
            .text_color(theme.text)
            .hover(move |s| s.bg(hover))
    }
}

/// A switch drawn as a pill with a knob.
pub fn switch(id: impl Into<ElementId>, on: bool, theme: &Theme) -> Stateful<Div> {
    let knob =
        div()
            .size(px(14.0))
            .rounded(px(7.0))
            .bg(if on { theme.on_accent } else { theme.muted });
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .w(px(34.0))
        .h(px(20.0))
        .px(px(3.0))
        .rounded(px(10.0))
        .cursor_pointer()
        .bg(if on { theme.accent } else { theme.raised })
        .when(on, Styled::justify_end)
        .child(knob)
}

/// A label next to a control, the row of a settings list.
pub fn setting_row(
    label: impl Into<SharedString>,
    hint: Option<SharedString>,
    control: impl IntoElement,
    theme: &Theme,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .py(px(8.0))
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(div().text_sm().text_color(theme.text).child(label.into()))
                .children(hint.map(|h| div().text_xs().text_color(theme.muted).child(h))),
        )
        .child(control)
}

/// A section heading inside a panel.
pub fn heading(text: impl Into<SharedString>, theme: &Theme) -> Div {
    div()
        .pt(px(12.0))
        .pb(px(4.0))
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.muted)
        .child(text.into())
}

/// The view a tooltip shows.
struct Tooltip {
    text: SharedString,
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        div()
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(5.0))
            .bg(theme.raised)
            .border_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.text)
            .child(self.text.clone())
    }
}

/// A tooltip with one line of text.
pub fn tooltip(text: impl Into<SharedString>, cx: &mut App) -> AnyView {
    let text = text.into();
    cx.new(|_| Tooltip { text }).into()
}
