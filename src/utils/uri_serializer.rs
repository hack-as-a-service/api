use provisioner::hyper::Uri;
use serde::de::{Deserializer, Error as DeError, Unexpected as DeUnexpected, Visitor as DeVisitor};
use serde::ser::Serializer;

pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Uri, D::Error> {
	struct UriVisitor;

	impl<'de> DeVisitor<'de> for UriVisitor {
		type Value = Uri;

		fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			v.parse()
				.map_err(|_| DeError::invalid_value(DeUnexpected::Str(v), &self))
		}

		fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(formatter, "a URI")
		}
	}

	de.deserialize_str(UriVisitor)
}

#[allow(dead_code)]
pub fn serialize<S: Serializer>(uri: &Uri, ser: S) -> Result<S::Ok, S::Error> {
	ser.serialize_str(&uri.to_string())
}
