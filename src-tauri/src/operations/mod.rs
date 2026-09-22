pub mod policy;
mod recycle;

use crate::desktop::shell::Target;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Default)]
pub struct Operations;

#[derive(Clone, Serialize)]
pub struct Outcome {
    pub status: &'static str,
    pub code: String,
    pub recycled: bool,
}

impl Operations {
    pub fn cancel(&self) {}

    pub fn recycle_target(&self, target: Target) -> Outcome {
        let result = (|| {
            let path = PathBuf::from(&target.path);
            let identity = policy::inspect(&path)?;
            let done = std::thread::spawn(move || {
                recycle::recycle(path, identity)
                    .map_err(|error| format!("shell_{:?}", error.code()))
            })
            .join()
            .map_err(|_| "worker_failed")??;
            done.then_some(())
                .ok_or("recycle_not_confirmed".to_string())
        })();
        outcome(result)
    }
}

fn outcome(result: Result<(), String>) -> Outcome {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_is_structured() {
        let result = outcome(Err("blocked".into()));
        assert!(!result.recycled);
        assert_eq!(result.status, "rejected");
        assert_eq!(result.code, "blocked");
    }
}
