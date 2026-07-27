use super::{quarantine_executor, types::CleanupPlan};
use crate::{
    audit,
    capability_policy::{CapabilityPolicy, DangerousWriteCapability},
    models::*,
    quarantine::QuarantineService,
    scanner,
};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

pub(crate) fn execute(
    plan: CleanupPlan,
    quarantine: &QuarantineService,
    app: &AppHandle,
) -> ExecuteResult {
    let plan_id = plan.preview.plan_id.clone();
    let total_items = plan.snapshots.len();
    let total_files = plan
        .snapshots
        .iter()
        .map(|snapshot| snapshot.item().file_count)
        .sum::<usize>();
    let mut result = ExecuteResult {
        reclaimed_bytes: 0,
        staged_bytes: 0,
        succeeded: 0,
        failed: Vec::new(),
    };
    let mut cleanup_record = audit::OperationRecord::new(
        audit::OperationKind::Cleanup,
        plan.preview.rule_version.clone(),
    );
    let mut quarantine_record = audit::OperationRecord::new(
        audit::OperationKind::Quarantine,
        plan.preview.rule_version.clone(),
    );
    let mut completed_items = 0usize;
    let mut completed_files = 0usize;

    emit_progress(
        app,
        CleanupProgress {
            phase: "starting".into(),
            completed_items,
            total_items,
            completed_files,
            total_files,
            current_item_id: String::new(),
            current_item_name: String::new(),
            current_path: String::new(),
            reclaimed_bytes: 0,
            failed_files: 0,
        },
    );

    for snapshot in plan.snapshots {
        let id = snapshot.item().id.clone();
        let item_name = snapshot.item().name.clone();
        let item_root = snapshot.item().path.clone();
        let item_files = snapshot.item().file_count;
        emit_progress(
            app,
            CleanupProgress {
                phase: "running".into(),
                completed_items,
                total_items,
                completed_files,
                total_files,
                current_item_id: id.clone(),
                current_item_name: item_name.clone(),
                current_path: item_root.clone(),
                reclaimed_bytes: result.reclaimed_bytes,
                failed_files: result.failed.len(),
            },
        );

        let reclaimed_before = result.reclaimed_bytes;
        let failed_before = result.failed.len();
        let files_before = completed_files;
        let items_before = completed_items;
        let progress_id = id.clone();
        let progress_name = item_name.clone();
        let delete_mode = snapshot.item().delete_mode.clone();
        let outcome = execute_item(
            &snapshot,
            &plan_id,
            quarantine,
            |item_completed, current_path, item_reclaimed, item_failed| {
                emit_progress(
                    app,
                    CleanupProgress {
                        phase: "running".into(),
                        completed_items: items_before,
                        total_items,
                        completed_files: files_before.saturating_add(item_completed),
                        total_files,
                        current_item_id: progress_id.clone(),
                        current_item_name: progress_name.clone(),
                        current_path: current_path.to_string_lossy().into_owned(),
                        reclaimed_bytes: reclaimed_before.saturating_add(item_reclaimed),
                        failed_files: failed_before.saturating_add(item_failed),
                    },
                );
            },
        );
        result.reclaimed_bytes = result
            .reclaimed_bytes
            .saturating_add(outcome.reclaimed_bytes);
        result.staged_bytes = result.staged_bytes.saturating_add(outcome.staged_bytes);
        let record = if matches!(delete_mode, DeleteMode::Quarantine) {
            &mut quarantine_record
        } else {
            &mut cleanup_record
        };
        record.reclaimed_bytes = record
            .reclaimed_bytes
            .saturating_add(outcome.reclaimed_bytes);
        record.staged_bytes = record.staged_bytes.saturating_add(outcome.staged_bytes);
        if outcome.reclaimed_bytes > 0 || outcome.staged_bytes > 0 || outcome.failures.is_empty() {
            record.succeeded.push(audit::OperationDetail {
                item_id: id.clone(),
                path: None,
                bytes: outcome.reclaimed_bytes.saturating_add(outcome.staged_bytes),
                detail: if matches!(delete_mode, DeleteMode::Quarantine) {
                    format!(
                        "已提交 {} 个可校验隔离对象；隔离占用不计入实际释放",
                        outcome.quarantine_record_ids.len()
                    )
                } else {
                    "已按不可变清理计划逐文件复检并处理".into()
                },
            });
        }
        if outcome.failures.is_empty() {
            result.succeeded = result.succeeded.saturating_add(1);
        } else {
            for failure in outcome.failures {
                let error = failure.error;
                result.failed.push(ItemFailure {
                    id: id.clone(),
                    error: error.clone(),
                    path: Some(failure.path.display().to_string()),
                });
                record.skipped.push(audit::OperationDetail {
                    item_id: id.clone(),
                    path: None,
                    bytes: 0,
                    detail: error,
                });
            }
        }
        completed_items = completed_items.saturating_add(1);
        completed_files = completed_files.saturating_add(item_files);
        emit_progress(
            app,
            CleanupProgress {
                phase: "item_complete".into(),
                completed_items,
                total_items,
                completed_files,
                total_files,
                current_item_id: id,
                current_item_name: item_name,
                current_path: item_root,
                reclaimed_bytes: result.reclaimed_bytes,
                failed_files: result.failed.len(),
            },
        );
    }

    emit_progress(
        app,
        CleanupProgress {
            phase: "complete".into(),
            completed_items: total_items,
            total_items,
            completed_files: total_files,
            total_files,
            current_item_id: String::new(),
            current_item_name: String::new(),
            current_path: String::new(),
            reclaimed_bytes: result.reclaimed_bytes,
            failed_files: result.failed.len(),
        },
    );

    finish_and_persist_audit(&mut cleanup_record);
    finish_and_persist_audit(&mut quarantine_record);
    result
}

