//! Login page component

use leptos::*;

/// Login page component (simplified for MVP - API-focused)
#[component]
pub fn Login() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-900 px-4">
            <div class="max-w-md w-full space-y-8">
                <div class="text-center">
                    <h1 class="text-4xl font-bold text-blue-400">"🚀 Orbit Nebula"</h1>
                    <p class="mt-2 text-gray-400">"Next-Gen Data Orchestration - API Ready"</p>
                    <p class="mt-4 text-sm text-gray-500">
                        "MVP focuses on backend API. Use curl or Postman to test endpoints."
                    </p>
                </div>

                <div class="bg-gray-800 p-8 rounded-lg shadow-xl space-y-4">
                    <h2 class="text-xl font-semibold text-gray-200">"API Endpoints"</h2>

                    <div class="space-y-3 text-sm font-mono">
                        <div class="bg-gray-700 p-3 rounded">
                            <span class="text-green-400">"POST"</span>
                            <span class="text-gray-300">" /api/auth/login"</span>
                            <div class="text-xs text-gray-400 mt-1">
                                "Default: admin / orbit2025"
                            </div>
                        </div>

                        <div class="bg-gray-700 p-3 rounded">
                            <span class="text-blue-400">"GET"</span>
                            <span class="text-gray-300">" /api/health"</span>
                        </div>

                        <div class="bg-gray-700 p-3 rounded">
                            <span class="text-purple-400">"WS"</span>
                            <span class="text-gray-300">" /ws/*path"</span>
                        </div>
                    </div>

                    <div class="mt-4 p-3 bg-blue-900/30 border border-blue-700 rounded text-xs">
                        <p class="text-blue-200">
                            "Full UI components coming in v1.0.0-beta. Current MVP provides production-ready backend APIs."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/* Client-side interactive login - disabled for MVP compilation
#[component]
pub fn LoginInteractive() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(false);

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

    view! { /* ... interactive form code ... */ }
}
*/
