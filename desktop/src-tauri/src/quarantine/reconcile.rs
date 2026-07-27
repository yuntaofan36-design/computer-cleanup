use super::types::{JournalEntry, JournalEvent, QuarantineRecordState};
use std::io::{self, ErrorKind};

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message.into())
}

pub(crate) fn derive_state(entries: &[JournalEntry]) -> io::Result<QuarantineRecordState> {
    let Some(last) = entries.last() else {
        return Err(invalid_data("quarantine journal is empty"));
    };

    let mut expected = JournalEvent::Prepared;
    for (index, entry) in entries.iter().enumerate() {
        if entry.event != expected {
            return Err(invalid_data(format!(
                "invalid quarantine transition at sequence {}: expected {:?}, found {:?}",
                index + 1,
                expected,
                entry.event
            )));
        }

        expected = match entry.event {
            JournalEvent::Prepared => JournalEvent::Copying,
            JournalEvent::Copying => JournalEvent::ObjectVerified,
            JournalEvent::ObjectVerified => JournalEvent::ObjectCommitted,
            JournalEvent::ObjectCommitted => {
                if entries
                    .get(index + 1)
                    .is_some_and(|next| matches!(next.event, JournalEvent::SourceRetained))
                {
                    JournalEvent::SourceRetained
                } else {
                    JournalEvent::SourceDeletePrepared
                }
            }
            JournalEvent::SourceDeletePrepared => {
                if entries
                    .get(index + 1)
                    .is_some_and(|next| matches!(next.event, JournalEvent::SourceRetained))
                {
                    JournalEvent::SourceRetained
                } else {
                    JournalEvent::Committed
                }
            }
            JournalEvent::Committed | JournalEvent::SourceRetained => {
                if index + 1 != entries.len() {
                    return Err(invalid_data(
                        "quarantine journal contains an entry after a terminal event",
                    ));
                }
                entry.event
            }
        };
    }

    Ok(match last.event {
        JournalEvent::Committed => QuarantineRecordState::Committed,
        JournalEvent::SourceRetained | JournalEvent::ObjectCommitted => {
            QuarantineRecordState::SourceRetained
        }
        JournalEvent::Prepared
        | JournalEvent::Copying
        | JournalEvent::ObjectVerified
        | JournalEvent::SourceDeletePrepared => QuarantineRecordState::RecoveryRequired,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quarantine::types::QUARANTINE_SCHEMA_VERSION;

    fn entries(events: &[JournalEvent]) -> Vec<JournalEntry> {
        events
            .iter()
            .enumerate()
            .map(|(index, event)| JournalEntry {
                schema_version: QUARANTINE_SCHEMA_VERSION,
                sequence: index as u64 + 1,
                previous_entry_sha256: None,
                occurred_at_ms: index as u64,
                event: *event,
            })
            .collect()
    }

    #[test]
    fn maps_supported_terminal_and_uncertain_states() {
        let committed = entries(&[
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
            JournalEvent::SourceDeletePrepared,
            JournalEvent::Committed,
        ]);
        assert_eq!(
            derive_state(&committed).unwrap(),
            QuarantineRecordState::Committed
        );

        let retained = entries(&[
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
        ]);
        assert_eq!(
            derive_state(&retained).unwrap(),
            QuarantineRecordState::SourceRetained
        );

        let uncertain = entries(&[
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
            JournalEvent::SourceDeletePrepared,
        ]);
        assert_eq!(
            derive_state(&uncertain).unwrap(),
            QuarantineRecordState::RecoveryRequired
        );
    }

    #[test]
    fn allows_source_retained_before_or_after_delete_is_armed() {
        for events in [
            vec![
                JournalEvent::Prepared,
                JournalEvent::Copying,
                JournalEvent::ObjectVerified,
                JournalEvent::ObjectCommitted,
                JournalEvent::SourceRetained,
            ],
            vec![
                JournalEvent::Prepared,
                JournalEvent::Copying,
                JournalEvent::ObjectVerified,
                JournalEvent::ObjectCommitted,
                JournalEvent::SourceDeletePrepared,
                JournalEvent::SourceRetained,
            ],
        ] {
            assert_eq!(
                derive_state(&entries(&events)).unwrap(),
                QuarantineRecordState::SourceRetained
            );
        }
    }

    #[test]
    fn rejects_skipped_or_post_terminal_transitions() {
        let skipped = entries(&[JournalEvent::Prepared, JournalEvent::ObjectVerified]);
        assert_eq!(
            derive_state(&skipped).unwrap_err().kind(),
            ErrorKind::InvalidData
        );

        let post_terminal = entries(&[
            JournalEvent::Prepared,
            JournalEvent::Copying,
            JournalEvent::ObjectVerified,
            JournalEvent::ObjectCommitted,
            JournalEvent::SourceRetained,
            JournalEvent::SourceRetained,
        ]);
        assert_eq!(
            derive_state(&post_terminal).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
}
