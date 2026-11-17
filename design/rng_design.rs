//! RNG Design tokens: one file shared byte-for-byte by every RNG Studio app (CutCat, PhotoFalcon,
//! PDFrog, DocDuck, SlideSeal, RNG Studio). Source of truth: rng-app-builds/design/rng_design.rs,
//! spec: rng-app-builds/docs/RNG_DESIGN.md. Copy with scripts/sync-design.sh; don't edit per app.
#![allow(dead_code)]

use egui::{Color32, CornerRadius, Shadow, Stroke};

/// One appearance's colours (spec §2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub canvas: Color32,
    pub base: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub control: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub separator: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub accent: Color32,
    pub accent_fill: Color32,
    pub accent_text: Color32,
    pub accent_tint: Color32,
    pub value: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    /// Rim of floating surfaces (popovers, dialogs, HUDs).
    pub rim: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub const DARK: Palette = Palette {
    dark: true,
    canvas: hex(0x0e0d0d),
    base: hex(0x151414),
    panel: hex(0x1c1b1a),
    raised: hex(0x242322),
    control: hex(0x2c2a29),
    hover: hex(0x33312f),
    pressed: hex(0x3d3a38),
    separator: hex(0x2a2827),
    border: hex(0x3a3735),
    text: hex(0xf3eee8),
    text_secondary: hex(0xaaa49d),
    text_tertiary: hex(0x7d7871),
    accent: hex(0xff453a),
    accent_fill: hex(0xd92b20),
    accent_text: hex(0xff6961),
    // accent at 16 % over `panel`, premultiplied into an opaque colour so it never stacks
    accent_tint: hex(0x3e1f1c),
    value: hex(0xe8b85c),
    success: hex(0x32d74b),
    warning: hex(0xffb340),
    danger: hex(0xff6961),
    rim: Color32::from_rgba_premultiplied(20, 20, 20, 20),
};

pub const LIGHT: Palette = Palette {
    dark: false,
    canvas: hex(0xe8e5e1),
    base: hex(0xf3f1ee),
    panel: hex(0xfbfaf8),
    raised: hex(0xffffff),
    control: hex(0xefece8),
    hover: hex(0xe6e2dd),
    pressed: hex(0xdbd6d0),
    separator: hex(0xe3ded8),
    border: hex(0xcdc7c0),
    text: hex(0x1d1b19),
    text_secondary: hex(0x5f5a54),
    text_tertiary: hex(0x8a847c),
    accent: hex(0xd70015),
    accent_fill: hex(0xd70015),
    accent_text: hex(0xc4000f),
    // accent at 12 % over `panel`
    accent_tint: hex(0xf8dcdb),
    value: hex(0x946510),
    success: hex(0x1f8f3a),
    warning: hex(0xa05a00),
    danger: hex(0xc4000f),
    rim: Color32::from_rgba_premultiplied(0, 0, 0, 20),
};

pub fn palette(dark: bool) -> &'static Palette {
    if dark { &DARK } else { &LIGHT }
}

/// Type scale in points (spec §3).
pub mod size {
    pub const LARGE_TITLE: f32 = 26.0;
    pub const TITLE1: f32 = 20.0;
    pub const TITLE2: f32 = 15.0;
    pub const HEADLINE: f32 = 13.0;
    pub const BODY: f32 = 13.0;
    pub const BODY_DENSE: f32 = 12.0;
    pub const CALLOUT: f32 = 12.0;
    pub const CAPTION: f32 = 11.0;
    pub const MICRO: f32 = 10.0;
}

/// 8 pt grid (spec §4).
pub mod space {
    pub const XXS: f32 = 4.0;
    pub const XS: f32 = 8.0;
    pub const S: f32 = 12.0;
    pub const M: f32 = 16.0;
    pub const L: f32 = 24.0;
    pub const XL: f32 = 32.0;
    pub const XXL: f32 = 48.0;
}

/// Control and bar metrics (spec §4).
pub mod metric {
    pub const CONTROL_H: f32 = 28.0;
    pub const CONTROL_H_DENSE: f32 = 24.0;
    pub const CONTROL_H_LARGE: f32 = 32.0;
    pub const TOOLBAR_H: f32 = 40.0;
    pub const TOOLBAR_H_DENSE: f32 = 36.0;
    pub const TAB_H: f32 = 30.0;
    pub const STATUS_H: f32 = 22.0;
    pub const SIDEBAR_W: f32 = 232.0;
    pub const INSPECTOR_W: f32 = 296.0;
    pub const ROW_H: f32 = 28.0;
    pub const ROW_H_DENSE: f32 = 24.0;
    pub const ICON: f32 = 16.0;
    pub const ICON_DENSE: f32 = 14.0;
    pub const ICON_LARGE: f32 = 20.0;
}

/// Corner radii (spec §4).
pub mod radius {
    pub const SMALL: f32 = 4.0;
    pub const CONTROL: f32 = 6.0;
    pub const CARD: f32 = 10.0;
    pub const WINDOW: f32 = 14.0;
}

