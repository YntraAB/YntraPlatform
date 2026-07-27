use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use std::future::Future;
use std::time::Duration;

pub struct UserFriendlyError {
    pub title: String,
    pub description: String,
}

pub fn map_error(err: &yntra_core::YntraError) -> UserFriendlyError {
    match err {
        yntra_core::YntraError::AuthError(detail) => {
            log::error!("Authentication error occurred: {}", detail);
            UserFriendlyError {
                title: "Access Denied".to_string(),
                description: "You do not have the required permissions to perform this action."
                    .to_string(),
            }
        }
        yntra_core::YntraError::DbError(detail) => {
            log::error!("Database error occurred: {}", detail);
            UserFriendlyError {
                title: "Database Error".to_string(),
                description: "Failed to read or write local data. Please reload the app."
                    .to_string(),
            }
        }
        yntra_core::YntraError::NetworkError(detail) => {
            log::error!("Network error occurred: {}", detail);
            UserFriendlyError {
                title: "Network Connection Failed".to_string(),
                description: "Unable to reach remote servers. Please check your connection."
                    .to_string(),
            }
        }
        yntra_core::YntraError::ValidationError(detail) => {
            log::error!("Validation failed: {}", detail);
            UserFriendlyError {
                title: "Invalid Input".to_string(),
                description: detail.clone(),
            }
        }
        yntra_core::YntraError::NotFoundError(detail) => {
            log::error!("Resource not found: {}", detail);
            UserFriendlyError {
                title: "Not Found".to_string(),
                description: detail.clone(),
            }
        }
        yntra_core::YntraError::SyncError(detail) => {
            log::error!("Sync error: {}", detail);
            UserFriendlyError {
                title: "Sync Failed".to_string(),
                description:
                    "Failed to synchronize changes with the remote server. Please try again."
                        .to_string(),
            }
        }
        yntra_core::YntraError::ConstraintError(detail) => {
            log::error!("Database constraint violation: {}", detail);
            UserFriendlyError {
                title: "Data Constraint Violation".to_string(),
                description: "This operation violates database integrity constraints.".to_string(),
            }
        }
        yntra_core::YntraError::SerializationError(detail) => {
            log::error!("Serialization error: {}", detail);
            UserFriendlyError {
                title: "Data Error".to_string(),
                description: "Failed to encode or decode local storage payload.".to_string(),
            }
        }
        yntra_core::YntraError::InvitationError(detail) => {
            log::error!("Invitation error: {}", detail);
            UserFriendlyError {
                title: "Invitation Error".to_string(),
                description: detail.clone(),
            }
        }
        yntra_core::YntraError::CryptoError(detail) => {
            log::error!("Cryptographic/Security error: {}", detail);
            UserFriendlyError {
                title: "Security Violation".to_string(),
                description: "A cryptographic signature validation failed.".to_string(),
            }
        }
        yntra_core::YntraError::NoRowsReturned => {
            log::error!("Query returned no rows");
            UserFriendlyError {
                title: "Record Not Found".to_string(),
                description: "The requested record was not found in the local database."
                    .to_string(),
            }
        }
    }
}

#[derive(Clone)]
pub struct ActionRunner {
    toast: dioxus_primitives::toast::Toasts,
}

impl ActionRunner {
    pub fn run<F>(&self, fut: F)
    where
        F: Future<Output = Result<(), yntra_core::YntraError>> + 'static,
    {
        let toast = self.toast.clone();
        spawn(async move {
            if let Err(e) = fut.await {
                let user_err = map_error(&e);
                toast.error(
                    user_err.title,
                    ToastOptions::new()
                        .description(user_err.description)
                        .duration(Duration::from_secs(4)),
                );
            }
        });
    }

    pub fn run_with_success<F>(&self, fut: F, success_title: impl Into<String>, success_desc: impl Into<String>)
    where
        F: Future<Output = Result<(), yntra_core::YntraError>> + 'static,
    {
        let toast = self.toast.clone();
        let title = success_title.into();
        let desc = success_desc.into();
        spawn(async move {
            match fut.await {
                Ok(()) => {
                    toast.success(
                        title,
                        ToastOptions::new()
                            .description(desc)
                            .duration(Duration::from_secs(3)),
                    );
                }
                Err(e) => {
                    let user_err = map_error(&e);
                    toast.error(
                        user_err.title,
                        ToastOptions::new()
                            .description(user_err.description)
                            .duration(Duration::from_secs(4)),
                    );
                }
            }
        });
    }

    pub fn run_exclusive<F>(&self, task_signal: &mut Signal<Option<dioxus::core::Task>>, fut: F)
    where
        F: Future<Output = Result<(), yntra_core::YntraError>> + 'static,
    {
        if let Some(existing_task) = task_signal.read().as_ref() {
            existing_task.cancel();
        }

        let toast = self.toast.clone();
        let task = spawn(async move {
            if let Err(e) = fut.await {
                let user_err = map_error(&e);
                toast.error(
                    user_err.title,
                    ToastOptions::new()
                        .description(user_err.description)
                        .duration(Duration::from_secs(4)),
                );
            }
        });
        task_signal.set(Some(task));
    }
}

pub fn use_action_runner() -> ActionRunner {
    let toast = use_toast();
    ActionRunner { toast }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActionStatus<E> {
    Idle,
    Loading,
    Success,
    Error(E),
}

#[derive(Clone, Copy)]
pub struct UseAction<T: 'static, E: 'static> {
    pub status: Signal<ActionStatus<E>>,
    pub result: Signal<Option<T>>,
    pub trigger: Callback<()>,
}

impl<T: Clone + 'static, E: Clone + 'static> UseAction<T, E> {
    pub fn run(&self) {
        self.trigger.call(());
    }

    pub fn is_loading(&self) -> bool {
        matches!(*self.status.read(), ActionStatus::Loading)
    }

    pub fn error(&self) -> Option<E> {
        match &*self.status.read() {
            ActionStatus::Error(e) => Some(e.clone()),
            _ => None,
        }
    }
}

pub fn use_action<F, Fut, T, E>(action_fn: F) -> UseAction<T, E>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    T: Clone + 'static,
    E: Clone + 'static,
{
    let status = use_signal(|| ActionStatus::Idle);
    let result = use_signal(|| None);
    let current_task: Signal<Option<dioxus::core::Task>> = use_signal(|| None);

    let trigger = move || {
        let mut current_task_mut = current_task;
        if let Some(task) = current_task_mut.read().clone() {
            task.cancel();
        }

        let mut status_mut = status;
        status_mut.set(ActionStatus::Loading);
        let fut = action_fn();
        let mut status_clone = status;
        let mut result_clone = result;
        let task = spawn(async move {
            match fut.await {
                Ok(val) => {
                    result_clone.set(Some(val));
                    status_clone.set(ActionStatus::Success);
                }
                Err(err) => {
                    status_clone.set(ActionStatus::Error(err));
                }
            }
        });
        current_task_mut.set(Some(task));
    };

    UseAction {
        status,
        result,
        trigger: Callback::new(move |_| trigger()),
    }
}
