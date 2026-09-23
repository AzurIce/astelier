//! 内联 SVG 图标（lucide 风格，24×24 stroke，随 currentColor 着色）。

use dioxus::prelude::*;

fn svg(children: Element, size: Option<u32>) -> Element {
    let s = size.unwrap_or(16);
    rsx! {
        svg {
            width: "{s}",
            height: "{s}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            "aria-hidden": "true",
            {children}
        }
    }
}

#[component]
pub fn IconSun(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            circle { cx: "12", cy: "12", r: "4" }
            path { d: "M12 2v2" } path { d: "M12 20v2" }
            path { d: "m4.93 4.93 1.41 1.41" } path { d: "m17.66 17.66 1.41 1.41" }
            path { d: "M2 12h2" } path { d: "M20 12h2" }
            path { d: "m6.34 17.66-1.41 1.41" } path { d: "m19.07 4.93-1.41 1.41" }
        }, size)}
    }
}

#[component]
pub fn IconMoon(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" } }, size)}
    }
}

#[component]
pub fn IconSliders(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            line { x1: "21", x2: "14", y1: "4", y2: "4" }
            line { x1: "10", x2: "3", y1: "4", y2: "4" }
            line { x1: "21", x2: "12", y1: "12", y2: "12" }
            line { x1: "8", x2: "3", y1: "12", y2: "12" }
            line { x1: "21", x2: "16", y1: "20", y2: "20" }
            line { x1: "12", x2: "3", y1: "20", y2: "20" }
            line { x1: "14", x2: "14", y1: "2", y2: "6" }
            line { x1: "8", x2: "8", y1: "10", y2: "14" }
            line { x1: "16", x2: "16", y1: "18", y2: "22" }
        }, size)}
    }
}

#[component]
pub fn IconPlus(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "M5 12h14" } path { d: "M12 5v14" } }, size)}
    }
}

#[component]
pub fn IconPlay(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { polygon { points: "6 3 20 12 6 21 6 3" } }, size)}
    }
}

#[component]
pub fn IconWorkflow(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            rect { x: "3", y: "3", width: "8", height: "8", rx: "2" }
            rect { x: "13", y: "13", width: "8", height: "8", rx: "2" }
            path { d: "M11 7h4a2 2 0 0 1 2 2v4" }
            path { d: "M7 11v4a2 2 0 0 0 2 2h4" }
        }, size)}
    }
}

#[component]
pub fn IconX(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "M18 6 6 18" } path { d: "m6 6 12 12" } }, size)}
    }
}

#[component]
pub fn IconTrash(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M3 6h18" }
            path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" }
            path { d: "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
            line { x1: "10", x2: "10", y1: "11", y2: "17" }
            line { x1: "14", x2: "14", y1: "11", y2: "17" }
        }, size)}
    }
}

#[component]
pub fn IconDownload(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { points: "7 10 12 15 17 10" }
            line { x1: "12", x2: "12", y1: "15", y2: "3" }
        }, size)}
    }
}

#[component]
pub fn IconImage(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            rect { x: "3", y: "3", width: "18", height: "18", rx: "2", ry: "2" }
            circle { cx: "9", cy: "9", r: "2" }
            path { d: "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" }
        }, size)}
    }
}

#[component]
pub fn IconSparkles(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" }
            path { d: "M20 3v4" } path { d: "M22 5h-4" }
            path { d: "M4 17v2" } path { d: "M5 18H3" }
        }, size)}
    }
}

#[component]
pub fn IconRefresh(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" }
            path { d: "M21 3v5h-5" }
            path { d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" }
            path { d: "M8 16H3v5" }
        }, size)}
    }
}

#[component]
pub fn IconChevronDown(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "m6 9 6 6 6-6" } }, size)}
    }
}

#[component]
pub fn IconChevronLeft(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "m15 18-6-6 6-6" } }, size)}
    }
}

#[component]
pub fn IconChevronRight(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "m9 18 6-6-6-6" } }, size)}
    }
}

#[component]
pub fn IconEye(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" }
            circle { cx: "12", cy: "12", r: "3" }
        }, size)}
    }
}

#[component]
pub fn IconEyeOff(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M9.88 9.88a3 3 0 1 0 4.24 4.24" }
            path { d: "M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68" }
            path { d: "M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61" }
            line { x1: "2", x2: "22", y1: "2", y2: "22" }
        }, size)}
    }
}

#[component]
pub fn IconAlert(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "12", x2: "12", y1: "8", y2: "12" }
            line { x1: "12", x2: "12.01", y1: "16", y2: "16" }
        }, size)}
    }
}

#[component]
pub fn IconLoader(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "M21 12a9 9 0 1 1-6.219-8.56" } }, size)}
    }
}

#[component]
pub fn IconCopy(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            rect { x: "8", y: "8", width: "14", height: "14", rx: "2", ry: "2" }
            path { d: "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" }
        }, size)}
    }
}

#[component]
pub fn IconCheck(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! { path { d: "M20 6 9 17l-5-5" } }, size)}
    }
}

#[component]
pub fn IconMask(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            rect { x: "3", y: "3", width: "18", height: "18", rx: "2" }
            path { d: "M3 15h18" } path { d: "M9 15v6" }
        }, size)}
    }
}

#[component]
pub fn IconDots(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            circle { cx: "12", cy: "12", r: "1" }
            circle { cx: "19", cy: "12", r: "1" }
            circle { cx: "5", cy: "12", r: "1" }
        }, size)}
    }
}

#[component]
pub fn IconSwap(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M8 21V3" } path { d: "m5 6 3-3 3 3" }
            path { d: "M16 3v18" } path { d: "m13 18 3 3 3-3" }
        }, size)}
    }
}

#[component]
pub fn IconFolder(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
        }, size)}
    }
}

#[component]
pub fn IconFolderPlus(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
            line { x1: "12", x2: "12", y1: "10", y2: "16" }
            line { x1: "9", x2: "15", y1: "13", y2: "13" }
        }, size)}
    }
}

#[component]
pub fn IconPencil(size: Option<u32>) -> Element {
    rsx! {
        {svg(rsx! {
            path { d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" }
            path { d: "m15 5 4 4" }
        }, size)}
    }
}
