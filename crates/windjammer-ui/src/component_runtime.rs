//! Component runtime for direct DOM manipulation
//!
//! This module provides the runtime support for compiled Windjammer UI components.
//! Unlike traditional virtual DOM approaches, we generate direct DOM manipulation code
//! for maximum performance and transparency.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, Element, Event, HtmlElement, Window};

/// Get the browser window
#[cfg(target_arch = "wasm32")]
pub fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("No window found"))
}

/// Get the document
#[cfg(target_arch = "wasm32")]
pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or_else(|| JsValue::from_str("No document found"))
}

/// Create an element with a tag name
#[cfg(target_arch = "wasm32")]
pub fn create_element(tag: &str) -> Result<Element, JsValue> {
    document()?.create_element(tag)
}

/// Set text content on an element
#[cfg(target_arch = "wasm32")]
pub fn set_text(element: &Element, text: &str) {
    element.set_text_content(Some(text));
}

/// Set an attribute on an element
#[cfg(target_arch = "wasm32")]
pub fn set_attribute(element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
    element.set_attribute(name, value)
}

/// Add a class to an element
#[cfg(target_arch = "wasm32")]
pub fn add_class(element: &Element, class: &str) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.class_list().add_1(class)?;
    }
    Ok(())
}

/// Remove a class from an element
#[cfg(target_arch = "wasm32")]
pub fn remove_class(element: &Element, class: &str) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.class_list().remove_1(class)?;
    }
    Ok(())
}

/// Append a child element
#[cfg(target_arch = "wasm32")]
pub fn append_child(parent: &Element, child: &Element) -> Result<(), JsValue> {
    parent.append_child(child)?;
    Ok(())
}

/// Get element by ID
#[cfg(target_arch = "wasm32")]
pub fn get_element_by_id(id: &str) -> Result<Element, JsValue> {
    document()?
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("Element with id '{}' not found", id)))
}

/// Event handler type
#[cfg(target_arch = "wasm32")]
pub type EventCallback = Box<dyn FnMut(Event)>;

/// Helper to create an event listener closure
#[cfg(target_arch = "wasm32")]
pub fn create_event_listener<F>(mut handler: F) -> Closure<dyn FnMut(Event)>
where
    F: FnMut(Event) + 'static,
{
    Closure::wrap(Box::new(move |event: Event| {
        handler(event);
    }) as Box<dyn FnMut(Event)>)
}

/// Add an event listener to an element
#[cfg(target_arch = "wasm32")]
pub fn add_event_listener(
    element: &Element,
    event_type: &str,
    closure: &Closure<dyn FnMut(Event)>,
) -> Result<(), JsValue> {
    element.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
}

/// Remove an element from its parent
#[cfg(target_arch = "wasm32")]
pub fn remove_element(element: &Element) -> Result<(), JsValue> {
    if let Some(parent) = element.parent_element() {
        parent.remove_child(element)?;
    }
    Ok(())
}

/// Toggle a class on an element
#[cfg(target_arch = "wasm32")]
pub fn toggle_class(element: &Element, class: &str) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.class_list().toggle(class)?;
    }
    Ok(())
}

/// Check if element has a class
#[cfg(target_arch = "wasm32")]
pub fn has_class(element: &Element, class: &str) -> bool {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.class_list().contains(class)
    } else {
        false
    }
}

/// Set multiple classes at once
#[cfg(target_arch = "wasm32")]
pub fn set_classes(element: &Element, classes: &[&str]) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.set_class_name(&classes.join(" "));
    }
    Ok(())
}

/// Get attribute value
#[cfg(target_arch = "wasm32")]
pub fn get_attribute(element: &Element, name: &str) -> Option<String> {
    element.get_attribute(name)
}

/// Remove an attribute
#[cfg(target_arch = "wasm32")]
pub fn remove_attribute(element: &Element, name: &str) -> Result<(), JsValue> {
    element.remove_attribute(name)
}

/// Set inner HTML (use with caution)
#[cfg(target_arch = "wasm32")]
pub fn set_inner_html(element: &Element, html: &str) {
    element.set_inner_html(html);
}

/// Get inner HTML
#[cfg(target_arch = "wasm32")]
pub fn get_inner_html(element: &Element) -> String {
    element.inner_html()
}

/// Set style property
#[cfg(target_arch = "wasm32")]
pub fn set_style(element: &Element, property: &str, value: &str) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        // Get the style attribute and set it
        let current_style = html_element.get_attribute("style").unwrap_or_default();
        let new_style = if current_style.is_empty() {
            format!("{}: {};", property, value)
        } else {
            format!("{}; {}: {};", current_style, property, value)
        };
        html_element.set_attribute("style", &new_style)?;
    }
    Ok(())
}

/// Get style property
#[cfg(target_arch = "wasm32")]
pub fn get_style(element: &Element, property: &str) -> Result<String, JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        // Parse the style attribute
        if let Some(style_attr) = html_element.get_attribute("style") {
            for part in style_attr.split(';') {
                let parts: Vec<&str> = part.split(':').collect();
                if parts.len() == 2 && parts[0].trim() == property {
                    return Ok(parts[1].trim().to_string());
                }
            }
        }
    }
    Ok(String::new())
}

/// Focus an element
#[cfg(target_arch = "wasm32")]
pub fn focus(element: &Element) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.focus()?;
    }
    Ok(())
}

/// Blur an element
#[cfg(target_arch = "wasm32")]
pub fn blur(element: &Element) -> Result<(), JsValue> {
    if let Some(html_element) = element.dyn_ref::<HtmlElement>() {
        html_element.blur()?;
    }
    Ok(())
}

/// Query selector - find first matching element
#[cfg(target_arch = "wasm32")]
pub fn query_selector(selector: &str) -> Result<Option<Element>, JsValue> {
    document()?.query_selector(selector)
}

/// Query selector all - find all matching elements
#[cfg(target_arch = "wasm32")]
pub fn query_selector_all(selector: &str) -> Result<web_sys::NodeList, JsValue> {
    document()?.query_selector_all(selector)
}

/// Create a text node
#[cfg(target_arch = "wasm32")]
pub fn create_text_node(text: &str) -> Result<web_sys::Text, JsValue> {
    Ok(document()?.create_text_node(text))
}

/// Request animation frame
#[cfg(target_arch = "wasm32")]
pub fn request_animation_frame<F>(f: F) -> Result<i32, JsValue>
where
    F: FnMut() + 'static,
{
    let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
    let handle = window()?.request_animation_frame(closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(handle)
}

/// Set timeout
#[cfg(target_arch = "wasm32")]
pub fn set_timeout<F>(f: F, millis: i32) -> Result<i32, JsValue>
where
    F: FnMut() + 'static,
{
    let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
    let handle = window()?.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        millis,
    )?;
    closure.forget();
    Ok(handle)
}

/// Console log helper
#[cfg(target_arch = "wasm32")]
pub fn console_log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

/// Console error helper
#[cfg(target_arch = "wasm32")]
pub fn console_error(message: &str) {
    web_sys::console::error_1(&JsValue::from_str(message));
}

/// Console warn helper
#[cfg(target_arch = "wasm32")]
pub fn console_warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn window() -> Result<(), String> {
    Err("Window is only available on WASM target".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn document() -> Result<(), String> {
    Err("Document is only available on WASM target".to_string())
}
