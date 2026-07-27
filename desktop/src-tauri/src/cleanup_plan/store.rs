use super::types::{
    CleanupPlan, CleanupPlanPreview, CleanupScanResponse, CreateCleanupPlanRequest,
    ExecuteCleanupPlanRequest,
};
use crate::{
    audit,
    models::{DeleteMode, RiskLevel},
    quarantine::{MAX_QUARANTINE_PLAN_BYTES, MAX_QUARANTINE_PLAN_FILES},
    scanner::CleanupSnapshot,
};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const DEFAULT_SCAN_TTL_MS: u64 = 15 * 60 * 1_000;
const DEFAULT_PLAN_TTL_MS: u64 = 10 * 60 * 1_000;
const DEFAULT_MAX_SCAN_SESSIONS: usize = 4;
const DEFAULT_MAX_PLANS: usize = 8;
const MAX_PLAN_ITEMS: usize = 100;

#[derive(Clone, Copy)]
struct StoreConfig {
    scan_ttl_ms: u64,
    plan_ttl_ms: u64,
    max_scan_sessions: usize,
    max_plans: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            scan_ttl_ms: DEFAULT_SCAN_TTL_MS,
            plan_ttl_ms: DEFAULT_PLAN_TTL_MS,
            max_scan_sessions: DEFAULT_MAX_SCAN_SESSIONS,
            max_plans: DEFAULT_MAX_PLANS,
        }
    }
}

struct ScanSession {
    scan_id: Uuid,
    rule_version: String,
    created_at_ms: u64,
    expires_at_ms: u64,
    snapshots: HashMap<String, CleanupSnapshot>,
}

#[derive(Default)]
struct StoreInner {
    scans: HashMap<Uuid, ScanSession>,
    plans: HashMap<Uuid, CleanupPlan>,
}

pub(crate) struct CleanupPlanStore {
    config: StoreConfig,
    inner: Mutex<StoreInner>,
}

impl Default for CleanupPlanStore {
    fn default() -> Self {
        Self {
            config: StoreConfig::default(),
            inner: Mutex::new(StoreInner::default()),
        }
    }
}

impl CleanupPlanStore {
    pub(crate) fn record_scan(
        &self,
        rule_version: &str,
        snapshots: Vec<CleanupSnapshot>,
    ) -> Result<CleanupScanResponse, String> {
        self.record_scan_at(rule_version, snapshots, audit::unix_time_ms())
    }

    pub(crate) fn create_plan(
        &self,
        request: CreateCleanupPlanRequest,
    ) -> Result<CleanupPlanPreview, String> {
        self.create_plan_at(request, audit::unix_time_ms())
    }

    pub(crate) fn take_confirmed_plan(
        &self,
        request: ExecuteCleanupPlanRequest,
    ) -> Result<CleanupPlan, String> {
        self.take_confirmed_plan_at(request, audit::unix_time_ms())
    }

    fn record_scan_at(
        &self,
        rule_version: &str,
        snapshots: Vec<CleanupSnapshot>,
        now_ms: u64,
    ) -> Result<CleanupScanResponse, String> {
        let scan_id = Uuid::new_v4();
        let expires_at_ms = now_ms.saturating_add(self.config.scan_ttl_ms);
        let mut indexed = HashMap::with_capacity(snapshots.len());
        let mut items = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            let item = snapshot.item().clone();
            if indexed.insert(item.id.clone(), snapshot).is_some() {
                return Err(format!("扫描结果包含重复规则条目: {}", item.id));
            }
            items.push(item);
        }

