use blobatar_core::{Avatar, Generation, Options};
use url::Url;

pub(crate) struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl Reply {
    fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.to_owned(),
        }
    }
}

pub(crate) fn handle(method: &str, url: &Url) -> Reply {
    let mut reply = if !matches!(url.path(), "/healthz" | "/probe/avatar.svg") {
        Reply::text(404, "Not found")
    } else if !matches!(method, "GET" | "HEAD") {
        Reply::text(405, "Use GET or HEAD")
    } else if url.path() == "/healthz" {
        Reply::text(200, "Blobatar P0 Worker probe")
    } else {
        avatar(url)
    };
    if method == "HEAD" {
        reply.body.clear();
    }
    reply
}

fn avatar(url: &Url) -> Reply {
    let mut name = None;
    let mut generation = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "name" if name.is_none() => name = Some(value.into_owned()),
            "generation" if generation.is_none() => generation = Some(value.into_owned()),
            _ => return Reply::text(400, "Unknown or duplicate parameter"),
        }
    }
    let name = name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or("blobatar");
    if name.len() > 256 {
        return Reply::text(400, "Name exceeds 256 UTF-8 bytes");
    }
    let generation = match generation.as_deref().unwrap_or("2") {
        "1" => Generation::One,
        "2" => Generation::Two,
        _ => return Reply::text(400, "Generation must be 1 or 2"),
    };
    let options = Options::default();
    Reply {
        status: 200,
        content_type: "image/svg+xml; charset=utf-8",
        body: Avatar::with_generation(name, &options, generation).svg(&options),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, path: &str) -> Reply {
        handle(
            method,
            &Url::parse(&format!("http://localhost{path}")).unwrap(),
        )
    }

    #[test]
    fn matches_existing_core_for_both_generations_and_unicode() {
        for (number, generation) in [("1", Generation::One), ("2", Generation::Two)] {
            let url = Url::parse_with_params(
                "http://localhost/probe/avatar.svg",
                [("name", "ひろと + 🦀"), ("generation", number)],
            )
            .unwrap();
            let reply = handle("GET", &url);
            let options = Options::default();
            assert_eq!(reply.status, 200);
            assert_eq!(reply.content_type, "image/svg+xml; charset=utf-8");
            assert_eq!(
                reply.body,
                Avatar::with_generation("ひろと + 🦀", &options, generation).svg(&options)
            );
        }
    }

    #[test]
    fn absent_and_empty_names_use_blobatar() {
        for generation in [1, 2] {
            let expected = request(
                "GET",
                &format!("/probe/avatar.svg?name=blobatar&generation={generation}"),
            );
            for name in ["", "&name="] {
                let reply = request(
                    "GET",
                    &format!("/probe/avatar.svg?generation={generation}{name}"),
                );
                assert_eq!(reply.status, 200);
                assert_eq!(reply.body, expected.body);
            }
        }
    }

    #[test]
    fn rejects_unsupported_ambiguous_or_oversized_queries() {
        for query in [
            "generation=3",
            "generation=",
            "generation=1&generation=2",
            "name=a&name=b",
            "size=100",
        ] {
            assert_eq!(
                request("GET", &format!("/probe/avatar.svg?{query}")).status,
                400,
                "{query}"
            );
        }
        assert_eq!(
            request(
                "GET",
                &format!("/probe/avatar.svg?name={}", "x".repeat(257))
            )
            .status,
            400
        );
        assert_eq!(
            request(
                "GET",
                &format!("/probe/avatar.svg?name={}", "x".repeat(256))
            )
            .status,
            200
        );
    }

    #[test]
    fn routes_methods_and_head_bodies() {
        assert_eq!(request("GET", "/healthz").status, 200);
        assert_eq!(request("GET", "/wall/place").status, 404);
        assert_eq!(request("GET", "/avatar/name").status, 404);
        assert_eq!(request("POST", "/probe/avatar.svg").status, 405);
        for path in [
            "/healthz",
            "/probe/avatar.svg",
            "/missing",
            "/probe/avatar.svg?generation=3",
        ] {
            let head = request("HEAD", path);
            let get = request("GET", path);
            assert_eq!(head.status, get.status);
            assert_eq!(head.content_type, get.content_type);
            assert!(head.body.is_empty());
        }
    }
}