#[derive(Debug)]
struct ItemExecutionFailure {
    path: PathBuf,
    error: String,
}

#[derive(Debug, Default)]
struct ItemExecutionOutcome {
    reclaimed_bytes: u64,
    staged_bytes: u64,
    quarantine_record_ids: Vec<String>,
    failures: Vec<ItemExecutionFailure>,
}

fn execute_item<F>(
    snapshot: &scanner::CleanupSnapshot,
    plan_id: &str,
    quarantine: &QuarantineService,
    on_progress: F,
) -> ItemExecutionOutcome
where
    F: FnMut(usize, &std::path::Path, u64, usize),
{
    execute_item_with_policy(
        snapshot,
        plan_id,
        quarantine,
        CapabilityPolicy::compiled(),
        on_progress,
    )
}

fn execute_item_with_policy<F>(
    snapshot: &scanner::CleanupSnapshot,
    plan_id: &str,
    quarantine: &QuarantineService,
    policy: CapabilityPolicy,
    mut on_progress: F,
) -> ItemExecutionOutcome
where
    F: FnMut(usize, &std::path::Path, u64, usize),
{
    let required_capability = match snapshot.item().delete_mode {
        DeleteMode::Permanent => Some(DangerousWriteCapability::PermanentOriginalFileDelete),
        DeleteMode::Quarantine => {
            Some(DangerousWriteCapability::ExperimentalQuarantineSourceRemoval)
        }
        DeleteMode::RecycleBin => None,
    };
    if let Some(capability) = required_capability {
        if let Err(error) = policy.require(capability) {
            return ItemExecutionOutcome {
                failures: vec![ItemExecutionFailure {
                    path: PathBuf::from(&snapshot.item().path),
                    error,
                }],
                ..ItemExecutionOutcome::default()
            };
        }
    }

    match snapshot.item().delete_mode {
        DeleteMode::Permanent => {
            let outcome = scanner::execute_with_progress(
                snapshot,
                |completed, _, path, reclaimed, failed| {
                    on_progress(completed, path, reclaimed, failed);
                },
            );
            ItemExecutionOutcome {
                reclaimed_bytes: outcome.reclaimed_bytes,
                staged_bytes: 0,
                quarantine_record_ids: Vec::new(),
                failures: outcome
                    .failures
                    .into_iter()
                    .map(|failure| ItemExecutionFailure {
                        path: failure.path,
                        error: failure.error,
                    })
                    .collect(),
            }
        }
        DeleteMode::Quarantine => {
            let outcome = quarantine_executor::execute_with_progress(
                snapshot,
                plan_id,
                quarantine,
                |completed, _, path, _, failed| {
                    on_progress(completed, path, 0, failed);
                },
            );
            ItemExecutionOutcome {
                reclaimed_bytes: 0,
                staged_bytes: outcome.staged_bytes,
                quarantine_record_ids: outcome
                    .records
                    .into_iter()
                    .map(|record| record.record_id)
                    .collect(),
                failures: outcome
                    .failures
                    .into_iter()
                    .map(|failure| ItemExecutionFailure {
                        path: failure.path,
                        error: failure.error,
                    })
                    .collect(),
            }
        }
        DeleteMode::RecycleBin => ItemExecutionOutcome {
            failures: vec![ItemExecutionFailure {
                path: PathBuf::from(&snapshot.item().path),
                error: "回收站执行器尚未启用，条目已安全保留".into(),
            }],
            ..ItemExecutionOutcome::default()
        },
    }
}