        let mut inner = self.inner.lock();
        purge_expired(&mut inner, now_ms);
        evict_oldest_scans(&mut inner, self.config.max_scan_sessions);
        inner.scans.insert(
            scan_id,
            ScanSession {
                scan_id,
                rule_version: rule_version.to_string(),
                created_at_ms: now_ms,
                expires_at_ms,
                snapshots: indexed,
            },
        );
        Ok(CleanupScanResponse {
            scan_id: scan_id.to_string(),
            rule_version: rule_version.to_string(),
            expires_at_ms,
            items,
        })
    }

    fn create_plan_at(
        &self,
        request: CreateCleanupPlanRequest,
        now_ms: u64,
    ) -> Result<CleanupPlanPreview, String> {
        validate_requested_item_ids(&request.item_ids)?;
        let scan_id = parse_id(&request.scan_id, "扫描会话 ID")?;
        let mut inner = self.inner.lock();
        purge_expired(&mut inner, now_ms);
        let session = inner
            .scans
            .get(&scan_id)
            .ok_or_else(|| "扫描会话不存在或已过期，请重新扫描".to_string())?;

        let mut snapshots = Vec::with_capacity(request.item_ids.len());
        let mut items = Vec::with_capacity(request.item_ids.len());
        let mut irreversible_item_ids = Vec::new();
        let mut total_files = 0usize;
        let mut total_bytes = 0u64;
        let mut quarantine_files = 0usize;
        let mut quarantine_bytes = 0u64;
        for item_id in &request.item_ids {
            let snapshot = session
                .snapshots
                .get(item_id)
                .ok_or_else(|| format!("条目不属于指定扫描会话: {item_id}"))?;
            let item = snapshot.item();
            if item.blocked_reason.is_some() {
                return Err(format!("条目当前不可执行: {item_id}"));
            }
            if matches!(item.risk, RiskLevel::High)
                && matches!(item.delete_mode, DeleteMode::Permanent)
            {
                irreversible_item_ids.push(item_id.clone());
            }
            if matches!(item.delete_mode, DeleteMode::Quarantine) {
                quarantine_files = quarantine_files.saturating_add(item.file_count);
                quarantine_bytes = quarantine_bytes.saturating_add(item.size_bytes);
            }
            total_files = total_files.saturating_add(item.file_count);
            total_bytes = total_bytes.saturating_add(item.size_bytes);
            items.push(item.clone());
            snapshots.push(snapshot.clone());
        }
        if quarantine_files > MAX_QUARANTINE_PLAN_FILES
            || quarantine_bytes > MAX_QUARANTINE_PLAN_BYTES
        {
            return Err(format!(
                "实验性隔离计划最多包含 {MAX_QUARANTINE_PLAN_FILES} 个文件、总计 1 GiB"
            ));
        }

        let plan_id = Uuid::new_v4();
        let preview = CleanupPlanPreview {
            plan_id: plan_id.to_string(),
            scan_id: session.scan_id.to_string(),
            rule_version: session.rule_version.clone(),
            created_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(self.config.plan_ttl_ms),
            total_items: items.len(),
            total_files,
            total_bytes,
            items,
            irreversible_item_ids,
        };
        let plan = CleanupPlan {
            preview: preview.clone(),
            snapshots,
        };
        evict_oldest_plans(&mut inner, self.config.max_plans);
        inner.plans.insert(plan_id, plan);
        Ok(preview)
    }

    fn take_confirmed_plan_at(
        &self,
        request: ExecuteCleanupPlanRequest,
        now_ms: u64,
    ) -> Result<CleanupPlan, String> {
        if !request.confirmed {
            return Err("必须确认清理操作".into());
        }
        let plan_id = parse_id(&request.plan_id, "清理计划 ID")?;
        let mut inner = self.inner.lock();
        purge_expired(&mut inner, now_ms);
        let plan = inner
            .plans
            .get(&plan_id)
            .ok_or_else(|| "清理计划不存在、已过期或已经执行".to_string())?;
        validate_irreversible_confirmations(
            &plan.preview.items,
            &plan.preview.irreversible_item_ids,
            &request.confirmed_irreversible_item_ids,
        )?;
        inner
            .plans
            .remove(&plan_id)
            .ok_or_else(|| "清理计划已被其他执行请求获取".to_string())
    }
}

fn parse_id(value: &str, label: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("{label} 格式无效"))
}

fn validate_requested_item_ids(item_ids: &[String]) -> Result<(), String> {
    if item_ids.is_empty() || item_ids.len() > MAX_PLAN_ITEMS {
        return Err(format!("清理计划必须包含 1 到 {MAX_PLAN_ITEMS} 个规则条目"));
    }
    if item_ids.iter().collect::<HashSet<_>>().len() != item_ids.len() {
        return Err("清理计划包含重复条目".into());
    }
    Ok(())
}

