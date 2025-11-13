use crate::config::{Config, HtmlPage, PageAttributes, SpaPage};
use crate::prelude::*;
use color_eyre::eyre::{Context, Result};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::collections::HashMap;

pub fn generate_html(config: &Config, page: &SpaPage) -> String {
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

                @for style in &config.default_page_attributes.styles {
                    link rel="stylesheet" href=(style);
                }

                @for style in &page.attributes.styles {
                    link rel="stylesheet" href=(style);
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

pub async fn find_html_files(page_name: &str, page: &HtmlPage) -> Result<Vec<String>> {
    if let Some(pattern) = &page.pattern {
        // Pattern-based file discovery
        let mut files = vec![];
        let mut entries = tokio::fs::read_dir("src")
            .await
            .wrap_err("Failed to read src directory")?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("Failed to read directory entry")?
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    if name_str.ends_with(".html") && glob_match(pattern, name_str) {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }

        if files.is_empty() {
            warn!("No HTML files found matching pattern '{}' in src/", pattern);
        }

        Ok(files)
    } else {
        // Use page name as filename
        let filename = if page_name == "index" {
            "src/index.html".to_string()
        } else {
            format!("src/{}.html", page_name)
        };

        if tokio::fs::metadata(&filename).await.is_ok() {
            Ok(vec![filename])
        } else {
            warn!("HTML file not found: {}", filename);
            Ok(vec![])
        }
    }
}

fn glob_match(pattern: &str, filename: &str) -> bool {
    if pattern.contains('*') {
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 2 {
            let prefix = pattern_parts[0];
            let suffix = pattern_parts[1];
            filename.starts_with(prefix) && filename.ends_with(suffix)
        } else {
            filename.contains(pattern)
        }
    } else {
        filename.contains(pattern)
    }
}

pub async fn process_html_page(
    config: &Config,
    page: &HtmlPage,
    input_file: &str,
) -> Result<String> {
    let content = tokio::fs::read_to_string(input_file)
        .await
        .wrap_err_with(|| format!("Failed to read HTML file: {}", input_file))?;

    let body_content = extract_body_content(&content)?;

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
            (PreEscaped(body_content))
        }
    };

    Ok(markup.into_string())
}

fn extract_body_content(html: &str) -> Result<String> {
    let html = html.trim();

    if let Some(head_start) = html.find("<head") {
        if let Some(head_end) = html.find("</head>") {
            warn!(
                "Found <head> section in HTML file, removing it as it will be replaced with custom head"
            );
            let before_head = &html[..head_start];
            let after_head = &html[head_end + 7..];
            let html = format!("{}{}", before_head, after_head);
            return extract_body_from_clean_html(&html);
        }
    }

    extract_body_from_clean_html(html)
}

fn extract_body_from_clean_html(html: &str) -> Result<String> {
    if let Some(body_start) = html.find("<body") {
        if let Some(body_content_start) = html[body_start..].find('>') {
            let body_start_pos = body_start + body_content_start + 1;

            if let Some(body_end) = html.rfind("</body>") {
                let body_content = html[body_start_pos..body_end].trim();
                return Ok(format!("<body>{}</body>", body_content));
            } else {
                warn!("No closing </body> tag found, assuming rest of content is body");
                let body_content = html[body_start_pos..].trim();
                return Ok(format!("<body>{}</body>", body_content));
            }
        }
    }

    warn!("No <body> tag found, treating entire content as body");
    Ok(format!("<body>{}</body>", html.trim()))
}