fn emit_progress(app: &AppHandle, progress: CleanupProgress) {
    let _ = app.emit("cleanup-progress", progress);
}

fn audit_status(record: &audit::OperationRecord) -> audit::OperationStatus {
    if !record.failed.is_empty() {
        if record.succeeded.is_empty() && record.skipped.is_empty() {
            audit::OperationStatus::Failed
        } else {
            audit::OperationStatus::PartiallySucceeded
        }
    } else if !record.skipped.is_empty() {
        if record.succeeded.is_empty() {
            audit::OperationStatus::Skipped
        } else {
            audit::OperationStatus::PartiallySucceeded
        }
    } else {
        audit::OperationStatus::Succeeded
    }
}

fn finish_and_persist_audit(record: &mut audit::OperationRecord) {
    if record.succeeded.is_empty() && record.failed.is_empty() && record.skipped.is_empty() {
        return;
    }
    record.completed_at_ms = audit::unix_time_ms();
    record.status = audit_status(record);
    persist_audit(record);
}

fn persist_audit(record: &audit::OperationRecord) {
    let outcome = audit::default_path().and_then(|path| audit::append_record(path, record));
    if let Err(error) = outcome {
        eprintln!("failed to persist local audit record: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_production_release_denies_without_dispatch(
        delete_mode: DeleteMode,
        expected_error: &str,
    ) {
        let snapshot = scanner::CleanupSnapshot::test_snapshot_with_mode(
            "temp",
            "must-not-execute",
            RiskLevel::Low,
            delete_mode,
            1,
            128,
        );
        let quarantine = QuarantineService::default();
        let mut progress_calls = 0usize;

        let outcome = execute_item_with_policy(
            &snapshot,
            "plan-test",
            &quarantine,
            CapabilityPolicy::production_release_for_test(),
            |_, _, _, _| progress_calls += 1,
        );

        assert_eq!(progress_calls, 0);
        assert_eq!(outcome.reclaimed_bytes, 0);
        assert_eq!(outcome.staged_bytes, 0);
        assert!(outcome.quarantine_record_ids.is_empty());
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].path, PathBuf::from("must-not-execute"));
        assert!(outcome.failures[0].error.contains(expected_error));
    }

    #[test]
    fn production_release_denies_permanent_delete_before_dispatch() {
        assert_production_release_denies_without_dispatch(DeleteMode::Permanent, "原文件永久删除");
    }

    #[test]
    fn production_release_denies_quarantine_staging_before_dispatch() {
        assert_production_release_denies_without_dispatch(DeleteMode::Quarantine, "隔离源文件移除");
    }
}
