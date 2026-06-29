use dioxus::prelude::*;

use crate::config;
use crate::ui::common::NavView;

#[component]
pub fn Sidebar(
    current: NavView,
    session_active: bool,
    api_ready: bool,
    on_nav: EventHandler<NavView>,
) -> Element {
    rsx! {
        div { class: "sidebar",
            p { class: "logo-title", "⚕ {config::APP_NAME}" }
            p { class: "logo-sub", "USMLE & COMLEX Prep" }
            if session_active {
                p { class: "sidebar-status", "Studying…" }
            }
            div { class: "api-status",
                span {
                    class: if api_ready { "api-pill api-pill--ready" } else { "api-pill api-pill--warn" },
                    if api_ready { "AI ready" } else { "AI not configured" }
                }
            }
            for (view, label) in [
                (NavView::Home, "Home"),
                (NavView::Materials, "Materials"),
                (NavView::Bank, "Question Bank"),
                (NavView::Settings, "Settings"),
            ] {
                button {
                    class: if current == view && !session_active { "nav-btn active" } else { "nav-btn" },
                    disabled: session_active,
                    onclick: move |_| on_nav.call(view),
                    "{label}"
                }
            }
        }
    }
}