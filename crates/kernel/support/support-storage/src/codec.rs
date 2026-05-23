use crate::error::StorageError;
use crate::value::StorageValue;

pub trait StorageCodec: Send + Sync {
    fn encode(&self, value: &StorageValue) -> Result<Vec<u8>, StorageError>;
    fn decode(&self, bytes: &[u8]) -> Result<StorageValue, StorageError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonStorageCodec;

impl StorageCodec for JsonStorageCodec {
    fn encode(&self, value: &StorageValue) -> Result<Vec<u8>, StorageError> {
        serde_json::to_vec(value).map_err(|_| StorageError::SerializationFailed)
    }

    fn decode(&self, bytes: &[u8]) -> Result<StorageValue, StorageError> {
        serde_json::from_slice(bytes).map_err(|_| StorageError::DeserializationFailed)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn json_codec_roundtrips_value() {
        let codec = JsonStorageCodec;
        let value = json!({"name":"mini","items":[1,2,3]});

        let bytes = codec.encode(&value).unwrap();
        let decoded = codec.decode(&bytes).unwrap();

        assert_eq!(decoded, value);
    }
}
