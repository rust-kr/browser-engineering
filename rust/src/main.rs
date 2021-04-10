use druid::{AppLauncher, LocalizedString, WindowDesc};
use lib::display::BrowserWidget;
use lib::http::{lex, request};

struct BrowserApplication {}

impl BrowserApplication {
    fn run(&self, url: &str) {
        let (_headers, body) = request(&url);
        let text = lex(&body);
        let browser_widget = || -> BrowserWidget { BrowserWidget::new(text) };
        let window = WindowDesc::new(browser_widget)
            .title(LocalizedString::new("Browser-engineering"))
            .window_size((BrowserWidget::get_width(), BrowserWidget::get_height()));
        AppLauncher::with_window(window)
            .use_simple_logger()
            .launch(0)
            .expect("launch failed");
    }
}

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let app = BrowserApplication {};
    app.run(&args[1]);
}
