mod identity;
mod metadata;

pub(crate) use identity::{file_identity_from_file, hard_link_count_from_file, FileIdentity};
pub(crate) use metadata::{has_only_default_data_stream, is_link_or_reparse, is_offline_or_recall};
