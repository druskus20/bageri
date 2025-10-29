use crate::config::{Config, Page};
use std::collections::HashMap;

pub fn generate_html(config: &Config, page_name: &str, page: &Page) -> String {
    let title = page.title.as_ref().unwrap_or(&config.title);

    let meta_tags = generate_meta_tags(&config.meta);
    let favicon_tag = config.favicon.as_ref()
        .map(|f| format!(r#"    <link rel="icon" href="{}" />"#, f))
        .unwrap_or_default();

    let env_object = generate_env_object(&config.env);

    let favicon_line = if favicon_tag.is_empty() {
        String::new()
    } else {
        format!("{}\n", favicon_tag)
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
{favicon_line}{meta_tags}
    <script>
        // Inject environment variables
        window.ENV = {env_object};
    </script>
</head>
<body>
    <div id="app"></div>
    <script type="module" src="./{script}"></script>
</body>
</html>"#,
        title = title,
        favicon_line = favicon_line,
        meta_tags = meta_tags,
        env_object = env_object,
        script = page.script
    )
}

fn generate_meta_tags(meta: &HashMap<String, String>) -> String {
    meta.iter()
        .map(|(name, content)| {
            format!(r#"    <meta name="{}" content="{}" />"#,
                escape_html(name),
                escape_html(content)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn generate_env_object(env: &HashMap<String, String>) -> String {
    let entries = env.iter()
        .map(|(key, value)| {
            format!(r#"            "{}": "{}""#,
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}