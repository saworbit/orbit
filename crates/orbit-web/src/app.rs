//! Root application component

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::Dashboard;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html lang="en" />
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Title text="Orbit Web - File Transfer Dashboard"/>

        // Tailwind CSS via CDN (for MVP - use a build process in production)
        <Link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css"/>

        <Router>
            <nav class="bg-gray-800 text-white p-4 mb-4">
                <div class="container mx-auto flex justify-between items-center">
                    <h1 class="text-xl font-bold">"Orbit Web"</h1>
                    <div class="flex gap-4">
                        <A href="/" class="hover:text-gray-300">"Dashboard"</A>
                        <A href="/about" class="hover:text-gray-300">"About"</A>
                    </div>
                </div>
            </nav>

            <main class="min-h-screen bg-gray-100">
                <Routes>
                    <Route path="/" view=Dashboard/>
                    <Route path="/about" view=About/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>

            <footer class="bg-gray-800 text-white p-4 mt-8">
                <div class="container mx-auto text-center">
                    <p class="text-sm">
                        "Orbit Web v0.1.0 - High-performance file transfer orchestration"
                    </p>
                </div>
            </footer>
        </Router>
    }
}

#[component]
fn About() -> impl IntoView {
    view! {
        <div class="container mx-auto p-4">
            <h1 class="text-3xl font-bold mb-4">"About Orbit Web"</h1>
            <div class="bg-white shadow-md rounded px-8 pt-6 pb-8">
                <p class="mb-4">
                    "Orbit Web is a modern web interface for the Orbit file transfer system."
                </p>
                <p class="mb-4">
                    "Built with Leptos and Axum, it provides real-time monitoring and control "
                    "of file transfers with features like:"
                </p>
                <ul class="list-disc list-inside mb-4 space-y-2">
                    <li>"Live progress tracking"</li>
                    <li>"Job creation and management"</li>
                    <li>"Crash recovery and resumption"</li>
                    <li>"Parallel transfer orchestration"</li>
                    <li>"Compression and verification"</li>
                </ul>
                <p class="text-sm text-gray-600">
                    "For more information, visit the "
                    <a href="https://github.com/saworbit/orbit" class="text-blue-500 hover:underline">
                        "GitHub repository"
                    </a>
                </p>
            </div>
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="container mx-auto p-4">
            <h1 class="text-3xl font-bold mb-4">"404 - Page Not Found"</h1>
            <p>
                <A href="/" class="text-blue-500 hover:underline">"Return to Dashboard"</A>
            </p>
        </div>
    }
}
