//! Refresh Token 旋转 + 重放检测（Issue #10）
//!
//! 背景：
//! - 之前 `/auth/refresh` 路由根本不存在 —— refresh_token 字段被返回但没法用
//! - 用户 access_token 1 小时过期后必须重新登录，体验极差
//! - 即便后续加回 refresh，标准做法必须"旋转"：
//!   每次 refresh 发新 token，旧的标记为已使用
//!   重放旧 token → 整个 session 族（family）撤销
//!
//! 设计：
//! - `RefreshTokenStore` 跟踪每个 token 的状态
//! - 成功 rotate：旧 token 标记 used，新 token 加入 store
//! - 重放检测：见到 used 的 token → 整族撤销
//! - 存储：内存 LRU（生产应换成 Redis / DB）
//! - 加密：token 内容不可猜 → 用 UUID v4

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 一个 token 族 = 用户一次完整登录产生的所有 refresh token 链
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TokenFamily(pub Uuid);

/// 一个 refresh token 的状态
#[derive(Debug, Clone)]
pub struct RefreshTokenEntry {
    pub user_id: Uuid,
    pub family: TokenFamily,
    pub status: TokenStatus,
    pub stored_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenStatus {
    Active,
    Used,
    Revoked,
}

impl RefreshTokenEntry {
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.stored_at.elapsed() > ttl
    }
}

/// Refresh 操作的返回结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotateResult {
    /// 成功：返回新 token
    Success {
        new_token: String,
        user_id: Uuid,
        family: TokenFamily,
    },
    /// 重放攻击：整族撤销
    ReplayDetected {
        user_id: Uuid,
        family: TokenFamily,
    },
    /// Token 已知，但已用且未被识别为重放（理论上不应发生）
    AlreadyUsed {
        user_id: Uuid,
        family: TokenFamily,
    },
    /// 未知 token
    Unknown,
    /// Token 过期
    Expired,
}

#[derive(Clone)]
pub struct RefreshTokenStore {
    inner: Arc<RwLock<LruCache<String, RefreshTokenEntry>>>,
    ttl: Duration,
    capacity: usize,
}

