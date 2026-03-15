use crate::hashing::Hash;

pub struct FileMetadata {
    pub file_size_bytes: u64,
    pub file_modified_time: u128,
    #[allow(dead_code)]
    pub file_created_time: u128,
    pub file_hash: Hash,
}
