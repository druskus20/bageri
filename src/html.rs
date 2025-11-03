use crate::config::{Config, Page};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::HashMap;

pub fn generate_html(config: &Config, page_name: &str, page: &Page) -> String {
    let title = page.title.as_ref().unwrap_or(&config.title);

    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                @if let Some(favicon) = &config.favicon {
                    link rel="icon" href=(favicon);
                }
                (generate_meta_tags(&config.meta))
                script {
                    (PreEscaped(format!("// Inject environment variables\nwindow.ENV = {};", generate_env_object(&config.env))))
                }
            }
            body {
                div id="app" {}
                script type="module" src=(format!("./{}", page.script)) {}
            }
        }
    };

    markup.into_string()
}

fn generate_meta_tags(meta: &HashMap<String, String>) -> Markup {
    html! {
        @for (name, content) in meta {
            meta name=(name) content=(content);
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
        format!("{{\n{}\n        }}", entries)
    }
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

