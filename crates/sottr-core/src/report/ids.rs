//! 契约表示规则：内部 ID（u32）、资源键（u64 字符串往返）与固定字符串类型。

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 报告契约版本。
pub const SCHEMA_VERSION: &str = "1.0.0";

/// 内部 ID 解析错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// 缺少 `0x` 前缀。
    MissingPrefix,
    /// 不是恰好 8 位。
    WrongLength,
    /// 含非大写十六进制字符。
    NotUpperHex,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            IdError::MissingPrefix => "internal id must start with 0x",
            IdError::WrongLength => "internal id must contain exactly 8 hex digits",
            IdError::NotUpperHex => "internal id must use uppercase hex digits",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for IdError {}

/// u32 内部 ID，序列化为 `0x` 加 8 位大写十六进制字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InternalId(u32);

impl InternalId {
    /// 从原始 u32 构造。
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// 原始 u32 值。
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for InternalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:08X}", self.0)
    }
}

impl FromStr for InternalId {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let hex = value.strip_prefix("0x").ok_or(IdError::MissingPrefix)?;
        if hex.len() != 8 {
            return Err(IdError::WrongLength);
        }
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
        {
            return Err(IdError::NotUpperHex);
        }
        u32::from_str_radix(hex, 16)
            .map(InternalId)
            .map_err(|_| IdError::WrongLength)
    }
}

impl Serialize for InternalId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

struct InternalIdVisitor;

impl Visitor<'_> for InternalIdVisitor {
    type Value = InternalId;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string like \"0x439B19A6\"")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<InternalId, E> {
        InternalId::from_str(value).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for InternalId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(InternalIdVisitor)
    }
}

/// u64 资源/本地化键：序列化为十进制字符串；反序列化只接受字符串（拒绝 JSON 数字）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceKey(u64);

impl ResourceKey {
    /// 从原始 u64 构造。
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// 原始 u64 值。
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for ResourceKey {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(hex) = value.strip_prefix("0x") {
            if hex.is_empty() || hex.len() > 16 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(IdError::NotUpperHex);
            }
            return u64::from_str_radix(hex, 16)
                .map(ResourceKey)
                .map_err(|_| IdError::WrongLength);
        }
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(IdError::NotUpperHex);
        }
        if value.len() > 1 && value.starts_with('0') {
            return Err(IdError::WrongLength);
        }
        value
            .parse::<u64>()
            .map(ResourceKey)
            .map_err(|_| IdError::WrongLength)
    }
}

impl Serialize for ResourceKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

struct ResourceKeyVisitor;

impl Visitor<'_> for ResourceKeyVisitor {
    type Value = ResourceKey;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a decimal or 0x-hex string for a 64-bit key")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<ResourceKey, E> {
        ResourceKey::from_str(value).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for ResourceKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(ResourceKeyVisitor)
    }
}

/// 生成仅接受固定字符串常量的序列化类型（用于 schema_version、document_kind 等）。
macro_rules! fixed_string {
    ($(#[$meta:meta])* $name:ident, $value:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl $name {
            /// 固定值。
            pub const VALUE: &'static str = $value;
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str($value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct FixedVisitor;

                impl Visitor<'_> for FixedVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str(concat!("the string \"", $value, "\""))
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<$name, E> {
                        if value == $value {
                            Ok($name)
                        } else {
                            Err(E::custom(concat!("expected \"", $value, "\"")))
                        }
                    }
                }

                deserializer.deserialize_str(FixedVisitor)
            }
        }
    };
}

fixed_string!(
    /// 契约 schema_version 固定 "1.0.0"。
    SchemaVersion,
    "1.0.0"
);

fixed_string!(
    /// 导出信封 document_kind 固定 "report_export"。
    ExportDocumentKind,
    "report_export"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_id_round_trip_is_upper_hex() {
        let id = InternalId::new(0x439B19A6);
        assert_eq!(id.to_string(), "0x439B19A6");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"0x439B19A6\"");
        let parsed: InternalId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn internal_id_rejects_malformed() {
        for value in [
            "439B19A6",
            "0x439b19a6",
            "0x439B19A",
            "0x439B19A61",
            "0xZZZZZZZZ",
        ] {
            assert!(
                serde_json::from_str::<InternalId>(&format!("\"{value}\"")).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn resource_key_max_round_trips_through_string() {
        let key = ResourceKey::new(u64::MAX);
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, "\"18446744073709551615\"");
        let parsed: ResourceKey = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.raw(), u64::MAX);
        let hex: ResourceKey = serde_json::from_str("\"0xFFFFFFFFFFFFFFFF\"").unwrap();
        assert_eq!(hex.raw(), u64::MAX);
    }

    #[test]
    fn resource_key_rejects_json_numbers() {
        assert!(serde_json::from_str::<ResourceKey>("18446744073709551615").is_err());
        assert!(serde_json::from_str::<ResourceKey>("9007199254740993").is_err());
        assert!(serde_json::from_str::<ResourceKey>("\"0\"").is_ok());
        assert!(serde_json::from_str::<ResourceKey>("0").is_err());
        assert!(serde_json::from_str::<ResourceKey>("\"0123\"").is_err());
    }

    #[test]
    fn schema_version_rejects_other_values() {
        assert!(serde_json::from_str::<SchemaVersion>("\"1.0.0\"").is_ok());
        assert!(serde_json::from_str::<SchemaVersion>("\"2\"").is_err());
        assert!(serde_json::from_str::<SchemaVersion>("1.0.0").is_err());
    }
}
