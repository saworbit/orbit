//! Login page component

use leptos::*;
use leptos_router::*;

/// Login page component
#[component]
pub fn Login() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);

    let navigate = use_navigate();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_loading.set(true);
        set_error.set(None);

        let username_val = username.get();
        let password_val = password.get();

        spawn_local(async move {
            // Call login API
            let response = reqwest::Client::new()
                .post("/api/auth/login")
                .json(&serde_json::json!({
                    "username": username_val,
                    "password": password_val
                }))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    // Login successful, redirect to dashboard
                    navigate("/", Default::default());
                }
                Ok(resp) => {
                    // Login failed
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Login failed".to_string());
                    set_error.set(Some(error_msg));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Network error: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-900 px-4">
            <div class="max-w-md w-full space-y-8">
                <div class="text-center">
                    <h1 class="text-4xl font-bold text-blue-400">"🚀 Orbit Nebula"</h1>
                    <p class="mt-2 text-gray-400">"Next-Gen Data Orchestration"</p>
                </div>

                <form class="mt-8 space-y-6 bg-gray-800 p-8 rounded-lg shadow-xl" on:submit=on_submit>
                    <div class="space-y-4">
                        <div>
                            <label for="username" class="block text-sm font-medium text-gray-300">
                                "Username"
                            </label>
                            <input
                                id="username"
                                name="username"
                                type="text"
                                required
                                class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                placeholder="admin"
                                on:input=move |ev| set_username.set(event_target_value(&ev))
                                prop:value=username
                            />
                        </div>

                        <div>
                            <label for="password" class="block text-sm font-medium text-gray-300">
                                "Password"
                            </label>
                            <input
                                id="password"
                                name="password"
                                type="password"
                                required
                                class="mt-1 block w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                placeholder="••••••••"
                                on:input=move |ev| set_password.set(event_target_value(&ev))
                                prop:value=password
                            />
                        </div>
                    </div>

                    {move || error.get().map(|err| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-200 px-4 py-3 rounded">
                            {err}
                        </div>
                    })}

                    <button
                        type="submit"
                        disabled=loading
                        class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if loading.get() { "Logging in..." } else { "Sign In" }}
                    </button>

                    <p class="mt-4 text-center text-sm text-gray-400">
                        "Default: admin / orbit2025"
                    </p>
                </form>
            </div>
        </div>
    }
}
