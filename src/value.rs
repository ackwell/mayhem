use enum_as_inner::EnumAsInner;
use thiserror::Error;

/// Value of a field in a Node.
#[allow(missing_docs)]
#[derive(Clone, Debug, EnumAsInner)]
pub enum Value {
	U8(u8),
	I32(i32),
	F32(f32),
	String(String),
	Node(usize),
	Vector(Vec<Value>),
}

#[derive(Debug, Error)]
#[error("Expected {expected}, got {value:?}.")]
pub struct TryFromValueError {
	value: Value,
	expected: &'static str,
}

impl TryFrom<&Value> for u8 {
	type Error = TryFromValueError;

	fn try_from(value: &Value) -> Result<Self, Self::Error> {
		value.as_u8().cloned().ok_or(TryFromValueError {
			value: value.clone(),
			expected: "U8",
		})
	}
}

impl TryFrom<&Value> for i32 {
	type Error = TryFromValueError;

	fn try_from(value: &Value) -> Result<Self, Self::Error> {
		value.as_i32().cloned().ok_or(TryFromValueError {
			value: value.clone(),
			expected: "I32",
		})
	}
}

impl TryFrom<&Value> for f32 {
	type Error = TryFromValueError;

	fn try_from(value: &Value) -> Result<Self, Self::Error> {
		value.as_f32().cloned().ok_or(TryFromValueError {
			value: value.clone(),
			expected: "F32",
		})
	}
}

impl TryFrom<&Value> for String {
	type Error = TryFromValueError;

	fn try_from(value: &Value) -> Result<Self, Self::Error> {
		value.as_string().cloned().ok_or(TryFromValueError {
			value: value.clone(),
			expected: "String",
		})
	}
}

impl<'a, T> TryFrom<&'a Value> for Vec<T>
where
	T: TryFrom<&'a Value, Error = TryFromValueError>,
{
	type Error = TryFromValueError;

	fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
		value
			.as_vector()
			.ok_or(TryFromValueError {
				value: value.clone(),
				expected: "Vector",
			})?
			.iter()
			.map(|x| T::try_from(x))
			.collect()
	}
}
