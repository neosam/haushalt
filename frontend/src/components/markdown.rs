use leptos::*;
use pulldown_cmark::{html, Parser};

/// Renders markdown content as HTML
#[component]
pub fn MarkdownView(
    /// The markdown content to render
    content: String,
) -> impl IntoView {
    let html_content = {
        let parser = Parser::new(&content);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    };

    view! {
        <div class="markdown-content" inner_html=html_content></div>
    }
}

/// Renders markdown content reactively (for signals)
#[component]
pub fn MarkdownViewReactive(
    /// Signal containing the markdown content
    content: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="markdown-content" inner_html=move || {
            let text = content.get();
            let parser = Parser::new(&text);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            html_output
        }></div>
    }
}
