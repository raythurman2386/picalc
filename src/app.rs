use std::borrow::Cow;
use std::time::{Duration, Instant};

use gpui_kit::component::{h_flex, v_flex, ActiveTheme, Root, Theme, ThemeMode};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use picalc::calc::key_from_str;
use picalc::theme::{detect_system_dark, detect_text_scale, omarchy_watch_paths, OmarchyPalette};
use picalc::Calculator;

actions!(
    picalc_actions,
    [
        PressZero,
        PressOne,
        PressTwo,
        PressThree,
        PressFour,
        PressFive,
        PressSix,
        PressSeven,
        PressEight,
        PressNine,
        PressDecimal,
        PressAdd,
        PressSubtract,
        PressMultiply,
        PressDivide,
        PressPercent,
        PressEquals,
        PressSign,
        PressBackspace,
        PressClear,
        CopyResult,
        PasteNumber,
        Quit,
    ]
);

pub fn init(cx: &mut App) {
    load_fonts(cx);

    // Digits and symbols are bound as keybindings so every keyboard input is
    // a real, greppable binding; enter, backspace, clear and sign are bound
    // by key name below.
    cx.bind_keys([
        KeyBinding::new("0", PressZero, None),
        KeyBinding::new("1", PressOne, None),
        KeyBinding::new("2", PressTwo, None),
        KeyBinding::new("3", PressThree, None),
        KeyBinding::new("4", PressFour, None),
        KeyBinding::new("5", PressFive, None),
        KeyBinding::new("6", PressSix, None),
        KeyBinding::new("7", PressSeven, None),
        KeyBinding::new("8", PressEight, None),
        KeyBinding::new("9", PressNine, None),
        KeyBinding::new(".", PressDecimal, None),
        KeyBinding::new(",", PressDecimal, None),
        KeyBinding::new("+", PressAdd, None),
        KeyBinding::new("-", PressSubtract, None),
        KeyBinding::new("*", PressMultiply, None),
        KeyBinding::new("/", PressDivide, None),
        KeyBinding::new("%", PressPercent, None),
        KeyBinding::new("=", PressEquals, None),
        KeyBinding::new("enter", PressEquals, None),
        KeyBinding::new("backspace", PressBackspace, None),
        KeyBinding::new("delete", PressBackspace, None),
        KeyBinding::new("c", PressClear, None),
        KeyBinding::new("escape", PressClear, None),
        KeyBinding::new("s", PressSign, None),
        KeyBinding::new("ctrl-c", CopyResult, None),
        KeyBinding::new("super-c", CopyResult, None),
        KeyBinding::new("ctrl-v", PasteNumber, None),
        KeyBinding::new("super-v", PasteNumber, None),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
}

fn load_fonts(cx: &mut App) {
    let fonts: [&'static [u8]; 4] = [
        include_bytes!("../fonts/iAWriterMonoS-Regular.ttf"),
        include_bytes!("../fonts/iAWriterMonoS-Italic.ttf"),
        include_bytes!("../fonts/iAWriterMonoS-Bold.ttf"),
        include_bytes!("../fonts/iAWriterMonoS-BoldItalic.ttf"),
    ];
    let blobs = fonts.into_iter().map(Cow::Borrowed).collect::<Vec<_>>();
    let _ = cx.text_system().add_fonts(blobs);
}

pub fn open_window(cx: &AsyncApp) -> anyhow::Result<WindowHandle<Root>> {
    cx.open_window(window_options(), move |window, cx| {
        let view: Entity<Picalc> = cx.new(|cx| Picalc::new(window, cx));
        cx.new(|cx| {
            let any_view: AnyView = view.into();
            Root::new(any_view, window, cx)
        })
    })
    .map_err(|e| anyhow::anyhow!("{e}"))
}

fn window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(120.), px(120.)),
            size: size(px(400.), px(568.)),
        })),
        window_min_size: Some(size(px(340.), px(500.))),
        titlebar: Some(TitlebarOptions {
            title: Some("Picalc".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        app_id: Some("picalc".into()),
        ..Default::default()
    }
}

/// A notify watcher on the Omarchy theme paths, drained on each frame so a
/// theme switch re-tints the calculator live.
struct ThemeWatch {
    events: std::sync::Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>,
    _watcher: Option<RecommendedWatcher>,
}

impl ThemeWatch {
    fn new() -> Self {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let tx = events.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Create(_)
                    ) {
                        if let Ok(mut queue) = tx.lock() {
                            queue.extend(event.paths);
                        }
                    }
                }
            },
            notify::Config::default(),
        )
        .ok();
        if let Some(watcher) = watcher.as_mut() {
            for path in omarchy_watch_paths() {
                if path.exists() {
                    let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
                }
            }
        }
        Self {
            events,
            _watcher: watcher,
        }
    }

    fn drain(&self) -> Vec<std::path::PathBuf> {
        self.events
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }
}