fn validate_irreversible_confirmations(
    items: &[crate::models::CleanupItem],
    irreversible_item_ids: &[String],
    confirmed_irreversible_item_ids: &[String],
) -> Result<(), String> {
    let planned = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    let irreversible = irreversible_item_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let confirmed = confirmed_irreversible_item_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if confirmed.len() != confirmed_irreversible_item_ids.len()
        || confirmed.iter().any(|id| !planned.contains(id))
        || confirmed.iter().any(|id| !irreversible.contains(id))
    {
        return Err("不可恢复内容确认列表无效".into());
    }
    if irreversible.iter().any(|id| !confirmed.contains(id)) {
        return Err("清理计划包含未明确确认的不可恢复内容".into());
    }
    Ok(())
}

fn purge_expired(inner: &mut StoreInner, now_ms: u64) {
    inner
        .scans
        .retain(|_, session| session.expires_at_ms > now_ms);
    inner
        .plans
        .retain(|_, plan| plan.preview.expires_at_ms > now_ms);
}

fn evict_oldest_scans(inner: &mut StoreInner, limit: usize) {
    while inner.scans.len() >= limit.max(1) {
        let Some(oldest) = inner
            .scans
            .iter()
            .min_by_key(|(_, session)| session.created_at_ms)
            .map(|(id, _)| *id)
        else {
            break;
        };
        inner.scans.remove(&oldest);
    }
}

