use dioxus::prelude::*;

// Example of fine-grained counter component in Dioxus 0.7+
#[component]
pub fn Counter() -> Element {
    let mut count = use_signal(|| 0);
    
    rsx! {
        button {
            class: "btn-primary transition-all duration-200 active:scale-95",
            onclick: move |_| count.set(count.read() + 1),
            "Count: {count}"
        }
    }
}