/// One keypad button: the engine key it dispatches and its visual role.
#[derive(Clone, Copy)]
struct KeypadButton {
    id: &'static str,
    label: &'static str,
    key: &'static str,
    role: ButtonRole,
}

#[derive(Clone, Copy, PartialEq)]
enum ButtonRole {
    Number,
    Operator,
    Equals,
}

/// The keypad in omacalc's layout: AC ± % ÷ / 7 8 9 × / 4 5 6 − /
/// 1 2 3 + / 0 . ⌫ =.
fn keypad_rows() -> Vec<Vec<KeypadButton>> {
    const NUMBER: ButtonRole = ButtonRole::Number;
    const OPERATOR: ButtonRole = ButtonRole::Operator;
    const EQUALS: ButtonRole = ButtonRole::Equals;
    vec![
        vec![
            KeypadButton {
                id: "clear",
                label: "AC",
                key: "clear",
                role: NUMBER,
            },
            KeypadButton {
                id: "sign",
                label: "±",
                key: "sign",
                role: NUMBER,
            },
            KeypadButton {
                id: "percent",
                label: "%",
                key: "%",
                role: NUMBER,
            },
            KeypadButton {
                id: "divide",
                label: "÷",
                key: "÷",
                role: OPERATOR,
            },
        ],
        vec![
            KeypadButton {
                id: "seven",
                label: "7",
                key: "7",
                role: NUMBER,
            },
            KeypadButton {
                id: "eight",
                label: "8",
                key: "8",
                role: NUMBER,
            },
            KeypadButton {
                id: "nine",
                label: "9",
                key: "9",
                role: NUMBER,
            },
            KeypadButton {
                id: "multiply",
                label: "×",
                key: "×",
                role: OPERATOR,
            },
        ],
        vec![
            KeypadButton {
                id: "four",
                label: "4",
                key: "4",
                role: NUMBER,
            },
            KeypadButton {
                id: "five",
                label: "5",
                key: "5",
                role: NUMBER,
            },
            KeypadButton {
                id: "six",
                label: "6",
                key: "6",
                role: NUMBER,
            },
            KeypadButton {
                id: "subtract",
                label: "−",
                key: "−",
                role: OPERATOR,
            },
        ],
        vec![
            KeypadButton {
                id: "one",
                label: "1",
                key: "1",
                role: NUMBER,
            },
            KeypadButton {
                id: "two",
                label: "2",
                key: "2",
                role: NUMBER,
            },
            KeypadButton {
                id: "three",
                label: "3",
                key: "3",
                role: NUMBER,
            },
            KeypadButton {
                id: "add",
                label: "+",
                key: "+",
                role: OPERATOR,
            },
        ],
        vec![
            KeypadButton {
                id: "zero",
                label: "0",
                key: "0",
                role: NUMBER,
            },
            KeypadButton {
                id: "decimal",
                label: ".",
                key: ".",
                role: NUMBER,
            },
            KeypadButton {
                id: "backspace",
                label: "⌫",
                key: "backspace",
                role: NUMBER,
            },
            KeypadButton {
                id: "equals",
                label: "=",
                key: "=",
                role: EQUALS,
            },
        ],
    ]
}

pub struct Picalc {
    calculator: Calculator,
    palette: OmarchyPalette,
    text_scale: f32,
    fit_scale: f32,
    theme_watch: ThemeWatch,
    last_theme_check: Instant,
    focus_handle: FocusHandle,
}

impl Focusable for Picalc {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Picalc {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dark = detect_system_dark();
        let palette = OmarchyPalette::load(dark);
        let text_scale = detect_text_scale();
        let focus_handle = cx.focus_handle();

