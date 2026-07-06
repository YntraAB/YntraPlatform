use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use std::future::Future;

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
                description: "You do not have the required permissions to perform this action.".to_string(),
            }
        }
        yntra_core::YntraError::DbError(detail) => {
            log::error!("Database error occurred: {}", detail);
            UserFriendlyError {
                title: "Database Error".to_string(),
                description: "Failed to read or write local data. Please reload the app.".to_string(),
            }
        }
        yntra_core::YntraError::NetworkError(detail) => {
            log::error!("Network error occurred: {}", detail);
            UserFriendlyError {
                title: "Network Connection Failed".to_string(),
                description: "Unable to reach remote servers. Please check your connection.".to_string(),
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
        other => {
            log::error!("Unexpected error occurred: {:?}", other);
            UserFriendlyError {
                title: "Unexpected Error".to_string(),
                description: "An unexpected error occurred. Please try again.".to_string(),
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
                    ToastOptions::new().description(user_err.description),
                );
            }
        });
    }
}

pub fn use_action_runner() -> ActionRunner {
    let toast = use_toast();
    ActionRunner { toast }
}
