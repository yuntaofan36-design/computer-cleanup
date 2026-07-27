use crate::quarantine::{QuarantineCandidate, QuarantineService, StageResult};
use crate::scanner::{self, CleanupSnapshot};
use std::path::{Path, PathBuf};

const PREVIEW_RULE_ID: &str = "temp";

#[derive(Debug)]
pub(crate) struct QuarantineFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Default)]
pub(crate) struct QuarantineOutcome {
    pub staged_bytes: u64,
    pub records: Vec<StageResult>,
    pub failures: Vec<QuarantineFailure>,
}

fn stage_result_failure_detail(result: &StageResult) -> Option<String> {
    if result.recovery_required {
        return Some(
            result
                .detail
                .clone()
                .unwrap_or_else(|| "隔离记录需要人工核对；普通恢复和导出操作已禁用".into()),
        );
    }
    if result.source_retained {
        return Some(
            result
                .detail
                .clone()
                .unwrap_or_else(|| "隔离副本已提交，但源文件已安全保留".into()),
        );
    }
    None
}

pub(crate) fn execute_with_progress<F>(
    snapshot: &CleanupSnapshot,
    plan_id: &str,
    service: &QuarantineService,
    mut on_progress: F,
) -> QuarantineOutcome
where
    F: FnMut(usize, usize, &Path, u64, usize),
{
    let files = scanner::execution_files(snapshot);
    let total_files = files.len();
    let mut outcome = QuarantineOutcome::default();

    if snapshot.item().id != PREVIEW_RULE_ID {
        outcome.failures.push(QuarantineFailure {
            path: PathBuf::from(&snapshot.item().path),
            error: "实验性隔离当前只允许 Windows 临时文件规则，其他规则已安全保留".into(),
        });
        return outcome;
    }

    let report_every = total_files
        .saturating_add(99)
        .checked_div(100)
        .unwrap_or(1)
        .max(1);
    for (index, file) in files.into_iter().enumerate() {
        let candidate = QuarantineCandidate {
            source_path: file.path.clone(),
            file_name: file.file_name,
            rule_id: snapshot.item().id.clone(),
            plan_id: plan_id.to_string(),
            expected_size: file.size,
        };
        match service.stage_file(candidate, |path| {
            scanner::revalidate_execution_file(snapshot, path)
        }) {
            Ok(result) => {
                outcome.staged_bytes = outcome.staged_bytes.saturating_add(result.size_bytes);
                if let Some(error) = stage_result_failure_detail(&result) {
                    outcome.failures.push(QuarantineFailure {
                        path: file.path.clone(),
                        error,
                    });
                }
                outcome.records.push(result);
            }
            Err(error) => outcome.failures.push(QuarantineFailure {
                path: file.path.clone(),
                error,
            }),
        }

        let completed_files = index + 1;
        if completed_files == total_files || completed_files % report_every == 0 {
            on_progress(
                completed_files,
                total_files,
                &file.path,
                outcome.staged_bytes,
                outcome.failures.len(),
            );
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(source_retained: bool, recovery_required: bool) -> StageResult {
        StageResult {
            record_id: "record-1".into(),
            size_bytes: 1024,
            source_retained,
            recovery_required,
            detail: None,
        }
    }

    #[test]
    fn recovery_required_stage_result_is_a_failure() {
        let detail = stage_result_failure_detail(&result(false, true))
            .expect("recovery-required records must require review");
        assert!(detail.contains("人工核对"));
    }

    #[test]
    fn committed_stage_result_is_not_a_failure() {
        assert!(stage_result_failure_detail(&result(false, false)).is_none());
    }
}
