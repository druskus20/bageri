use crate::config::{Config, Page, PageAttributes};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::HashMap;

pub fn generate_html(config: &Config, page: &Page) -> String {
    let title = if page.attributes.title.is_empty() {
        &config.default_page_attributes.title
    } else {
        &page.attributes.title
    };

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                @if !page.attributes.favicon.is_empty() || !config.default_page_attributes.favicon.is_empty() {
                    @let favicon = if !page.attributes.favicon.is_empty() { &page.attributes.favicon } else { &config.default_page_attributes.favicon };
                    link rel="icon" href=(favicon);
                }
                (generate_meta_tags(&page.attributes, &config.default_page_attributes))
                // Include global scripts first
                @for script in &config.default_page_attributes.scripts {
                    script type="module" src=(script) {}
                }
                // Then include page-specific scripts
                @for script in &page.attributes.scripts {
                    script type="module" src=(script) {}
                }
                script {
                    (PreEscaped(format!("// Inject environment variables\nwindow.ENV = {};", generate_env_object(&config.env))))
                }
            }
            body {
                div id="app" {}
            }
        }
    };

    markup.into_string()
}

fn generate_meta_tags(page_attrs: &PageAttributes, default_attrs: &PageAttributes) -> Markup {
    html! {
        @if !page_attrs.author.is_empty() || !default_attrs.author.is_empty() {
            @let author = if !page_attrs.author.is_empty() { &page_attrs.author } else { &default_attrs.author };
            meta name="author" content=(author);
        }
        @if !page_attrs.description.is_empty() || !default_attrs.description.is_empty() {
            @let description = if !page_attrs.description.is_empty() { &page_attrs.description } else { &default_attrs.description };
            meta name="description" content=(description);
        }
    }
}

fn generate_env_object(env: &HashMap<String, String>) -> String {
    let entries = env
        .iter()
        .map(|(key, value)| {
            format!(
                r#"            "{}": "{}""#,
                escape_js(key),
                escape_js(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    if entries.is_empty() {
        "{}".to_string()
    } else {
        format!("{{\n{entries}\n        }}")
    }
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
