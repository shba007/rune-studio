use crate::sync::LockExt;
use std::sync::Mutex;

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

static LATEST: Mutex<Vec<Element>> = Mutex::new(Vec::new());

pub fn remember(elements: Vec<Element>) {
    *LATEST.locked() = elements;
}

pub fn lookup(id: &str) -> Option<Element> {
    let id = id.trim();
    LATEST
        .locked()
        .iter()
        .find(|element| element.id.eq_ignore_ascii_case(id))
        .cloned()
}

pub fn best_match(before: &Element, fresh: &[Element]) -> Option<Element> {
    let (cx, cy) = before.centre();
    fresh
        .iter()
        .filter(|element| element.role == before.role && element.name == before.name)
        .min_by_key(|element| {
            let (x, y) = element.centre();
            i64::from(x - cx).pow(2) + i64::from(y - cy).pow(2)
        })
        .map(|element| Element {
            id: before.id.clone(),
            ..element.clone()
        })
}

pub fn resolve(id: &str) -> Result<Element, String> {
    let before = lookup(id).ok_or_else(|| format!("there is no element {id} in the latest list; use an id from the current list or click a point"))?;
    best_match(&before, &snapshot()).ok_or_else(|| {
        format!(
            "the {} \"{}\" is no longer on the screen; look at the screen and choose again",
            before.role, before.name
        )
    })
}

pub fn forget() {
    LATEST.locked().clear();
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
    fn the_nearest_of_several_with_the_same_name_wins_and_a_missing_one_is_none() {
        let before = Element {
            rect: (0, 0, 20, 20),
            ..element("e1", "Delete")
        };
        let near = Element {
            id: "e2".into(),
            rect: (0, 40, 20, 60),
            ..before.clone()
        };
        let far = Element {
            id: "e3".into(),
            rect: (0, 400, 20, 420),
            ..before.clone()
        };
        assert_eq!(
            best_match(&before, &[far, near]).unwrap().rect,
            (0, 40, 20, 60)
        );
        assert!(best_match(&before, &[element("e4", "Save")]).is_none());
        let other_role = Element {
            role: "link",
            ..before.clone()
        };
        assert!(best_match(&before, &[other_role]).is_none());
    }

    #[test]
    fn ids_belong_to_the_latest_listing_only() {
        forget();
        assert!(lookup("e1").is_none());
        remember(vec![element("e1", "Save"), element("e2", "Cancel")]);
        assert_eq!(lookup("e2").unwrap().name, "Cancel");
        assert_eq!(
            lookup(" E1 ").unwrap().name,
            "Save",
            "ids ignore case and spaces"
        );
        remember(vec![element("e1", "Open")]);
        assert!(
            lookup("e2").is_none(),
            "an old id must not survive a new listing"
        );
        assert_eq!(lookup("e1").unwrap().name, "Open");
        forget();
        assert!(lookup("e1").is_none());
    }
}
