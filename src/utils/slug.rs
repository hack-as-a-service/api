use rand::prelude::*;
use regex::Regex;

lazy_static! {
    static ref SLUG_REGEX: Regex = Regex::new("^[a-z0-9\\-]+$").unwrap();
}

pub fn validate_slug(slug: &str) -> bool {
    SLUG_REGEX.is_match(slug)
}

/// Converts any string to a slug
pub fn into_slug(text: &str, randomize: bool) -> String {
    lazy_static! {
        static ref INVALID_REGEX: Regex = Regex::new("[^a-z0-9\\-]").unwrap();
    }

    let slug = INVALID_REGEX
        .replace_all(&text.trim().to_lowercase().replace(" ", "-"), "")
        .to_string();

    if randomize {
        let mut rng = thread_rng();

        format!("{}-{:4}", slug, rng.gen_range(0..10000))
    } else {
        slug
    }
}
