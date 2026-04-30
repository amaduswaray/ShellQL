use std::fmt;

use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};
use url::Url;

#[derive(Debug)]
pub enum ConnectionError {
    InvalidUrl {
        input: String,
        error: url::ParseError,
    },
    UnsupportedScheme {
        input: String,
        scheme: String,
    },
    MissingHost {
        input: String,
    },
    MissingPath {
        input: String,
    },
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let renderer = Renderer::styled();
        let help = "use a connection string in the form: postgres://user:pass@host/dbname";

        let output = match self {
            ConnectionError::InvalidUrl { input, error } => {
                let msg = error.to_string();
                let report = &[Level::ERROR
                    .primary_title("invalid connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(0..input.len())
                                .label(msg.as_str()),
                        ),
                    )
                    .element(Level::HELP.message(help))];
                renderer.render(report).to_string()
            }

            ConnectionError::UnsupportedScheme { input, scheme } => {
                let scheme_end = scheme.len();
                let report = &[Level::ERROR
                    .primary_title("unsupported database scheme")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(0..scheme_end)
                                .label("this scheme is not supported"),
                        ),
                    )
                    .element(
                        Level::HELP
                            .message("supported schemes: postgres, postgresql, mysql, sqlite"),
                    )];
                renderer.render(report).to_string()
            }

            ConnectionError::MissingHost { input } => {
                let after_scheme = input.find("://").map(|i| i + 3).unwrap_or(input.len());
                let span_end = (after_scheme + 1).min(input.len()).max(after_scheme);
                let report = &[Level::ERROR
                    .primary_title("missing host in connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(after_scheme..span_end)
                                .label("expected a hostname here"),
                        ),
                    )
                    .element(Level::HELP.message(help))];
                renderer.render(report).to_string()
            }

            ConnectionError::MissingPath { input } => {
                let span_start = input.rfind('/').map(|i| i + 1).unwrap_or(input.len());
                let span_end = input.len().max(span_start + 1).min(input.len());
                let report = &[Level::ERROR
                    .primary_title("missing database name in connection string")
                    .element(
                        Snippet::source(input.as_str()).annotation(
                            AnnotationKind::Primary
                                .span(span_start..span_end)
                                .label("expected a database name after the last '/'"),
                        ),
                    )
                    .element(Level::HELP.message(help))];
                renderer.render(report).to_string()
            }
        };

        write!(f, "{output}")
    }
}

impl std::error::Error for ConnectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectionError::InvalidUrl { error, .. } => Some(error),
            _ => None,
        }
    }
}

pub fn validate_connection_string(conn: &str) -> Result<Url, ConnectionError> {
    let url = Url::parse(conn).map_err(|e| ConnectionError::InvalidUrl {
        input: conn.to_string(),
        error: e,
    })?;

    match url.scheme() {
        "postgres" | "postgresql" | "mysql" | "sqlite" => {}
        other => {
            return Err(ConnectionError::UnsupportedScheme {
                input: conn.to_string(),
                scheme: other.to_string(),
            });
        }
    }

    if url.scheme() != "sqlite" && url.host_str().is_none() {
        return Err(ConnectionError::MissingHost {
            input: conn.to_string(),
        });
    }

    let path = url.path();
    if path.is_empty() || path == "/" {
        return Err(ConnectionError::MissingPath {
            input: conn.to_string(),
        });
    }

    Ok(url)
}
