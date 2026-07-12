use dioxus::prelude::*;
use crate::infra::errors::YntraError;

// Mock representation of get_workspace_details function
async fn get_workspace_details(_id: String) -> Result<WorkspaceData, YntraError> {
    Ok(WorkspaceData { name: "Sample Workspace".to_string() })
}

pub struct WorkspaceData {
    pub name: String,
}

// Example of resource data fetching component in Dioxus 0.7+
#[component]
pub fn WorkspaceViewer(workspace_id: String) -> Element {
    // Fetches the workspace data reactively when workspace_id changes
    let workspace = use_resource(move || {
        let id = workspace_id.clone();
        async move {
            get_workspace_details(id).await
        }
    });

    rsx! {
        div { class: "workspace-panel",
            match &*workspace.read() {
                Some(Ok(data)) => rsx! {
                    h1 { class: "text-2xl font-bold", "{data.name}" }
                },
                Some(Err(err)) => rsx! {
                    p { class: "text-red-500", "Failed to load workspace: {err}" }
                },
                None => rsx! {
                    div { class: "animate-pulse h-6 w-48 bg-gray-200 rounded" }
                }
            }
        }
    }
}
