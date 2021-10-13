use regex::Regex;

lazy_static! {
	static ref DOMAIN_REGEX: Regex = Regex::new("^([A-Za-z0-9-]{1,63}\\.)+[A-Za-z]{2,6}$").unwrap();
}

pub fn validate_domain(domain: &str) -> bool {
	DOMAIN_REGEX.is_match(domain)
}
