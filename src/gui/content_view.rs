use agx_definitions::{
    Color, Drawable, LikeLayerSlice, NestedLayerSlice, PixelByteLayout, Point, Rect, RectInsets,
    Size,
};
use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    vec::Vec,
};
use libgui::{
    bordered::Bordered, text_view::TextView, ui_elements::UIElement, view::View, KeyCode,
};
use libgui_derive::{Bordered, Drawable, NestedLayerSlice, UIElement};
use ttf_renderer::Font;

#[derive(Drawable, NestedLayerSlice, UIElement, Bordered)]
pub struct ContentView {
    pub view: Rc<TextView>,
}

impl ContentView {
    pub fn new<F: Fn(&View, Size) -> Rect + 'static>(
        font: Font,
        font_size: Size,
        sizer: F,
    ) -> Rc<Self> {
        let view = TextView::new_with_font(
            Color::white(),
            font.clone(),
            font_size,
            RectInsets::new(2, 2, 2, 2),
            sizer,
            // PT: My emulated UEFI environment uses BGRA
            PixelByteLayout::BGRA,
        );

        Rc::new(Self {
            view: Rc::clone(&view),
        })
    }

    pub fn add_component(self: Rc<Self>, elem: Rc<dyn UIElement>) {
        Rc::clone(&self.view).add_component(elem)
    }
}