/// Motion in seconds (spec §9).
pub mod motion {
    pub const FAST: f32 = 0.12;
    pub const MEDIUM: f32 = 0.18;
    pub const SLOW: f32 = 0.24;
}

pub fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round() as u8)
}

/// Floating-layer shadows (spec §5).
pub fn popover_shadow(p: &Palette) -> Shadow {
    Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if p.dark { 102 } else { 40 }) }
}
pub fn dialog_shadow(p: &Palette) -> Shadow {
    Shadow { offset: [0, 16], blur: 48, spread: 0, color: Color32::from_black_alpha(if p.dark { 128 } else { 56 }) }
}

/// Popover fill: `raised` at 96 % (the "glass" without a blur egui can't do).
pub fn popover_fill(p: &Palette) -> Color32 {
    Color32::from_rgba_unmultiplied(p.raised.r(), p.raised.g(), p.raised.b(), 245)
}
pub fn rim_stroke(p: &Palette) -> Stroke {
    Stroke::new(1.0, if p.dark { Color32::from_white_alpha(20) } else { Color32::from_black_alpha(20) })
}

/// Keyboard focus ring: 2 px accent.
pub fn focus_stroke(p: &Palette) -> Stroke {
    Stroke::new(2.0, p.accent)
}

/// egui visuals for an app that has no richer theme of its own (spec §2, §5, §9). Apps with their
/// own token struct map these values in their theme.rs instead.
pub fn visuals(p: &Palette) -> egui::Visuals {
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.panel;
    v.window_fill = popover_fill(p);
    v.extreme_bg_color = p.base;
    v.faint_bg_color = p.raised;
    v.code_bg_color = p.control;
    v.override_text_color = Some(p.text);
    v.weak_text_color = Some(p.text_secondary);
    v.hyperlink_color = p.accent_text;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.danger;
    v.selection.bg_fill = p.accent_tint;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = rim_stroke(p);
    v.window_corner_radius = cr(radius::WINDOW);
    v.menu_corner_radius = cr(radius::CARD);
    v.window_shadow = dialog_shadow(p);
    v.popup_shadow = popover_shadow(p);
    v.striped = false;
    v.slider_trailing_fill = true;
    let w = &mut v.widgets;
    for s in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        s.corner_radius = cr(radius::CONTROL);
        s.expansion = 0.0;
    }
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text_secondary);
    w.inactive.bg_fill = p.control;
    w.inactive.weak_bg_fill = p.control;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.active.bg_fill = p.pressed;
    w.active.weak_bg_fill = p.pressed;
    w.active.bg_stroke = Stroke::NONE;
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.open.bg_fill = p.hover;
    w.open.weak_bg_fill = p.hover;
    w.open.bg_stroke = Stroke::NONE;
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    v
}

/// Spacing for the egui style (spec §4).
pub fn apply_spacing(s: &mut egui::Style) {
    s.spacing.item_spacing = egui::vec2(space::XS, space::XXS + 2.0);
    s.spacing.button_padding = egui::vec2(space::S, space::XXS + 1.0);
    s.spacing.interact_size.y = metric::CONTROL_H_DENSE;
    s.spacing.menu_margin = egui::Margin::same(6);
    s.spacing.window_margin = egui::Margin::same(space::M as i8);
    s.animation_time = motion::FAST;
}

/// RNG Glass (spec §5): which pane of glass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glass {
    Bar,
    Sidebar,
    Popover,
    Hud,
    Dialog,
}

impl Glass {
    fn alpha(self) -> u8 {
        match self {
            Glass::Bar => 219,
            Glass::Sidebar => 230,
            Glass::Popover => 240,
            Glass::Hud => 184,
            Glass::Dialog => 245,
        }
    }
    fn floating(self) -> bool {
        matches!(self, Glass::Popover | Glass::Hud | Glass::Dialog)
    }
}

/// The glass fill colour for `kind` (unmultiplied alpha over the backdrop).
pub fn glass_fill(p: &Palette, kind: Glass) -> Color32 {
    let c = if matches!(kind, Glass::Popover | Glass::Dialog) { p.raised } else { p.panel };
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), kind.alpha())
}

