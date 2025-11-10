//! Progress bar component

use leptos::*;

#[component]
pub fn ProgressBar(
    #[prop(into)] percent: f64,
) -> impl IntoView {
    let width_style = move || format!("width: {}%", percent.min(100.0).max(0.0));

    let color_class = move || {
        if percent >= 100.0 {
            "bg-green-500"
        } else if percent > 0.0 {
            "bg-blue-500"
        } else {
            "bg-gray-300"
        }
    };

    view! {
        <div class="w-full bg-gray-200 rounded-full h-4 overflow-hidden">
            <div
                class=move || format!("h-full transition-all duration-300 {}", color_class())
                style=width_style
            >
                <span class="text-xs text-white px-2 leading-4">
                    {move || format!("{:.1}%", percent)}
                </span>
            </div>
        </div>
    }
}
