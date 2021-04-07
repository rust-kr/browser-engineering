use druid::piet::{FontFamily, Text, TextLayoutBuilder};
use druid::widget::prelude::*;
use druid::{AppLauncher, Color, LocalizedString, WindowDesc};
use http::{lex, request};
use std::cmp;

static WIDTH: i32 = 800;
static HEIGHT: i32 = 600;
static HSTEP: i32 = 13;
static VSTEP: i32 = 12;
static SCROLL_STEP: i32 = 100;

struct Character {
    x: i32,
    y: i32,
    ch: char,
}

struct BrowserWidget {
    display_list: Vec<Character>,
    scroll: i32,
    min_scroll: i32,
    max_scroll: i32,
}

trait Browser {
    fn load(&mut self, url: &str);
}

impl Browser for BrowserWidget {
    fn load(&mut self, url: &str) {
        let (_headers, body) = request(url);
        let text = lex(&body);
        let mut cursor_x = HSTEP;
        let mut cursor_y = VSTEP;
        for c in text.chars() {
            self.max_scroll = cmp::max(self.max_scroll, cursor_y);
            self.display_list.push(Character {
                x: cursor_x,
                y: cursor_y,
                ch: c,
            });
            cursor_x += VSTEP;
            if cursor_x >= WIDTH - HSTEP || c == '\n' {
                cursor_y += VSTEP;
                cursor_x = HSTEP;
            }
        }
    }
}

impl Widget<i32> for BrowserWidget {
    fn event(&mut self, ctx: &mut EventCtx, _event: &Event, _data: &mut i32, _env: &Env) {
        match _event {
            Event::Wheel(e) => {
                if e.wheel_delta.y < 0.0 {
                    self.scroll -= SCROLL_STEP;
                    self.scroll = cmp::max(self.scroll, self.min_scroll);
                } else if e.wheel_delta.y > 0.0 {
                    self.scroll += SCROLL_STEP;
                    self.scroll = cmp::min(self.scroll, self.max_scroll);
                }
                *_data = self.scroll;
                ctx.request_update();
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &i32, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &i32, data: &i32, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &i32,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &i32, _env: &Env) {
        let size = ctx.size();
        let rect = size.to_rect();
        ctx.fill(rect, &Color::WHITE);
        for ch in &self.display_list {
            if ch.y > self.scroll + HEIGHT {
                continue;
            }

            if ch.y + VSTEP < self.scroll {
                continue;
            }

            let text = ctx.text();
            let layout = text
                .new_text_layout(String::from(ch.ch))
                .font(FontFamily::default(), 12.0)
                .text_color(Color::BLACK)
                .build()
                .unwrap();
            ctx.draw_text(&layout, (ch.x as f64, ch.y as f64 - self.scroll as f64));
        }
    }
}

pub fn main() {
    let browser = || -> BrowserWidget {
        let args: Vec<String> = std::env::args().collect();
        let mut ret = BrowserWidget {
            display_list: Vec::new(),
            scroll: 0,
            min_scroll: 0,
            max_scroll: 0,
        };
        ret.load(&args[1]);
        ret
    };
    let window = WindowDesc::new(browser)
        .title(LocalizedString::new("Browser-engineering"))
        .window_size((WIDTH as f64, HEIGHT as f64));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}