/// Paints one pane of glass: shadow (floating only), tinted fill, side rim, specular top highlight
/// and (dark) a darker bottom edge. Draw content on top afterwards.
pub fn paint_glass(painter: &egui::Painter, rect: egui::Rect, r: f32, kind: Glass, p: &Palette) {
    let radius = cr(r);
    if kind.floating() {
        let sh = if kind == Glass::Dialog { dialog_shadow(p) } else { popover_shadow(p) };
        painter.add(sh.as_shape(rect, radius));
    }
    painter.rect_filled(rect, radius, glass_fill(p, kind));
    let side = if p.dark { Color32::from_white_alpha(14) } else { Color32::from_black_alpha(18) };
    painter.rect_stroke(rect, radius, Stroke::new(1.0, side), egui::StrokeKind::Inside);
    // specular highlight along the top edge, inset by the corner radius so it follows the curve
    let hi = if p.dark { Color32::from_white_alpha(40) } else { Color32::from_white_alpha(180) };
    let inset = r.max(2.0);
    let y = rect.top() + 0.5;
    let x0 = rect.left() + inset;
    let x1 = rect.right() - inset;
    if x1 > x0 {
        let mid = (x0 + x1) * 0.5;
        let mut mesh = egui::Mesh::default();
        let fade = Color32::TRANSPARENT;
        // two quads: transparent → highlight → transparent, 1 px tall
        for (a, b, ca, cb) in [(x0, mid, fade, hi), (mid, x1, hi, fade)] {
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(egui::pos2(a, y - 0.5), ca);
            mesh.colored_vertex(egui::pos2(b, y - 0.5), cb);
            mesh.colored_vertex(egui::pos2(b, y + 0.5), cb);
            mesh.colored_vertex(egui::pos2(a, y + 0.5), ca);
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    if p.dark && kind.floating() {
        let y = rect.bottom() - 0.5;
        painter.line_segment(
            [egui::pos2(rect.left() + inset, y), egui::pos2(rect.right() - inset, y)],
            Stroke::new(1.0, Color32::from_black_alpha(64)),
        );
    }
}

/// The window backdrop the glass sits on: `canvas` with a soft warm glow toward the top centre.
pub fn paint_backdrop(painter: &egui::Painter, rect: egui::Rect, p: &Palette) {
    painter.rect_filled(rect, CornerRadius::ZERO, p.canvas);
    let glow = if p.dark { Color32::from_rgba_unmultiplied(0x6b, 0x55, 0x48, 26) } else { Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, 120) };
    let c = egui::pos2(rect.center().x, rect.top() - rect.height() * 0.15);
    let rad = rect.width().max(rect.height()) * 0.75;
    // radial glow as a fan of triangles: centre opaque, rim transparent
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, glow);
    let n = 48u32;
    for k in 0..=n {
        let a = std::f32::consts::TAU * k as f32 / n as f32;
        mesh.colored_vertex(c + egui::vec2(a.cos(), a.sin()) * rad, Color32::TRANSPARENT);
    }
    for k in 1..=n {
        mesh.add_triangle(0, k, k + 1);
    }
    painter.with_clip_rect(rect).add(egui::Shape::mesh(mesh));
}

/// Rounds the corners of an already-painted square pane by painting `backdrop` outside a radius-`r`
/// arc in each corner (for docked panels drawn by code that only paints rectangles).
pub fn paint_corner_mask(painter: &egui::Painter, rect: egui::Rect, r: f32, backdrop: Color32) {
    let corners = [
        (rect.left_top(), egui::vec2(1.0, 1.0)),
        (rect.right_top(), egui::vec2(-1.0, 1.0)),
        (rect.right_bottom(), egui::vec2(-1.0, -1.0)),
        (rect.left_bottom(), egui::vec2(1.0, -1.0)),
    ];
    for (corner, dir) in corners {
        let centre = corner + dir * r;
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(corner, backdrop);
        let n = 8u32;
        for k in 0..=n {
            let a = std::f32::consts::FRAC_PI_2 * k as f32 / n as f32;
            // from the edge point beside the corner along x, round to the edge point along y
            let v = egui::vec2(-dir.x * a.cos(), -dir.y * a.sin());
            mesh.colored_vertex(centre + v * r, backdrop);
        }
        for k in 1..=n {
            mesh.add_triangle(0, k, k + 1);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// Specular rim for a docked pane: 1 px highlight across the top edge plus a faint side rim.
pub fn paint_pane_rim(painter: &egui::Painter, rect: egui::Rect, r: f32, p: &Palette) {
    let side = if p.dark { Color32::from_white_alpha(12) } else { Color32::from_black_alpha(16) };
    painter.rect_stroke(rect, cr(r), Stroke::new(1.0, side), egui::StrokeKind::Inside);
    let hi = if p.dark { Color32::from_white_alpha(34) } else { Color32::from_white_alpha(200) };
    let y = rect.top() + 0.5;
    let (x0, x1) = (rect.left() + r, rect.right() - r);
    if x1 > x0 {
        let mid = (x0 + x1) * 0.5;
        let mut mesh = egui::Mesh::default();
        for (a, b, ca, cb) in [(x0, mid, Color32::TRANSPARENT, hi), (mid, x1, hi, Color32::TRANSPARENT)] {
            let i = mesh.vertices.len() as u32;
            mesh.colored_vertex(egui::pos2(a, y - 0.5), ca);
            mesh.colored_vertex(egui::pos2(b, y - 0.5), cb);
            mesh.colored_vertex(egui::pos2(b, y + 0.5), cb);
            mesh.colored_vertex(egui::pos2(a, y + 0.5), ca);
            mesh.add_triangle(i, i + 1, i + 2);
            mesh.add_triangle(i, i + 2, i + 3);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}
