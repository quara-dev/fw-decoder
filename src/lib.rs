use wasm_bindgen::prelude::*;
use yew::prelude::*;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use wasm_bindgen_futures::JsFuture;
use yew::platform::spawn_local;

#[function_component]
pub fn App(_props: &()) -> Html {
    let versions = use_state(|| Vec::<String>::new());
    let selected_version = use_state(|| String::new());
    let log_level = use_state(|| "4".to_string());
    let output = use_state(|| "".to_string());
    let file = use_state(|| None);

    // Fetch versions from backend on mount
    {
        let versions = versions.clone();
        let selected_version = selected_version.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let window = web_sys::window().unwrap();
                let resp_value = JsFuture::from(window.fetch_with_str("http://localhost:8080/api/versions")).await.unwrap();
                let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                let json = JsFuture::from(resp.json().unwrap()).await.unwrap();
                let arr = js_sys::Array::from(&json);
                let mut v = Vec::new();
                for i in 0..arr.length() {
                    v.push(arr.get(i).as_string().unwrap_or_default());
                }
                selected_version.set(v.get(0).cloned().unwrap_or_default());
                versions.set(v);
            });
            || ()
        });
    }

    let on_version_change = {
        let selected_version = selected_version.clone();
        Callback::from(move |e: Event| {
            let select = e.target_dyn_into::<HtmlSelectElement>().unwrap();
            selected_version.set(select.value());
        })
    };

    let on_log_level_change = {
        let log_level = log_level.clone();
        Callback::from(move |e: Event| {
            let select = e.target_dyn_into::<HtmlSelectElement>().unwrap();
            log_level.set(select.value());
        })
    };

    let on_file_upload = {
        let file = file.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>().unwrap();
            let file_obj = input.files().and_then(|fs| fs.get(0));
            file.set(file_obj);
        })
    };

    let on_submit = {
        let selected_version = selected_version.clone();
        let log_level = log_level.clone();
        let file = file.clone();
        let output = output.clone();
        Callback::from(move |_| {
            let version = (*selected_version).clone();
            let log_level = (*log_level).clone();
            let file_opt = (*file).clone();
            let output = output.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(file) = file_opt {
                    let form = web_sys::FormData::new().unwrap();
                    form.append_with_blob("file", &file).unwrap();
                    let url = format!("http://localhost:8080/api/decode?version={}&log_level={}", version, log_level);
                    let mut opts = web_sys::RequestInit::new();
                    opts.set_method("POST");
                    opts.set_body(form.as_ref());
                    let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();
                    let window = web_sys::window().unwrap();
                    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
                    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
                    let text = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap()).await.unwrap();
                    let raw = text.as_string().unwrap_or_else(|| format!("{:?}", text));
                    web_sys::console::log_1(&raw.clone().into());
                    output.set(raw);
                } else {
                    output.set("No file selected".to_string());
                }
            });
        })
    };

    html! {
        <div style="display:flex; height:100vh;">
            <div style="width:320px; min-width:220px; background:#f7f7f7; padding:2em; box-sizing:border-box; border-right:1px solid #ddd;">
                <h1 style="font-size:1.3em;">{ "FW Log Decoder" }</h1>
                <div style="margin-top:2em;">
                    <label>{ "Version: " }
                        <select id="version-select" onchange={on_version_change} style="width:100%;">
                            { for versions.iter().map(|v| html! { <option value={v.clone()}>{ v }</option> }) }
                        </select>
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <label>{ "Upload File: " }
                        <input type="file" id="file-input" onchange={on_file_upload} style="width:100%;" />
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <label>{ "Log Level: " }
                        <select id="loglevel-select" onchange={on_log_level_change} style="width:100%;">
                            <option value="1">{ "Fatal Error" }</option>
                            <option value="2">{ "Error" }</option>
                            <option value="3">{ "Warn" }</option>
                            <option value="4" selected=true>{ "Info" }</option>
                            <option value="5">{ "Debug" }</option>
                            <option value="6">{ "Trace" }</option>
                        </select>
                    </label>
                </div>
                <div style="margin-top:2em;">
                    <button onclick={on_submit} style="width:100%;padding:0.7em 0; font-size:1em;">{ "Submit" }</button>
                </div>
            </div>
            <div style="flex:1; display:flex; flex-direction:column;">
                <textarea id="output" rows=30 style="flex:1; width:100%; min-height:100%; resize:vertical; font-size:1em; padding:1em; box-sizing:border-box; border:none; background:#fff;" readonly=true placeholder="Log output will appear here..." value={(*output).clone()} />
            </div>
        </div>
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("root").unwrap();
    yew::Renderer::<App>::with_root(root).render();
}
