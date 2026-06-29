use dioxus::desktop::{Config, WindowBuilder};
use medquiz::config::APP_NAME;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(WindowBuilder::new().with_title(APP_NAME))
                .with_background_color((10, 12, 16, 255))
                .with_custom_head(
                    r#"
<meta name="color-scheme" content="dark">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
  html, body, #main {
    margin: 0;
    padding: 0;
    height: 100%;
    background: #0a0c10 !important;
    color: #eef1f6;
    color-scheme: dark;
  }
</style>
"#
                    .to_string(),
                ),
        )
        .launch(medquiz::ui::App);
}