        apply_palette(&palette, Some(window), cx);
        window.set_window_title("Picalc");

        let this = Self {
            calculator: Calculator::new(),
            palette,
            text_scale,
            fit_scale: Self::compute_fit_scale(window),
            theme_watch: ThemeWatch::new(),
            last_theme_check: Instant::now(),
            focus_handle,
        };
        let handle = this.focus_handle.clone();
        window.focus(&handle, cx);
        this
    }

    /// The one engine input entry the keypad buttons, the keyboard
    /// bindings, and the raw key handler all dispatch through — the same
    /// `Calculator::press_str` the unit tests exercise.
    fn press_str(&mut self, key: &str, cx: &mut Context<Self>) {
        self.calculator.press_str(key);
        cx.notify();
    }

    fn copy_result(&self, cx: &mut Context<Self>) {
        let text = self.calculator.copy_text();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn paste_number(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.calculator.paste_text(&text);
                cx.notify();
            }
        }
    }

    /// Re-check the Omarchy theme when the watcher fires (or the desktop
    /// text scale changes) so the face re-tints live.
    fn poll_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.last_theme_check.elapsed() <= Duration::from_millis(400) {
            return;
        }
        self.last_theme_check = Instant::now();
        if !self.theme_watch.drain().is_empty() {
            let palette = OmarchyPalette::load(self.palette.dark);
            if palette != self.palette {
                self.palette = palette;
                apply_palette(&self.palette, Some(window), cx);
                cx.notify();
            }
        }
        let scale = detect_text_scale();
        if (scale - self.text_scale).abs() > f32::EPSILON {
            self.text_scale = scale;
            apply_palette(&self.palette, Some(window), cx);
            cx.notify();
        }
    }

    fn on_copy(&mut self, _: &CopyResult, _: &mut Window, cx: &mut Context<Self>) {
        self.copy_result(cx);
    }

    fn on_paste(&mut self, _: &PasteNumber, _: &mut Window, cx: &mut Context<Self>) {
        self.paste_number(cx);
    }

    fn on_quit(&mut self, _: &Quit, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    fn on_press_zero(&mut self, _: &PressZero, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("0", cx);
    }
    fn on_press_one(&mut self, _: &PressOne, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("1", cx);
    }
    fn on_press_two(&mut self, _: &PressTwo, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("2", cx);
    }
    fn on_press_three(&mut self, _: &PressThree, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("3", cx);
    }
    fn on_press_four(&mut self, _: &PressFour, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("4", cx);
    }
    fn on_press_five(&mut self, _: &PressFive, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("5", cx);
    }
    fn on_press_six(&mut self, _: &PressSix, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("6", cx);
    }
    fn on_press_seven(&mut self, _: &PressSeven, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("7", cx);
    }
    fn on_press_eight(&mut self, _: &PressEight, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("8", cx);
    }
    fn on_press_nine(&mut self, _: &PressNine, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("9", cx);
    }
    fn on_press_decimal(&mut self, _: &PressDecimal, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str(".", cx);
    }
    fn on_press_add(&mut self, _: &PressAdd, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("+", cx);
    }
    fn on_press_subtract(&mut self, _: &PressSubtract, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("-", cx);
    }
    fn on_press_multiply(&mut self, _: &PressMultiply, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("*", cx);
    }
    fn on_press_divide(&mut self, _: &PressDivide, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("/", cx);
    }
    fn on_press_percent(&mut self, _: &PressPercent, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("%", cx);
    }
    fn on_press_equals(&mut self, _: &PressEquals, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("=", cx);
    }
    fn on_press_sign(&mut self, _: &PressSign, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("sign", cx);
    }
    fn on_press_backspace(&mut self, _: &PressBackspace, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("backspace", cx);
    }
    fn on_press_clear(&mut self, _: &PressClear, _: &mut Window, cx: &mut Context<Self>) {
        self.press_str("clear", cx);
    }

    /// Every fixed size in the layout routes through here, so one factor
    /// scales the whole face: the desktop text scale from `gsettings` times
    /// a fit factor that shrinks the design (573px tall) to the window's
    /// actual height when the compositor tiles it smaller.
    fn scaled(&self, px_value: f32) -> Pixels {
        px(px_value * self.text_scale * self.fit_scale)
    }

    /// The design is laid out at 400x573 logical pixels; compute how much
    /// of it fits the current viewport, clamped so it never grows past 1.
    fn compute_fit_scale(window: &Window) -> f32 {
        let viewport = window.viewport_size();
        let height_fit = (viewport.height / px(573.)).min(1.0);
        let width_fit = (viewport.width / px(400.)).min(1.0);
        height_fit.min(width_fit)
    }

    fn render_keypad_button(
        &self,
        button: KeypadButton,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let page = hex_to_hsla(&self.palette.background).unwrap_or_else(|| cx.theme().background);
        let ink = hex_to_hsla(&self.palette.foreground).unwrap_or_else(|| cx.theme().foreground);
        let resting_lift = match button.role {
            ButtonRole::Operator => 0.16,
            _ => 0.05,
        };
        let (button_bg, button_fg) = if button.role == ButtonRole::Equals {
            (ink, page)
        } else {
            (mix_colors(page, ink, resting_lift), ink)
        };
        let border = mix_colors(page, ink, 0.13);
        let label = button.label.to_string();
        let key = button.key.to_string();

        div()
            .id(button.id)
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(self.scaled(24.))
            .bg(button_bg)
            .rounded_lg()
            .when(button.role == ButtonRole::Number, |this| {
                this.border_color(border).border_1()
            })
            .text_color(button_fg)
            .hover(|style| style.bg(mix_colors(page, ink, resting_lift + 0.045)))
            .active(|style| style.bg(mix_colors(page, ink, resting_lift + 0.09)))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                // Keypad clicks dispatch through the same engine input
                // entry the keyboard bindings and tests use.
                this.press_str(&key, cx);
            }))
    }

    fn render_keypad(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = keypad_rows();
        v_flex()
            .id("keypad")
            .w_full()
            .flex_grow(1.)
            .min_h(self.scaled(64. * 5. + 12. * 4.))
            .gap(self.scaled(12.))
            .children(rows.into_iter().map(|row| {
                h_flex()
                    .id(row[0].id)
                    .w_full()
                    .flex_1()
                    .min_h(self.scaled(40.))
                    .gap(self.scaled(12.))
                    .children(
                        row.into_iter()
                            .map(|button| self.render_keypad_button(button, cx))
                            .collect::<Vec<_>>(),
                    )
            }))
    }

    fn render_display(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let page = hex_to_hsla(&self.palette.background).unwrap_or_else(|| cx.theme().background);
        let ink = hex_to_hsla(&self.palette.foreground).unwrap_or_else(|| cx.theme().foreground);
        let muted = mix_colors(page, ink, 0.5);

        let expression = self.calculator.expression();
        let display = self.calculator.display();
        let is_error = self.calculator.is_errored();

        v_flex()
            .id("display")
            .flex_1()
            .w_full()
            .min_h(self.scaled(64.))
            .gap(self.scaled(6.))
            .justify_end()
            .child(
                div()
                    .id("expression")
                    .w_full()
                    .text_size(self.scaled(21.))
                    .text_color(muted)
                    .child(expression),
            )
            .child(
                div()
                    .id("result")
                    .w_full()
                    .flex()
                    .justify_end()
                    .text_size(self.scaled(56.))
                    .line_height(relative(1.1))
                    .text_color(if is_error { muted } else { ink })
                    .child(display),
            )
    }
}