fn evict_oldest_plans(inner: &mut StoreInner, limit: usize) {
    while inner.plans.len() >= limit.max(1) {
        let Some(oldest) = inner
            .plans
            .iter()
            .min_by_key(|(_, plan)| plan.preview.created_at_ms)
            .map(|(id, _)| *id)
        else {
            break;
        };
        inner.plans.remove(&oldest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RiskLevel;
    use std::sync::{Arc, Barrier};

    fn store(scan_ttl_ms: u64, plan_ttl_ms: u64) -> CleanupPlanStore {
        CleanupPlanStore {
            config: StoreConfig {
                scan_ttl_ms,
                plan_ttl_ms,
                max_scan_sessions: 4,
                max_plans: 8,
            },
            inner: Mutex::new(StoreInner::default()),
        }
    }

    fn snapshot(id: &str, marker: &str, risk: RiskLevel) -> CleanupSnapshot {
        CleanupSnapshot::test_snapshot(id, marker, risk)
    }

    fn create_request(scan_id: &str, item_ids: &[&str]) -> CreateCleanupPlanRequest {
        CreateCleanupPlanRequest {
            scan_id: scan_id.to_string(),
            item_ids: item_ids.iter().map(|id| (*id).to_string()).collect(),
        }
    }

    fn execute_request(plan_id: &str, irreversible: &[&str]) -> ExecuteCleanupPlanRequest {
        ExecuteCleanupPlanRequest {
            plan_id: plan_id.to_string(),
            confirmed: true,
            confirmed_irreversible_item_ids: irreversible
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
        }
    }

    #[test]
    fn second_scan_does_not_replace_snapshots_bound_to_first_scan() {
        let store = store(1_000, 1_000);
        let first = store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("cache", "first-scan", RiskLevel::Low)],
                100,
            )
            .expect("first scan should be stored");
        store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("cache", "second-scan", RiskLevel::Low)],
                101,
            )
            .expect("second scan should be independent");
        let preview = store
            .create_plan_at(create_request(&first.scan_id, &["cache"]), 102)
            .expect("first scan should still create a plan");
        assert_eq!(preview.items[0].path, "first-scan");
    }

    #[test]
    fn plan_take_is_atomic_and_one_shot() {
        let store = Arc::new(store(1_000, 1_000));
        let scan = store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("cache", "scan", RiskLevel::Low)],
                100,
            )
            .expect("scan should be stored");
        let plan = store
            .create_plan_at(create_request(&scan.scan_id, &["cache"]), 101)
            .expect("plan should be created");
        let barrier = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                let request = execute_request(&plan.plan_id, &[]);
                std::thread::spawn(move || {
                    barrier.wait();
                    store.take_confirmed_plan_at(request, 102).is_ok()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let successes = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker should finish"))
            .filter(|succeeded| *succeeded)
            .count();
        assert_eq!(successes, 1);
    }

    #[test]
    fn duplicate_unknown_and_expired_requests_are_rejected() {
        let store = store(5, 5);
        let scan = store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("cache", "scan", RiskLevel::Low)],
                100,
            )
            .expect("scan should be stored");
        assert!(store
            .create_plan_at(create_request(&scan.scan_id, &["cache", "cache"]), 101)
            .expect_err("duplicates must fail")
            .contains("重复"));
        assert!(store
            .create_plan_at(create_request(&scan.scan_id, &["unknown"]), 101)
            .expect_err("unknown item must fail")
            .contains("不属于"));
        assert!(store
            .create_plan_at(create_request(&scan.scan_id, &["cache"]), 105)
            .expect_err("expired scan must fail")
            .contains("已过期"));

        let fresh = store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("cache", "fresh", RiskLevel::Low)],
                200,
            )
            .expect("fresh scan should be stored");
        let plan = store
            .create_plan_at(create_request(&fresh.scan_id, &["cache"]), 201)
            .expect("fresh plan should be created");
        assert!(store
            .take_confirmed_plan_at(execute_request(&plan.plan_id, &[]), 206)
            .expect_err("expired plan must fail")
            .contains("已过期"));
    }

    #[test]
    fn invalid_confirmation_does_not_consume_plan() {
        let store = store(1_000, 1_000);
        let scan = store
            .record_scan_at(
                "rules-v1",
                vec![snapshot("user-data", "scan", RiskLevel::High)],
                100,
            )
            .expect("scan should be stored");
        let plan = store
            .create_plan_at(create_request(&scan.scan_id, &["user-data"]), 101)
            .expect("plan should be created");
        assert!(store
            .take_confirmed_plan_at(execute_request(&plan.plan_id, &[]), 102)
            .expect_err("missing confirmation must fail")
            .contains("未明确确认"));
        assert!(store
            .take_confirmed_plan_at(execute_request(&plan.plan_id, &["user-data"]), 103)
            .is_ok());
    }

    #[test]
    fn quarantined_high_risk_item_is_not_marked_irreversible() {
        let store = store(1_000, 1_000);
        let snapshot = CleanupSnapshot::test_snapshot_with_mode(
            "recoverable",
            "scan",
            RiskLevel::High,
            DeleteMode::Quarantine,
            1,
            128,
        );
        let scan = store
            .record_scan_at("rules-v1", vec![snapshot], 100)
            .expect("scan should be stored");
        let plan = store
            .create_plan_at(create_request(&scan.scan_id, &["recoverable"]), 101)
            .expect("quarantine plan should be created");

        assert!(plan.irreversible_item_ids.is_empty());
        assert!(store
            .take_confirmed_plan_at(execute_request(&plan.plan_id, &[]), 102)
            .is_ok());
    }

    #[test]
    fn quarantine_plan_limits_are_enforced_before_plan_storage() {
        let store = store(1_000, 1_000);
        let too_many = CleanupSnapshot::test_snapshot_with_mode(
            "too-many",
            "scan",
            RiskLevel::Low,
            DeleteMode::Quarantine,
            MAX_QUARANTINE_PLAN_FILES + 1,
            1,
        );
        let too_large = CleanupSnapshot::test_snapshot_with_mode(
            "too-large",
            "scan",
            RiskLevel::Low,
            DeleteMode::Quarantine,
            1,
            MAX_QUARANTINE_PLAN_BYTES + 1,
        );
        let scan = store
            .record_scan_at("rules-v1", vec![too_many, too_large], 100)
            .expect("scan should be stored");

        for id in ["too-many", "too-large"] {
            let error = store
                .create_plan_at(create_request(&scan.scan_id, &[id]), 101)
                .expect_err("out-of-bounds quarantine plan must fail");
            assert!(error.contains("最多包含"));
        }
    }
}
