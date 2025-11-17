//! Root application component

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use super::{Dashboard, Login};

/// Root application component
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/orbit-web.css"/>
        <Title text="Orbit Nebula - Control Center"/>
        <Meta name="description" content="Next-gen data orchestration control center"/>

        <Router>
            <main class="min-h-screen bg-gray-900 text-gray-100">
                <Routes>
                    <Route path="/" view=Dashboard/>
                    <Route path="/login" view=Login/>
                </Routes>
            </main>
        </Router>
    }
}
