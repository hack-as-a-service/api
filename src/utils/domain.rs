use std::{net::Ipv4Addr, str::FromStr};

use regex::Regex;
use trust_dns_client::{
	client::{AsyncClient, ClientHandle},
	rr::{DNSClass, Name, RData, RecordType},
};

lazy_static! {
	static ref DOMAIN_REGEX: Regex = Regex::new("^([A-Za-z0-9-]{1,63}\\.)+[A-Za-z]{2,}$").unwrap();
}

pub fn validate_domain(domain: &str) -> bool {
	DOMAIN_REGEX.is_match(domain)
}

// Verifies that a domain's DNS configuration is correct
pub async fn verify_domain(client: &mut AsyncClient, domain: &str) -> Result<bool, String> {
	let name = Name::from_str(domain)?;
	let response = client
		.query(name, DNSClass::IN, RecordType::A)
		.await
		.map_err(|e| e.to_string())?;

	for answer in response.answers() {
		if let RData::A(addr) = answer.rdata() {
			if addr == &Ipv4Addr::new(167, 99, 113, 134) {
				return Ok(true);
			}
		}
	}

	Ok(false)
}