impl Render for Picalc {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_theme(window, cx);

        // Re-fit on every render: renders happen on resize, and the face
        // must shrink with the window when the compositor tiles it small.
        let fit_scale = Self::compute_fit_scale(window);
        if (fit_scale - self.fit_scale).abs() > f32::EPSILON {
            self.fit_scale = fit_scale;
            cx.notify();
        }

        let page = hex_to_hsla(&self.palette.background).unwrap_or_else(|| cx.theme().background);
        let ink = hex_to_hsla(&self.palette.foreground).unwrap_or_else(|| cx.theme().foreground);
        let divider = mix_colors(page, ink, 0.16);

        v_flex()
            .id("picalc")
            .size_full()
            .bg(page)
            .font_family("iA Writer Mono S")
            .text_size(self.scaled(16.))
            .key_context("picalc")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_quit))
            .on_action(cx.listener(Self::on_press_zero))
            .on_action(cx.listener(Self::on_press_one))
            .on_action(cx.listener(Self::on_press_two))
            .on_action(cx.listener(Self::on_press_three))
            .on_action(cx.listener(Self::on_press_four))
            .on_action(cx.listener(Self::on_press_five))
            .on_action(cx.listener(Self::on_press_six))
            .on_action(cx.listener(Self::on_press_seven))
            .on_action(cx.listener(Self::on_press_eight))
            .on_action(cx.listener(Self::on_press_nine))
            .on_action(cx.listener(Self::on_press_decimal))
            .on_action(cx.listener(Self::on_press_add))
            .on_action(cx.listener(Self::on_press_subtract))
            .on_action(cx.listener(Self::on_press_multiply))
            .on_action(cx.listener(Self::on_press_divide))
            .on_action(cx.listener(Self::on_press_percent))
            .on_action(cx.listener(Self::on_press_equals))
            .on_action(cx.listener(Self::on_press_sign))
            .on_action(cx.listener(Self::on_press_backspace))
            .on_action(cx.listener(Self::on_press_clear))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                handle_raw_key(this, &event.keystroke, cx)
            }))
            .child(
                v_flex()
                    .id("face")
                    .flex_1()
                    .w_full()
                    .p(self.scaled(20.))
                    .gap(self.scaled(22.))
                    .child(self.render_display(cx))
                    .child(div().id("divider").w_full().h(px(1.)).bg(divider))
                    .child(self.render_keypad(cx)),
            )
    }
}

