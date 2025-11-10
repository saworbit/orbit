//! Job creation form component

use leptos::*;

use crate::server_fns::create_job;
use crate::types::CreateJobRequest;

#[component]
pub fn JobForm() -> impl IntoView {
    let (source, set_source) = create_signal(String::new());
    let (destination, set_destination) = create_signal(String::new());
    let (compress, set_compress) = create_signal(false);
    let (verify, set_verify) = create_signal(true);
    let (parallel, set_parallel) = create_signal(4);

    let create_job_action = create_action(|request: &CreateJobRequest| {
        let request = request.clone();
        async move {
            create_job(request).await
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let request = CreateJobRequest {
            source: source.get(),
            destination: destination.get(),
            compress: compress.get(),
            verify: verify.get(),
            parallel: Some(parallel.get()),
        };

        create_job_action.dispatch(request);
    };

    view! {
        <div class="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
            <h2 class="text-xl font-bold mb-4">"Create New Job"</h2>

            <form on:submit=on_submit>
                <div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="source">
                        "Source Path"
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                        id="source"
                        type="text"
                        placeholder="/path/to/source"
                        on:input=move |ev| set_source.set(event_target_value(&ev))
                        prop:value=source
                    />
                </div>

                <div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="destination">
                        "Destination Path"
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                        id="destination"
                        type="text"
                        placeholder="/path/to/destination"
                        on:input=move |ev| set_destination.set(event_target_value(&ev))
                        prop:value=destination
                    />
                </div>

                <div class="mb-4">
                    <label class="flex items-center">
                        <input
                            type="checkbox"
                            class="mr-2"
                            on:change=move |ev| set_compress.set(event_target_checked(&ev))
                            prop:checked=compress
                        />
                        <span class="text-sm">"Enable Compression"</span>
                    </label>
                </div>

                <div class="mb-4">
                    <label class="flex items-center">
                        <input
                            type="checkbox"
                            class="mr-2"
                            on:change=move |ev| set_verify.set(event_target_checked(&ev))
                            prop:checked=verify
                        />
                        <span class="text-sm">"Verify Checksums"</span>
                    </label>
                </div>

                <div class="mb-6">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="parallel">
                        "Parallel Workers: " {move || parallel.get()}
                    </label>
                    <input
                        class="w-full"
                        id="parallel"
                        type="range"
                        min="1"
                        max="16"
                        on:input=move |ev| set_parallel.set(event_target_value(&ev).parse().unwrap_or(4))
                        prop:value=parallel
                    />
                </div>

                <button
                    class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline w-full"
                    type="submit"
                    disabled=move || create_job_action.pending().get()
                >
                    {move || if create_job_action.pending().get() {
                        "Creating..."
                    } else {
                        "Create Job"
                    }}
                </button>

                {move || {
                    create_job_action.value().get().map(|result| match result {
                        Ok(job_id) => view! {
                            <div class="mt-4 p-2 bg-green-100 text-green-700 rounded">
                                "Job created: " {job_id}
                            </div>
                        }.into_view(),
                        Err(e) => view! {
                            <div class="mt-4 p-2 bg-red-100 text-red-700 rounded">
                                "Error: " {e.to_string()}
                            </div>
                        }.into_view(),
                    })
                }}
            </form>
        </div>
    }
}
