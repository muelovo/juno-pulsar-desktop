pub mod policy;
mod recycle;
use crate::desktop::shell::Target;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
#[derive(Clone, Serialize)]
pub struct Proposal {
    pub token: String,
    pub target: Target,
    pub expires_seconds: u32,
}
struct Pending {
    proposal: Proposal,
    identity: policy::Identity,
    created: Instant,
}
struct Hover {
    path: String,
    since: Instant,
    last: Instant,
}
#[derive(Default)]
pub struct Operations {
    pending: Mutex<Option<Pending>>,
    hover: Mutex<Option<Hover>>,
}
#[derive(Serialize)]
pub struct Outcome {
    pub status: &'static str,
    pub code: String,
    pub recycled: bool,
}
impl Operations {
    pub fn observe(&self, target: Option<&Target>, down: bool) {
        if let Ok(mut h) = self.hover.lock() {
            if !down || target.is_none() {
                *h = None;
                return;
            }
            let path = &target.unwrap().path;
            let now = Instant::now();
            match h.as_mut() {
                Some(old)
                    if old.path == *path && old.last.elapsed() < Duration::from_millis(350) =>
                {
                    old.last = now
                }
                _ => {
                    *h = Some(Hover {
                        path: path.clone(),
                        since: now,
                        last: now,
                    })
                }
            }
        }
    }
    pub fn prepare(&self, target: Target) -> Result<Proposal, String> {
        let hover = self
            .hover
            .lock()
            .map_err(|_| "state_unavailable")?
            .take()
            .ok_or("hover_required")?;
        if hover.path != target.path
            || hover.since.elapsed() < Duration::from_millis(600)
            || hover.last.elapsed() > Duration::from_millis(350)
        {
            return Err("hover_not_stable".into());
        }
        let identity = policy::inspect(std::path::Path::new(&target.path))?;
        let proposal = Proposal {
            token: uuid::Uuid::new_v4().to_string(),
            target,
            expires_seconds: 30,
        };
        *self.pending.lock().map_err(|_| "state_unavailable")? = Some(Pending {
            proposal: proposal.clone(),
            identity,
            created: Instant::now(),
        });
        Ok(proposal)
    }
    pub fn current(&self) -> Option<Proposal> {
        self.pending
            .lock()
            .ok()?
            .as_ref()
            .filter(|p| p.created.elapsed() < Duration::from_secs(30))
            .map(|p| p.proposal.clone())
    }
    pub fn cancel(&self) {
        if let Ok(mut h) = self.hover.lock() {
            *h = None;
        }
        if let Ok(mut p) = self.pending.lock() {
            *p = None
        }
    }
    pub fn execute(&self, token: &str) -> Outcome {
        let pending = self.pending.lock().ok().and_then(|mut p| p.take());
        let result = (|| {
            let p = pending.ok_or("no_pending_confirmation".to_string())?;
            if p.proposal.token != token || p.created.elapsed() > Duration::from_secs(30) {
                return Err("expired_or_invalid_token".into());
            }
            let path = PathBuf::from(&p.proposal.target.path);
            if policy::inspect(&path)? != p.identity {
                return Err("identity_changed".into());
            }
            let done = std::thread::spawn(move || {
                recycle::recycle(path, p.identity).map_err(|e| format!("shell_{:?}", e.code()))
            })
            .join()
            .map_err(|_| "worker_failed")??;
            if !done {
                return Err("recycle_not_confirmed".into());
            }
            Ok(())
        })();
        match result {
            Ok(()) => Outcome {
                status: "completed",
                code: "recycled".into(),
                recycled: true,
            },
            Err(code) => {
                log::warn!("recycle result={code}");
                Outcome {
                    status: "rejected",
                    code,
                    recycled: false,
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_confirmation_no_delete() {
        let r = Operations::default().execute("fake");
        assert!(!r.recycled);
        assert_eq!(r.code, "no_pending_confirmation")
    }
}
#[cfg(test)]
mod token_tests {
    use super::*;
    fn proposal() -> Pending {
        Pending {
            proposal: Proposal {
                token: "one-time".into(),
                target: Target {
                    path: r"C:\not-a-target".into(),
                    name: "test".into(),
                    kind: "file".into(),
                },
                expires_seconds: 30,
            },
            identity: policy::Identity {
                volume: 0,
                high: 0,
                low: 0,
                created: 0,
            },
            created: Instant::now() - Duration::from_secs(31),
        }
    }
    #[test]
    fn expired_token_consumed() {
        let operations = Operations::default();
        *operations.pending.lock().unwrap() = Some(proposal());
        let r = operations.execute("one-time");
        assert_eq!(r.code, "expired_or_invalid_token");
        assert_eq!(
            operations.execute("one-time").code,
            "no_pending_confirmation"
        )
    }
    #[test]
    fn cancel_invalidates_pending() {
        let operations = Operations::default();
        *operations.pending.lock().unwrap() = Some(proposal());
        operations.cancel();
        assert!(operations.current().is_none())
    }
    #[test]
    fn prepare_requires_backend_hover() {
        let operations = Operations::default();
        assert_eq!(
            operations
                .prepare(proposal().proposal.target)
                .err()
                .unwrap(),
            "hover_required"
        )
    }
}