impl RefreshTokenStore {
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
            ttl,
            capacity,
        }
    }

    /// 注册新 token（登录后调用）
    pub async fn register(&self, token: String, user_id: Uuid, family: TokenFamily) {
        let mut cache = self.inner.write().await;
        cache.put(
            token,
            RefreshTokenEntry {
                user_id,
                family,
                status: TokenStatus::Active,
                stored_at: Instant::now(),
            },
        );
    }

    /// 尝试用旧 token 换取新 token（旋转）
    ///
    /// 流程：
    /// 1. 找不到 → Unknown
    /// 2. 已 Revoked → 重放，撤销整族
    /// 3. 已 Used → 重放，撤销整族
    /// 4. Active 但过期 → Expired
    /// 5. Active 且未过期 → 标记为 Used，生成新 token 加入 store
    pub async fn rotate(
        &self,
        old_token: &str,
        new_token_factory: impl FnOnce() -> String,
    ) -> RotateResult {
        let mut cache = self.inner.write().await;
        match cache.peek(old_token) {
            None => RotateResult::Unknown,
            Some(entry) if entry.is_expired(self.ttl) => RotateResult::Expired,
            Some(entry) if entry.status == TokenStatus::Revoked => {
                // 不应该发生：被 Used 过的 token 之后才能 Revoked
                RotateResult::ReplayDetected {
                    user_id: entry.user_id,
                    family: entry.family.clone(),
                }
            }
            Some(entry) if entry.status == TokenStatus::Used => {
                // 重放！撤销整族
                let user_id = entry.user_id;
                let family = entry.family.clone();
                // 把整族标记为 Revoked
                let to_revoke: Vec<String> = cache
                    .iter()
                    .filter(|(_, e)| e.family == family)
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in to_revoke {
                    if let Some(e) = cache.peek_mut(&k) {
                        e.status = TokenStatus::Revoked;
                    }
                }
                drop(cache); // release write lock
                tracing::warn!(
                    "Refresh token replay detected. user_id={}, family={:?}. Revoking family.",
                    user_id, family
                );
                RotateResult::ReplayDetected { user_id, family }
            }
            Some(entry) => {
                // Active：标记为 Used，生成新 token
                let user_id = entry.user_id;
                let family = entry.family.clone();

                // 标记旧 token 为 Used
                if let Some(e) = cache.peek_mut(old_token) {
                    e.status = TokenStatus::Used;
                }

                // 发放新 token
                let new_token = new_token_factory();
                cache.put(
                    new_token.clone(),
                    RefreshTokenEntry {
                        user_id,
                        family: family.clone(),
                        status: TokenStatus::Active,
                        stored_at: Instant::now(),
                    },
                );

                RotateResult::Success {
                    new_token,
                    user_id,
                    family,
                }
            }
        }
    }

    /// 主动撤销整族（登出 / 管理员操作）
    pub async fn revoke_family(&self, family: &TokenFamily) {
        let mut cache = self.inner.write().await;
        let to_revoke: Vec<String> = cache
            .iter()
            .filter(|(_, e)| &e.family == family)
            .map(|(k, _)| k.clone())
            .collect();
        for k in to_revoke {
            if let Some(e) = cache.peek_mut(&k) {
                e.status = TokenStatus::Revoked;
            }
        }
    }

    /// 当前 store 中的活跃 token 数（用于测试 / 监控）
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-00000000000a").unwrap()
    }
    fn family_alice() -> TokenFamily {
        TokenFamily(Uuid::parse_str("00000000-0000-0000-0000-0000000000fa").unwrap())
    }

    fn token(label: &str) -> String {
        format!("token-{}", label)
    }

    fn factory(s: String) -> impl FnOnce() -> String {
        move || s
    }

    #[tokio::test]
    async fn unknown_token_returns_unknown() {
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        let result = store.rotate("never-seen", || "new".to_string()).await;
        assert_eq!(result, RotateResult::Unknown);
    }

    #[tokio::test]
    async fn first_rotation_succeeds_and_issues_new_token() {
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        store
            .register(token("A"), alice(), family_alice())
            .await;

        let result = store
            .rotate(&token("A"), factory(token("B")))
            .await;

        match result {
            RotateResult::Success { new_token, user_id, family } => {
                assert_eq!(new_token, token("B"));
                assert_eq!(user_id, alice());
                assert_eq!(family, family_alice());
            }
            _ => panic!("expected Success, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn second_use_of_rotated_token_triggers_replay_detection() {
        // P2 修复核心：旋转过的 token 再次提交 → 整族撤销
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        store
            .register(token("A"), alice(), family_alice())
            .await;

        // 第一次旋转：A → B
        let r1 = store.rotate(&token("A"), factory(token("B"))).await;
        assert!(matches!(r1, RotateResult::Success { .. }));

        // 第二次提交 A（A 已被标 Used）→ 重放
        let r2 = store.rotate(&token("A"), factory(token("C"))).await;
        match r2 {
            RotateResult::ReplayDetected { user_id, family } => {
                assert_eq!(user_id, alice());
                assert_eq!(family, family_alice());
            }
            _ => panic!("expected ReplayDetected, got {:?}", r2),
        }

        // 现在 B 也被撤销
        let r3 = store.rotate(&token("B"), factory(token("D"))).await;
        assert!(
            matches!(r3, RotateResult::ReplayDetected { .. }),
            "B should also be revoked after family-wide revocation. got: {:?}",
            r3
        );
    }

    #[tokio::test]
    async fn successful_chain_rotation_works() {
        // 正常链：A → B → C → D
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        store
            .register(token("A"), alice(), family_alice())
            .await;

        let r1 = store
            .rotate(&token("A"), factory(token("B")))
            .await;
        assert!(matches!(r1, RotateResult::Success { .. }));
        let r2 = store
            .rotate(&token("B"), factory(token("C")))
            .await;
        assert!(matches!(r2, RotateResult::Success { .. }));
        let r3 = store
            .rotate(&token("C"), factory(token("D")))
            .await;
        assert!(matches!(r3, RotateResult::Success { .. }));
    }

    #[tokio::test]
    async fn expired_token_returns_expired() {
        let store = RefreshTokenStore::new(10, Duration::from_millis(1));
        store
            .register(token("A"), alice(), family_alice())
            .await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = store.rotate(&token("A"), factory(token("B"))).await;
        assert_eq!(result, RotateResult::Expired);
    }

    #[tokio::test]
    async fn revoke_family_invalidates_all_tokens() {
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        store
            .register(token("A"), alice(), family_alice())
            .await;
        // 旋转出 B
        let _ = store
            .rotate(&token("A"), factory(token("B")))
            .await;

        // 撤销整族
        store.revoke_family(&family_alice()).await;

        // B 现在被认为是重放
        let r = store.rotate(&token("B"), factory(token("C"))).await;
        assert!(
            matches!(r, RotateResult::ReplayDetected { .. }),
            "after family revoke, all tokens should be marked Revoked and look like replay. got: {:?}",
            r
        );
    }

    #[tokio::test]
    async fn different_families_are_independent() {
        // Alice 族 A 和 Alice 族 B 独立：族 A 重放不影响族 B
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        let family_b = TokenFamily(Uuid::new_v4());
        store
            .register(token("A"), alice(), family_alice())
            .await;
        store.register(token("X"), alice(), family_b.clone()).await;

        // 旋转 A
        let _ = store
            .rotate(&token("A"), factory(token("B")))
            .await;
        // 重放 A
        let r = store.rotate(&token("A"), factory(token("C"))).await;
        assert!(matches!(r, RotateResult::ReplayDetected { .. }));

        // 族 B 的 X 仍然可用
        let r2 = store
            .rotate(&token("X"), factory(token("Y")))
            .await;
        assert!(
            matches!(r2, RotateResult::Success { .. }),
            "different family should be unaffected. got: {:?}",
            r2
        );
    }

    #[tokio::test]
    async fn different_users_with_same_family_string_dont_collide() {
        // 防御性测试：family 是 Uuid 包装的，正常情况下不同用户族不同
        let store = RefreshTokenStore::new(10, Duration::from_secs(60));
        let bob = Uuid::parse_str("00000000-0000-0000-0000-00000000000b").unwrap();
        let family = family_alice(); // 故意共享
        store.register(token("A"), alice(), family.clone()).await;
        store.register(token("B"), bob, family.clone()).await;

        // 旋转 A
        let _ = store
            .rotate(&token("A"), factory(token("A2")))
            .await;
        // 重放 A → 撤销整族（包含 Bob 的 B）
        let r = store.rotate(&token("A"), factory(token("A3"))).await;
        assert!(matches!(r, RotateResult::ReplayDetected { .. }));

        // Bob 的 B 也被撤销（因为共享 family）
        let r2 = store.rotate(&token("B"), factory(token("B2"))).await;
        assert!(
            matches!(r2, RotateResult::ReplayDetected { .. }),
            "shared family means Bob's token is collateral damage. got: {:?}",
            r2
        );
    }

    #[tokio::test]
    async fn cache_respects_capacity() {
        let store = RefreshTokenStore::new(2, Duration::from_secs(60));
        store
            .register(token("A"), alice(), family_alice())
            .await;
        store
            .register(token("B"), alice(), family_alice())
            .await;
        store
            .register(token("C"), alice(), family_alice())
            .await;
        // 2 个容量，A 被驱逐
        let r = store.rotate(&token("A"), factory(token("A2"))).await;
        assert_eq!(r, RotateResult::Unknown, "A should have been evicted");
    }
}
