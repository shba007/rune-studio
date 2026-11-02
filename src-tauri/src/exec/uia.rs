use crate::sync::LockExt;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg_attr(not(windows), allow(dead_code))]
pub const MAX_ELEMENTS: usize = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub id: String,
    pub role: &'static str,
    pub name: String,
    pub value: Option<String>,
    pub rect: (i32, i32, i32, i32),
}

impl Element {
    pub fn centre(&self) -> (i32, i32) {
        (
            (self.rect.0 + self.rect.2) / 2,
            (self.rect.1 + self.rect.3) / 2,
        )
    }

    pub fn line(&self) -> String {
        let name: String = self.name.chars().take(60).collect();
        match &self.value {
            Some(value) if !value.is_empty() => {
                format!("{} {} \"{name}\" = \"{value}\"", self.id, self.role)
            }
            _ => format!("{} {} \"{name}\"", self.id, self.role),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub window: Option<isize>,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundTarget {
    pub window: isize,
    pub element: Element,
}

static LATEST: std::sync::LazyLock<Mutex<HashMap<String, Snapshot>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn remember(scope: &str, mut snapshot: Snapshot) -> Snapshot {
    for element in &mut snapshot.elements {
        element.id = format!("e{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    }
    let mut latest = LATEST.locked();
    if scope.starts_with("bridge:") {
        latest.retain(|key, _| !key.starts_with("bridge:") || key == scope);
    }
    latest.insert(scope.to_string(), snapshot.clone());
    snapshot
}

pub fn lookup(scope: &str, id: &str) -> Option<BoundTarget> {
    let id = id.trim();
    LATEST
        .locked()
        .get(scope)
        .and_then(|snapshot| Some((snapshot.window?, &snapshot.elements)))
        .and_then(|(window, elements)| {
            elements
                .iter()
                .find(|element| element.id.eq_ignore_ascii_case(id))
                .cloned()
                .map(|element| BoundTarget { window, element })
        })
}

pub fn best_match(before: &Element, fresh: &[Element]) -> Option<Element> {
    let mut matches = fresh
        .iter()
        .filter(|element| element.role == before.role && element.name == before.name);
    let element = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(Element {
        id: before.id.clone(),
        ..element.clone()
    })
}

pub fn bind(scope: &str, id: &str) -> Result<BoundTarget, String> {
    let target = lookup(scope, id).ok_or_else(|| {
        format!("there is no element {id} in this session's latest list; observe again")
    })?;
    validate_target(&target, &capture())?;
    Ok(target)
}

pub fn validate_target(target: &BoundTarget, fresh: &Snapshot) -> Result<(), String> {
    if fresh.window != Some(target.window) {
        return Err("The active window changed; observe and choose again".into());
    }
    let current = best_match(&target.element, &fresh.elements)
        .ok_or("The control is missing or ambiguous; observe and choose again")?;
    if current.rect != target.element.rect {
        return Err("The control moved; observe and choose again".into());
    }
    Ok(())
}

pub fn forget(scope: &str) {
    LATEST.locked().remove(scope);
}

#[cfg(windows)]
pub fn current_window() -> Option<isize> {
    let window = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    (!window.is_invalid()).then_some(window.0 as isize)
}
#[cfg(not(windows))]
pub fn current_window() -> Option<isize> {
    None
}

pub fn capture() -> Snapshot {
    let before = current_window();
    let elements = snapshot();
    if before.is_none() || before != current_window() {
        return Snapshot::default();
    }
    Snapshot {
        window: before,
        elements,
    }
}

pub fn describe(elements: &[Element]) -> String {
    elements
        .iter()
        .map(Element::line)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg_attr(not(windows), allow(dead_code))]
fn role(control_type: i32) -> &'static str {
    match control_type {
        50000 => "button",
        50002 => "checkbox",
        50003 => "dropdown",
        50004 => "edit",
        50005 => "link",
        50007 | 50029 => "item",
        50011 => "menu item",
        50013 => "radio button",
        50019 => "tab",
        50024 => "tree item",
        50030 => "document",
        50031 => "split button",
        _ => "control",
    }
}

#[cfg(windows)]
pub fn snapshot() -> Vec<Element> {
    use std::time::{Duration, Instant};

    use windows::{
        Win32::{
            System::{
                Com::{
                    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
                },
                Variant::VARIANT,
            },
            UI::{
                Accessibility::{
                    CUIAutomation, IUIAutomation, IUIAutomationValuePattern, TreeScope_Descendants,
                    UIA_IsKeyboardFocusablePropertyId, UIA_IsOffscreenPropertyId,
                    UIA_ValuePatternId,
                },
                WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
            },
        },
        core::Interface,
    };

    const BUDGET: Duration = Duration::from_millis(1_500);
    const MIN_SIDE: i32 = 6;

    let started = Instant::now();
    let found = unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return Vec::new();
        }
        let mut process = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut process));
        if process == std::process::id() {
            return Vec::new();
        }
        let Ok(automation) =
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            return Vec::new();
        };
        let Ok(root) = automation.ElementFromHandle(window) else {
            return Vec::new();
        };
        let Ok(focusable) = automation
            .CreatePropertyCondition(UIA_IsKeyboardFocusablePropertyId, &VARIANT::from(true))
        else {
            return Vec::new();
        };
        let Ok(visible) =
            automation.CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))
        else {
            return Vec::new();
        };
        let Ok(condition) = automation.CreateAndCondition(&focusable, &visible) else {
            return Vec::new();
        };
        let Ok(all) = root.FindAll(TreeScope_Descendants, &condition) else {
            return Vec::new();
        };

        let mut elements = Vec::new();
        for index in 0..all.Length().unwrap_or(0) {
            if elements.len() >= MAX_ELEMENTS || started.elapsed() > BUDGET {
                break;
            }
            let Ok(element) = all.GetElement(index) else {
                continue;
            };
            let Ok(rect) = element.CurrentBoundingRectangle() else {
                continue;
            };
            if rect.right - rect.left < MIN_SIDE || rect.bottom - rect.top < MIN_SIDE {
                continue;
            }
            let kind = element.CurrentControlType().map(|kind| kind.0).unwrap_or(0);
            let name = element
                .CurrentName()
                .map(|name| name.to_string())
                .unwrap_or_default();
            let value = (kind == 50004).then(|| {
                element
                    .GetCurrentPattern(UIA_ValuePatternId)
                    .ok()
                    .and_then(|pattern| pattern.cast::<IUIAutomationValuePattern>().ok())
                    .and_then(|pattern| pattern.CurrentValue().ok())
                    .map(|value| value.to_string().chars().take(60).collect::<String>())
                    .unwrap_or_default()
            });
            if name.trim().is_empty() && value.as_deref().unwrap_or("").is_empty() {
                continue;
            }
            elements.push((
                role(kind),
                name,
                value,
                (rect.left, rect.top, rect.right, rect.bottom),
            ));
        }
        elements
    };

    let mut elements: Vec<Element> = found
        .into_iter()
        .enumerate()
        .map(|(index, (role, name, value, rect))| Element {
            id: format!("e{}", index + 1),
            role,
            name,
            value,
            rect,
        })
        .collect();
    elements.truncate(MAX_ELEMENTS);
    log::info!(
        "{} control(s) listed in {} ms",
        elements.len(),
        started.elapsed().as_millis()
    );
    elements
}

