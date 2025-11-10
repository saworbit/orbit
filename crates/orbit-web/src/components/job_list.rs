//! Job list component with real-time updates

use leptos::*;

use crate::server_fns::list_jobs;
use super::ProgressBar;

#[component]
pub fn JobList() -> impl IntoView {
    let jobs = create_resource(
        || (),
        |_| async move { list_jobs().await.unwrap_or_default() }
    );

    // Auto-refresh every 2 seconds
    let (refresh_trigger, set_refresh_trigger) = create_signal(0);

    set_interval(
        move || {
            set_refresh_trigger.update(|n| *n += 1);
            jobs.refetch();
        },
        std::time::Duration::from_secs(2),
    );

    view! {
        <div class="bg-white shadow-md rounded px-8 pt-6 pb-8">
            <div class="flex justify-between items-center mb-4">
                <h2 class="text-xl font-bold">"Active Jobs"</h2>
                <button
                    class="bg-gray-500 hover:bg-gray-700 text-white font-bold py-1 px-3 rounded text-sm"
                    on:click=move |_| jobs.refetch()
                >
                    "Refresh"
                </button>
            </div>

            <Suspense fallback=move || view! { <p>"Loading jobs..."</p> }>
                {move || jobs.get().map(|jobs_data| {
                    if jobs_data.is_empty() {
                        view! {
                            <p class="text-gray-500 italic">"No active jobs"</p>
                        }.into_view()
                    } else {
                        view! {
                            <div class="overflow-x-auto">
                                <table class="min-w-full table-auto">
                                    <thead>
                                        <tr class="bg-gray-200">
                                            <th class="px-4 py-2 text-left">"ID"</th>
                                            <th class="px-4 py-2 text-left">"Source"</th>
                                            <th class="px-4 py-2 text-left">"Status"</th>
                                            <th class="px-4 py-2 text-left">"Progress"</th>
                                            <th class="px-4 py-2 text-left">"Chunks"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {jobs_data.into_iter().map(|job| {
                                            let job_id = job.id.clone();
                                            let status_class = match job.status.as_str() {
                                                "completed" => "text-green-600",
                                                "failed" => "text-red-600",
                                                "processing" => "text-blue-600",
                                                _ => "text-gray-600",
                                            };

                                            view! {
                                                <tr class="border-b hover:bg-gray-50">
                                                    <td class="px-4 py-2">{job.id}</td>
                                                    <td class="px-4 py-2 truncate max-w-xs">{job.source}</td>
                                                    <td class="px-4 py-2">
                                                        <span class=status_class>{job.status}</span>
                                                    </td>
                                                    <td class="px-4 py-2">
                                                        <ProgressBar percent=job.completion_percent />
                                                    </td>
                                                    <td class="px-4 py-2">
                                                        {format!("{}/{} ({} failed)", job.done, job.total_chunks, job.failed)}
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_view()
                    }
                })}
            </Suspense>
        </div>
    }
}
