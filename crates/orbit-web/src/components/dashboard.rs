//! Main dashboard component

use leptos::*;

use super::{JobForm, JobList};

#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div class="container mx-auto p-4">
            <h1 class="text-3xl font-bold mb-6">"Orbit Web Dashboard"</h1>

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div class="lg:col-span-1">
                    <JobForm />
                </div>

                <div class="lg:col-span-2">
                    <JobList />
                </div>
            </div>
        </div>
    }
}