#[cfg(not(windows))]
pub fn snapshot() -> Vec<Element> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(id: &str, name: &str) -> Element {
        Element {
            id: id.into(),
            role: "button",
            name: name.into(),
            value: None,
            rect: (100, 200, 140, 220),
        }
    }

    #[test]
    fn the_click_point_is_the_middle_of_the_control() {
        assert_eq!(element("e1", "Save").centre(), (120, 210));
    }

    #[test]
    fn a_line_names_the_id_role_and_name_and_shows_what_a_field_holds() {
        assert_eq!(element("e3", "Save").line(), "e3 button \"Save\"");
        let field = Element {
            value: Some("hello".into()),
            role: "edit",
            ..element("e4", "Search")
        };
        assert_eq!(field.line(), "e4 edit \"Search\" = \"hello\"");
        let empty = Element {
            value: Some(String::new()),
            role: "edit",
            ..element("e5", "Search")
        };
        assert_eq!(empty.line(), "e5 edit \"Search\"");
    }

    #[test]
    fn a_control_is_found_again_where_it_is_now() {
        let before = Element {
            rect: (100, 100, 140, 120),
            ..element("e5", "Qwen/Qwen3.8-27B")
        };
        let moved = Element {
            id: "e9".into(),
            rect: (100, 300, 140, 320),
            ..before.clone()
        };
        let found = best_match(&before, &[element("e1", "Other"), moved]).unwrap();
        assert_eq!(found.id, "e5", "it keeps the id the model used");
        assert_eq!(found.rect, (100, 300, 140, 320), "but its new position");
    }

    #[test]
    fn ambiguous_and_missing_controls_are_rejected() {
        let before = element("e1", "Delete");
        assert!(best_match(&before, &[before.clone(), before.clone()]).is_none());
        assert!(best_match(&before, &[element("e4", "Save")]).is_none());
    }

    #[test]
    fn ids_are_scoped_and_expire_on_the_next_listing() {
        let snapshot = Snapshot {
            window: Some(42),
            elements: vec![element("e1", "Save")],
        };
        let first = remember("test:first", snapshot.clone());
        let id = &first.elements[0].id;
        assert_eq!(lookup("test:first", id).unwrap().element.name, "Save");
        assert!(lookup("test:other", id).is_none());
        let second = remember("test:first", snapshot);
        assert_ne!(id, &second.elements[0].id);
        assert!(lookup("test:first", id).is_none());
        forget("test:first");
        assert!(lookup("test:first", &second.elements[0].id).is_none());
    }

    #[test]
    fn target_requires_same_window_unique_control_and_position() {
        let element = element("e1", "Save");
        let target = BoundTarget {
            window: 42,
            element: element.clone(),
        };
        let mut fresh = Snapshot {
            window: Some(42),
            elements: vec![element.clone()],
        };
        assert!(validate_target(&target, &fresh).is_ok());
        fresh.window = Some(43);
        assert!(validate_target(&target, &fresh).is_err());
        fresh.window = Some(42);
        fresh.elements.push(element.clone());
        assert!(validate_target(&target, &fresh).is_err());
        fresh.elements.pop();
        fresh.elements[0].rect.0 += 1;
        assert!(validate_target(&target, &fresh).is_err());
        fresh.elements.clear();
        assert!(validate_target(&target, &fresh).is_err());
    }

    #[test]
    fn switching_or_closing_a_menu_invalidates_its_original_item() {
        let item = Element {
            role: "menu item",
            ..element("e1", "New tab")
        };
        let target = BoundTarget {
            window: 42,
            element: item.clone(),
        };
        let mut fresh = Snapshot {
            window: Some(42),
            elements: vec![item],
        };
        assert!(validate_target(&target, &fresh).is_ok());
        fresh.elements = vec![Element {
            role: "menu item",
            ..element("e2", "Undo")
        }];
        assert!(validate_target(&target, &fresh).is_err());
        fresh.elements.clear();
        assert!(validate_target(&target, &fresh).is_err());
    }
}
