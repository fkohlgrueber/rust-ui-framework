use stm32f7::lcd::{Layer, Framebuffer, Color, FramebufferArgb8888, FramebufferAl88};
use graphics::{line, point::Point, TouchEvent, gui::Message};
use graphics::ui_component::UIComponent;

use core::any::Any;
use alloc::Vec;

pub struct Polygon {
    points: Vec<Point>,
    color: Color,
    filled: bool,
}

impl Polygon {
    pub fn new(points: Vec<Point>, color: Color, filled: bool) -> Polygon{
        Polygon{
            points,
            color,
            filled,
        }
    }
}

impl UIComponent for Polygon {
    fn as_any(&self) -> &Any{
        self
    }

    fn clear(&self, lcd_ui: &mut Layer<FramebufferArgb8888>, _lcd_text: &mut Layer<FramebufferAl88>){
        draw_polygon(lcd_ui, &self.points, Color::rgba(0,0,0,0), self.filled);
    }

    fn draw(&self, old_widget: Option<&UIComponent>, lcd_ui: &mut Layer<FramebufferArgb8888>, lcd_text: &mut Layer<FramebufferAl88>){

        let old_poly = match old_widget {
            Some(ow) => ow.as_any().downcast_ref::<Polygon>(),
            None => None,
        };

        match old_poly {
            Some(o_w) => {
                if o_w.points != self.points || o_w.color != self.color || o_w.filled != self.filled {
                    o_w.clear(lcd_ui, lcd_text);
                    self.paint(lcd_ui, lcd_text);
                }
            },
            None => {
                if old_widget.is_some(){
                    old_widget.unwrap().clear(lcd_ui, lcd_text);
                }

                self.paint(lcd_ui, lcd_text);
            }
        }
    }

    fn is_in_bounding_box(&self, _p: &Point) -> bool{
        false
    }

    fn on_touch(&mut self, _evt: &TouchEvent) -> Option<Message>{
        None
    }

    fn paint(&self, lcd_ui: &mut Layer<FramebufferArgb8888>, _lcd_text: &mut Layer<FramebufferAl88>){
        draw_polygon(lcd_ui, &self.points, self.color, self.filled);
    }
}

pub fn draw_polygon<T: Framebuffer> (lcd: &mut Layer<T>, points: &[Point], color: Color, fill: bool) {
    if !(points.len() > 2) {
        return;
    }

    if fill {
        fill_polygon(lcd, points, color);
    } else {
        let mut last_point = &points[points.len()-1];
        for point in points {
            line::draw_line(lcd, last_point, point, color);
            last_point = point;
        }
    }
}

fn get_bounds(points: &[Point]) -> (Point, Point) {
    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = min_x;
    let mut max_y = min_y;

    for p in points {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    (
        // screen size: 480x272
        Point {x: min_x.max(0), y: min_y.max(0),},
        Point {x: max_x.min(480-1), y: max_y.min(272-1),},
    )
}

/*
 * Polygon fill algorithm by Darel Rex Finley (originally in C)
 * URL: http://alienryderflex.com/polygon_fill/
 * visited: 12:59:37
 */
fn fill_polygon<T: Framebuffer> (lcd: &mut Layer<T>, points: &[Point], color: Color) {
    let mut node_x = [0 as i32, points.len() as i32];
    let bounds = get_bounds(points);
    let poly_size = points.len();

    // loop through the rows of the image
    for pixel_y in bounds.0.y..bounds.1.y {
        // build a list of nodes
        let mut nodes = 0;
        let mut j = poly_size - 1;
        for i in 0..poly_size {
            let bool_a = points[i].y < pixel_y && points[j].y >= pixel_y;
            let bool_b = points[j].y < pixel_y && points[i].y >= pixel_y;
            if bool_a || bool_b {
                let a = points[i].x as i32;
                let b = pixel_y as i32 - points[i].y as i32;
                let c = points[j].x as i32 - points[i].x as i32;
                let d = points[j].y as i32 - points[i].y as i32;
                node_x[nodes] = a + b * c / d;
                nodes += 1;
            }
            j = i;
        }

        // sort the nodes with bubble sort
        let mut i = 0;
        while i + 1 < nodes {
            if node_x[i] > node_x[i + 1] {
                node_x.swap(i, i + 1);
                if i != 0 {
                    i -= 1;
                }
            } else {
                i += 1;
            }
        }

        // fill the pixels between node pairs
        for i in (0..nodes).filter(|e| e % 2 == 0) {
            if node_x[i] >= bounds.1.x as i32 {
                break;
            }
            if node_x[i + 1] > bounds.0.x as i32 {
                if node_x[i] < bounds.0.x as i32 {
                    node_x[i] = bounds.0.x as i32;
                }
                if node_x[i + 1] > bounds.1.x as i32 {
                    node_x[i + 1] = bounds.1.x as i32;
                }
                for pixel_x in node_x[i]..node_x[i + 1] {
                    lcd.print_point_color_at(pixel_x as usize, pixel_y, color);
                }
            }
        }
    }
}
