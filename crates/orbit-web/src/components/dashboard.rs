//! Dashboard component with live job list

use crate::api::{list_jobs, JobInfo};
use leptos::*;
use leptos_router::*;

/// Dashboard component
#[component]
pub fn Dashboard() -> impl IntoView {
    // Fetch jobs list
    let jobs = create_resource(
        || (),
        |_| async move {
            list_jobs().await.unwrap_or_else(|e| {
                logging::error!("Failed to load jobs: {}", e);
                vec![]
            })
        },
    );

    // Auto-refresh every 2 seconds
    create_effect(move |_| {
        set_interval(
            move || {
                jobs.refetch();
            },
            std::time::Duration::from_secs(2),
        );
    });

    view! {
        <div class="container mx-auto px-4 py-8">
            <div class="flex justify-between items-center mb-8">
                <h1 class="text-3xl font-bold text-blue-400">"🚀 Orbit Nebula Control Center"</h1>
                <div class="flex items-center space-x-4">
                    <span class="text-sm text-gray-400">"Real-time Dashboard"</span>
                    <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                </div>
            </div>

            <div class="bg-gray-800 rounded-lg shadow-xl p-6">
                <h2 class="text-xl font-semibold mb-4 text-gray-200">"Active Jobs"</h2>

                <Suspense fallback=move || view! { <p class="text-gray-400">"Loading jobs..."</p> }>
                    {move || jobs.get().map(|jobs_list| {
                        if jobs_list.is_empty() {
                            view! {
                                <p class="text-gray-400 text-center py-8">
                                    "No jobs yet. Create your first transfer job!"
                                </p>
                            }.into_view()
                        } else {
                            view! {
                                <div class="space-y-4">
                                    <For
                                        each=move || jobs_list.clone()
                                        key=|job| job.id
                                        children=move |job: JobInfo| {
                                            view! {
                                                <div class="bg-gray-700 rounded-lg p-4 hover:bg-gray-650 transition-colors">
                                                    <div class="flex justify-between items-start">
                                                        <div class="flex-1">
                                                            <div class="flex items-center space-x-2">
                                                                <span class="text-sm font-mono text-gray-400">
                                                                    "Job #" {job.id}
                                                                </span>
                                                                <span class={format!(
                                                                    "px-2 py-1 text-xs rounded-full {}",
                                                                    match job.status.as_str() {
                                                                        "running" => "bg-blue-900 text-blue-200",
                                                                        "completed" => "bg-green-900 text-green-200",
                                                                        "failed" => "bg-red-900 text-red-200",
                                                                        _ => "bg-gray-600 text-gray-300",
                                                                    }
                                                                )}>
                                                                    {job.status.clone()}
                                                                </span>
                                                            </div>
                                                            <p class="mt-2 text-sm text-gray-300">
                                                                <span class="text-gray-500">"Source: "</span>
                                                                {job.source.clone()}
                                                            </p>
                                                            <p class="text-sm text-gray-300">
                                                                <span class="text-gray-500">"Dest: "</span>
                                                                {job.destination.clone()}
                                                            </p>
                                                            <div class="mt-3">
                                                                <div class="flex justify-between text-xs text-gray-400 mb-1">
                                                                    <span>"Progress"</span>
                                                                    <span>{format!("{:.1}%", job.progress * 100.0)}</span>
                                                                </div>
                                                                <div class="w-full bg-gray-600 rounded-full h-2">
                                                                    <div
                                                                        class="bg-blue-500 h-2 rounded-full transition-all duration-300"
                                                                        style:width=format!("{}%", job.progress * 100.0)
                                                                    ></div>
                                                                </div>
                                                                <p class="mt-1 text-xs text-gray-400">
                                                                    {format!(
                                                                        "{} / {} chunks completed",
                                                                        job.completed_chunks,
                                                                        job.total_chunks
                                                                    )}
                                                                </p>
                                                            </div>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            }.into_view()
                        }
                    })}
                </Suspense>
            </div>

            <div class="mt-6 text-center">
                <p class="text-sm text-gray-500">
                    "Auto-refreshing every 2 seconds • WebSocket support coming soon"
                </p>
            </div>
        </div>
    }
}
