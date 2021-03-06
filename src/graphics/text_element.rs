use stm32f7::lcd::{FramebufferAl88, FramebufferArgb8888};
use stm32f7::lcd::Layer;
use graphics::ui_component::UIComponent;
use graphics::point::Point;
use graphics::{gui::Message, TouchEvent};

use alloc::String;
use core::any::Any;

pub struct TextElement {
    text: String,
    x_pos: usize,
    y_pos: usize,
}

impl TextElement{
    pub fn new(x_pos: usize, y_pos: usize, text: String) -> TextElement {
        TextElement{ text, x_pos, y_pos,}
    }
}

impl UIComponent for TextElement {
    fn as_any(&self) -> &Any {
        self
    }

    fn clear(&self, _lcd_ui: &mut Layer<FramebufferArgb8888>, lcd_text: &mut Layer<FramebufferAl88>) {
        lcd_text
            .text_writer()
            .clear_str_at(self.x_pos, self.y_pos, &self.text);
    }

    fn draw(&self, old_widget: Option<&UIComponent>, _lcd_ui: &mut Layer<FramebufferArgb8888>, lcd_text: &mut Layer<FramebufferAl88>){
        let old_text = match old_widget {
            Some(ow) => ow.as_any().downcast_ref::<TextElement>(),
            None => None,
        };
        
        match old_text {
            Some(o_t) => //if position or text changes, clear old and write new
            if o_t.x_pos != self.x_pos || o_t.y_pos != self.y_pos || o_t.text != self.text {
                o_t.clear(_lcd_ui, lcd_text);
                self.paint(_lcd_ui, lcd_text);
            },
            None => {
                if old_widget.is_some(){
                    old_widget.unwrap().clear(_lcd_ui, lcd_text);
                }

                self.paint(_lcd_ui, lcd_text);
            },
        }
    }

    fn is_in_bounding_box(&self, _p: &Point) -> bool{ false }

    fn on_touch(&mut self, _evt: &TouchEvent) -> Option<Message> {None}

    fn paint(&self, _lcd_ui: &mut Layer<FramebufferArgb8888>, lcd_text: &mut Layer<FramebufferAl88>){
        lcd_text
            .text_writer()
            .print_str_at(self.x_pos, self.y_pos, &self.text);
    }
}