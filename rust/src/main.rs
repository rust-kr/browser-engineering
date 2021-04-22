use druid::{AppLauncher, LocalizedString, WindowDesc};
use lib::display::BrowserWidget;
use lib::http::{lex, request};

const APP_NAME: &str = "Browser-engineering";
struct BrowserApplication {}

impl BrowserApplication {
    fn run(&self, url: &str) {
        let (_headers, body) = request(&url);
        let text = lex(&body);
        let browser_widget = || -> BrowserWidget { BrowserWidget::new(text) };
        let window = WindowDesc::new(browser_widget)
            .title(LocalizedString::new(APP_NAME))
            .window_size((BrowserWidget::get_width(), BrowserWidget::get_height()));
        AppLauncher::with_window(window)
            .use_simple_logger()
            .launch(0)
            .expect("launch failed");
    }
}

pub fn main() {
    use clap::{App, Arg};
    let matches = App::new(APP_NAME)
        .arg(Arg::with_name("url").value_name("URL").takes_value(true))
        .get_matches();
    let url = matches
        .value_of("url")
        .expect("required argument at the moment");

    let app = BrowserApplication {};
    app.run(url);
}
