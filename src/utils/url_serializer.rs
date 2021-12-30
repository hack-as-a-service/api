use provisioner::caddy::Url;
use serde::de::{Deserializer, Error as DeError, Unexpected as DeUnexpected, Visitor as DeVisitor};
use serde::ser::Serializer;

pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Url, D::Error> {
	struct UrlVisitor;

	impl<'de> DeVisitor<'de> for UrlVisitor {
		type Value = Url;

		fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			v.parse()
				.map_err(|_| DeError::invalid_value(DeUnexpected::Str(v), &self))
		}

		fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(formatter, "a URL")
		}
	}

	de.deserialize_str(UrlVisitor)
}

pub fn serialize<S: Serializer>(url: &Url, ser: S) -> Result<S::Ok, S::Error> {
	ser.serialize_str(&url.to_string())
}