/// Keyboard input as omacalc handles it: printable keys type their
/// character; Enter/Backspace/Escape/Delete act by key name; any modifier
/// but Shift lets the action layer take over instead (so Ctrl+C copies
/// rather than typing "c").
fn handle_raw_key(this: &mut Picalc, keystroke: &Keystroke, cx: &mut Context<Picalc>) {
    let modifiers = &keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return;
    }
    match keystroke.key.as_str() {
        "enter" => this.press_str("=", cx),
        "backspace" => this.press_str("backspace", cx),
        "escape" | "delete" => this.press_str("clear", cx),
        _ => {
            let typed = keystroke
                .key_char
                .clone()
                .unwrap_or_else(|| keystroke.key.clone());
            let typed = match typed.as_str() {
                "\n" => "=".to_string(),
                "," => ".".to_string(),
                _ => typed,
            };
            // Unhandled printable keys (like "x") fall through so nothing is
            // consumed that the calculator does not understand.
            if key_from_str(&typed).is_some() {
                this.press_str(&typed, cx);
            }
        }
    }
}

fn apply_palette(palette: &OmarchyPalette, window: Option<&mut Window>, cx: &mut App) {
    Theme::change(
        if palette.dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        window,
        cx,
    );
    let theme = Theme::global_mut(cx);
    if let Some(bg) = hex_to_hsla(&palette.background) {
        theme.colors.background = bg;
    }
    if let Some(fg) = hex_to_hsla(&palette.foreground) {
        theme.colors.foreground = fg;
    }
    if let Some(accent) = hex_to_hsla(&palette.accent) {
        theme.colors.primary = accent;
        theme.colors.accent = accent;
    }
    theme.mono_font_family = "iA Writer Mono S".into();
    theme.mono_font_size = px(24.);
    Theme::sync_base(cx);
}

fn hex_to_hsla(value: &str) -> Option<Hsla> {
    let hex = value.trim().trim_start_matches('#');
    let expanded = if hex.len() == 3 {
        hex.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        hex.to_string()
    };
    let n = u32::from_str_radix(&expanded, 16).ok()?;
    Some(rgb(n).into())
}

/// Blend `amount` of `tint`'s lightness into `base`, taking tint's hue for
/// grayscale-tinted results the way omacalc's mixColors does.
fn mix_colors(base: Hsla, tint: Hsla, amount: f32) -> Hsla {
    Hsla {
        h: tint.h,
        s: tint.s,
        l: base.l + (tint.l - base.l) * amount,
        a: 1.0,
    }
}
