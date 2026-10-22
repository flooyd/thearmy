//! the look of the interface: two fonts (Audiowide for headings, Chakra Petch for everything else), the accent colour, and the
//! helpers that draw text and panels with them

use crate::prelude::*;
use std::cell::RefCell;

//the fonts are inside the program, so it is one file and needs nothing beside it
static AUDIOWIDE: &[u8] = include_bytes!("../assets/fonts/Audiowide-Regular.ttf");
static CHAKRA_PETCH: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-Medium.ttf");
static CHAKRA_PETCH_BOLD: &[u8] = include_bytes!("../assets/fonts/ChakraPetch-SemiBold.ttf");

pub(crate) const PANEL: Color = Color::new(0.03, 0.05, 0.09, 0.84); //what panels are filled with: a dark blue, mostly see-through
pub(crate) const INK: Color = Color::new(0.93, 0.97, 1.0, 1.0); //text: white with a cold tint
pub(crate) const MUTED: Color = Color::new(0.58, 0.69, 0.8, 1.0); //labels and anything that shouldn't shout
pub(crate) const TROPHY: Color = Color::new(1.0, 0.85, 0.3, 1.0); //victories and rewards
pub(crate) const ALERT: Color = Color::new(1.0, 0.42, 0.35, 1.0); //danger and anything that can't be afforded

//which font a piece of text is drawn in
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Face {
    Header, //Audiowide: the title, banners, boss and relic names, big numbers
    Body,   //Chakra Petch
    Bold,   //Chakra Petch semibold, for values and small caps
}

struct Fonts {
    header: Font,
    body: Font,
    bold: Font,
}

thread_local! {
    //macroquad draws from one thread, and a game with no window (the tests) has no fonts and uses the built in one
    static FONTS: RefCell<Option<Fonts>> = const { RefCell::new(None) };
}

//load the fonts, once the window is open. a font that fails to load is left out and the built in one is used instead
pub(crate) fn load_fonts() {
    let header = load_ttf_font_from_bytes(AUDIOWIDE);
    let body = load_ttf_font_from_bytes(CHAKRA_PETCH);
    let bold = load_ttf_font_from_bytes(CHAKRA_PETCH_BOLD);
    if let (Ok(header), Ok(body), Ok(bold)) = (header, body, bold) {
        FONTS.with(|fonts| *fonts.borrow_mut() = Some(Fonts { header, body, bold }));
    }
}

fn with_font<R>(face: Face, then: impl FnOnce(Option<&Font>) -> R) -> R {
    FONTS.with(|fonts| {
        let fonts = fonts.borrow();
        then(fonts.as_ref().map(|fonts| match face {
            Face::Header => &fonts.header,
            Face::Body => &fonts.body,
            Face::Bold => &fonts.bold,
        }))
    })
}

//how wide the text is when drawn at a size, in pixels
pub(crate) fn text_width(text: &str, size: f32, face: Face) -> f32 {
    with_font(face, |font| measure_text(text, font, size.round().max(1.0) as u16, 1.0).width)
}

//text with its baseline at y, starting at x
pub(crate) fn draw_label(text: &str, x: f32, y: f32, size: f32, color: Color, face: Face) {
    with_font(face, |font| {
        draw_text_ex(text, x, y, TextParams { font, font_size: size.round().max(1.0) as u16, color, ..Default::default() });
    });
}

//text drawn in the world: rasterized at `size` and scaled by `scale`, so it is sharp at the size it ends up on screen
pub(crate) fn draw_label_scaled(text: &str, x: f32, y: f32, size: f32, scale: f32, color: Color, face: Face) {
    with_font(face, |font| {
        draw_text_ex(text, x, y, TextParams { font, font_size: size.round().max(1.0) as u16, font_scale: scale, color, ..Default::default() });
    });
}

//text that ends at x
pub(crate) fn draw_label_right(text: &str, x: f32, y: f32, size: f32, color: Color, face: Face) {
    draw_label(text, x - text_width(text, size, face), y, size, color, face);
}

//text centred on x
pub(crate) fn draw_label_centered(text: &str, x: f32, y: f32, size: f32, color: Color, face: Face) {
    draw_label(text, x - text_width(text, size, face) / 2.0, y, size, color, face);
}

//text with a dark shadow behind it, so it can be read over anything
pub(crate) fn draw_label_shadowed(text: &str, x: f32, y: f32, size: f32, color: Color, face: Face) {
    let offset = (size / 16.0).max(1.0).round();
    draw_label(text, x + offset, y + offset, size, Color::new(0.0, 0.0, 0.0, color.a * 0.75), face);
    draw_label(text, x, y, size, color, face);
}

//a heading centred on x with its shadow
pub(crate) fn draw_heading(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_label_shadowed(text, x - text_width(text, size, Face::Header) / 2.0, y, size, color, Face::Header);
}

//the largest size, down from `size`, at which the text fits in `width`
pub(crate) fn fit_size(text: &str, size: f32, width: f32, face: Face) -> f32 {
    let mut size = size;
    while size > 10.0 && text_width(text, size, face) > width {
        size -= 2.0;
    }
    size
}

//a dark panel with a thin edge in the accent colour and a brighter mark in two of its corners
pub(crate) fn panel(x: f32, y: f32, w: f32, h: f32) {
    panel_edged(x, y, w, h, CYAN);
}

pub(crate) fn panel_edged(x: f32, y: f32, w: f32, h: f32, edge: Color) {
    draw_rectangle(x, y, w, h, PANEL);
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(edge.r, edge.g, edge.b, 0.45));
    let tick = (w.min(h) * 0.25).clamp(6.0, 14.0);
    let bright = Color::new(edge.r, edge.g, edge.b, 0.95);
    draw_line(x - 1.0, y, x + tick, y, 3.0, bright);
    draw_line(x, y, x, y + tick, 3.0, bright);
    draw_line(x + w - tick, y + h, x + w + 1.0, y + h, 3.0, bright);
    draw_line(x + w, y + h - tick, x + w, y + h, 3.0, bright);
}

//a thin bar filled to a share, for health and progress
pub(crate) fn draw_meter(x: f32, y: f32, w: f32, h: f32, share: f32, color: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.55));
    draw_rectangle(x, y, w * share.clamp(0.0, 1.0), h, color);
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.35));
}

//a small label with a key in a box, like [R]
pub(crate) fn draw_key(key: &str, x: f32, y: f32, size: f32) -> f32 {
    let width = text_width(key, size, Face::Bold) + 10.0;
    draw_rectangle(x, y - size, width, size + 6.0, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.16));
    draw_rectangle_lines(x, y - size, width, size + 6.0, 1.5, Color::new(CYAN.r, CYAN.g, CYAN.b, 0.8));
    draw_label(key, x + 5.0, y, size, CYAN, Face::Bold);
    width
}

//a number with thousands separated, 1234 as 1,234
pub(crate) fn grouped(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

//minutes and seconds, 125 as 2:05
pub(crate) fn clock(seconds: f32) -> String {
    let seconds = seconds.max(0.0) as usize;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
