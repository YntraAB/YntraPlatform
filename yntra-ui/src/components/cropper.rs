use crate::components;
use dioxus::prelude::*;

#[derive(Props, Clone)]
pub struct ImageCropperDialogProps {
    pub open: bool,
    pub image_data: String,
    pub onclose: EventHandler<()>,
    pub oncrop: EventHandler<String>,
}

impl PartialEq for ImageCropperDialogProps {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[component]
pub fn ImageCropperDialog(props: ImageCropperDialogProps) -> Element {
    let open = props.open;
    let image_data = props.image_data.clone();
    let preview_image_data = image_data.clone();
    let onclose = props.onclose;
    let oncrop = props.oncrop;

    let mut zoom = use_signal(|| 1.0);
    let mut pan_x = use_signal(|| 0);
    let mut pan_y = use_signal(|| 0);
    let mut is_saving = use_signal(|| false);

    let z = *zoom.read();
    let px = *pan_x.read();
    let py = *pan_y.read();
    let saving_val = *is_saving.read();

    let zoom_pct = (z * 100.0) as i32;

    let handle_save = move |_| {
        let img_data = image_data.clone();
        let cur_z = z;
        let cur_px = px;
        let cur_py = py;

        spawn(async move {
            is_saving.set(true);
            let mut js = document::eval(
                r#"
                const imgData = await dioxus.recv();
                const zoom = await dioxus.recv();
                const x = await dioxus.recv();
                const y = await dioxus.recv();
                
                const img = new Image();
                img.src = imgData;
                
                await new Promise((resolve, reject) => {
                    img.onload = resolve;
                    img.onerror = reject;
                });
                
                const canvas = document.createElement('canvas');
                canvas.width = 200;
                canvas.height = 200;
                const ctx = canvas.getContext('2d');
                
                // Set background to transparent or dark grey
                ctx.fillStyle = 'hsl(240, 10%, 3.9%)';
                ctx.fillRect(0, 0, 200, 200);
                
                const w = img.width;
                const h = img.height;
                const ratio = Math.min(200 / w, 200 / h);
                const drawW = w * ratio;
                const drawH = h * ratio;
                
                const zoomedW = drawW * zoom;
                const zoomedH = drawH * zoom;
                
                const drawX = 100 - (zoomedW / 2) + x;
                const drawY = 100 - (zoomedH / 2) + y;
                
                ctx.drawImage(img, drawX, drawY, zoomedW, zoomedH);
                
                const croppedDataUrl = canvas.toDataURL('image/png');
                dioxus.send(croppedDataUrl);
            "#,
            );

            js.send(serde_json::Value::String(img_data)).unwrap();
            js.send(serde_json::Value::Number(
                serde_json::Number::from_f64(cur_z).unwrap(),
            ))
            .unwrap();
            js.send(serde_json::Value::Number(serde_json::Number::from(cur_px)))
                .unwrap();
            js.send(serde_json::Value::Number(serde_json::Number::from(cur_py)))
                .unwrap();

            if let Ok(cropped_url) = js.recv::<String>().await {
                oncrop.call(cropped_url);
            }
            is_saving.set(false);
            onclose.call(());
        });
    };

    let handle_reset = move |_| {
        zoom.set(1.0);
        pan_x.set(0);
        pan_y.set(0);
    };

    rsx! {
        components::Dialog {
            open,
            title: "Crop Organization Logo".to_string(),
            onclose: move |_| onclose.call(()),
            div { class: "flex flex-col gap-5 w-full text-sm",
                    style: "max-width:400px;",
                p { class: "m-0 text-muted-foreground text-xs",
                    style: "line-height:1.4;",
                    "Adjust the image zoom and position using the sliders below to fit the logo inside the circular boundary."
                }

                // Visual Preview container
                div { class: "rounded-full",
                    style: "width: 200px; height: 200px; border: 2px dashed var(--accent); overflow: hidden; position: relative; margin: 1rem auto; background: hsl(240, 10%, 3.9%); box-shadow: 0 4px 20px rgba(0,0,0,0.4);",
                    img {
                        src: "{preview_image_data}",
                        style: "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%) translate({px}px, {py}px) scale({z}); max-width: 100%; max-height: 100%; object-fit: contain; transform-origin: center;",
                    }
                }

                // Controls
                div { class: "flex flex-col gap-3",
                    // Zoom Slider
                    div {
                        div { class: "flex justify-between text-xs font-bold text-muted-foreground",
                    style: "margin-bottom:0.2rem;",
                            span { "Zoom" }
                            span { "{zoom_pct}%" }
                        }
                        input {
                            r#type: "range",
                            class: "yntra-range-input w-full cursor-pointer",
                            style: "accent-color:var(--accent-color);",
                            min: "1.0",
                            max: "3.0",
                            step: "0.05",
                            value: "{z}",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<f64>() {
                                    zoom.set(val);
                                }
                            }
                        }
                    }

                    // Horizontal Pan Slider
                    div {
                        div { class: "flex justify-between text-xs font-bold text-muted-foreground",
                    style: "margin-bottom:0.2rem;",
                            span { "Horizontal Offset (X)" }
                            span { "{px}px" }
                        }
                        input {
                            r#type: "range",
                            class: "yntra-range-input w-full cursor-pointer",
                            style: "accent-color:var(--accent-color);",
                            min: "-150",
                            max: "150",
                            step: "1",
                            value: "{px}",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<i32>() {
                                    pan_x.set(val);
                                }
                            }
                        }
                    }

                    // Vertical Pan Slider
                    div {
                        div { class: "flex justify-between text-xs font-bold text-muted-foreground",
                    style: "margin-bottom:0.2rem;",
                            span { "Vertical Offset (Y)" }
                            span { "{py}px" }
                        }
                        input {
                            r#type: "range",
                            class: "yntra-range-input w-full cursor-pointer",
                            style: "accent-color:var(--accent-color);",
                            min: "-150",
                            max: "150",
                            step: "1",
                            value: "{py}",
                            oninput: move |e| {
                                if let Ok(val) = e.value().parse::<i32>() {
                                    pan_y.set(val);
                                }
                            }
                        }
                    }
                }

                // Footer Actions
                div { class: "grid gap-2 mt-2",
                    style: "grid-template-columns:1fr 120px;",
                    div { class: "flex gap-2",
                        button {
                            class: "yntra-btn btn-primary",
                            style: "padding:0.8rem 1.5rem;",
                            onclick: handle_save,
                            disabled: saving_val,
                            if saving_val { "Cropping..." } else { "Save Crop" }
                        }
                        button {
                            class: "yntra-btn btn-secondary",
                            style: "padding:0.8rem 1.5rem;",
                            onclick: move |_| onclose.call(()),
                            "Cancel"
                        }
                    }
                    button {
                        class: "yntra-btn secondary text-xs",
                        style: "padding:0.4rem 0.6rem;",
                        onclick: handle_reset,
                        "Reset Adjust"
                    }
                }
            }
        }
    }
}
