# TUI Patterns

## Table of Contents
1. [Screen-Based Architecture](#1-screen-based-architecture)
2. [View State Management](#2-view-state-management)
3. [Event Handling](#3-event-handling)
4. [Rendering Pipeline](#4-rendering-pipeline)
5. [Keyboard Navigation](#5-keyboard-navigation)
6. [Periodic Refresh](#6-periodic-refresh)

---

## 1. Screen-Based Architecture

Organize UI into switchable screens:

```rust
use cursive::Cursive;
use cursive::views::ScreensView;
use std::collections::HashMap;

pub struct App {
    siv: Cursive,
    screens: HashMap<String, usize>,
    active_screen: String,
}

impl App {
    pub fn new() -> Self {
        let mut siv = Cursive::default();

        // Configure theme
        siv.set_theme(Self::create_theme());

        Self {
            siv,
            screens: HashMap::new(),
            active_screen: String::new(),
        }
    }

    pub fn register_screen<V: cursive::View>(&mut self, name: &str, view: V) {
        let screen_id = self.siv.add_active_screen();
        self.siv.screen_mut().add_layer(view);
        self.screens.insert(name.to_string(), screen_id);

        if self.active_screen.is_empty() {
            self.active_screen = name.to_string();
        }
    }

    pub fn switch_screen(&mut self, name: &str) {
        if let Some(&id) = self.screens.get(name) {
            self.siv.set_screen(id);
            self.active_screen = name.to_string();
        }
    }

    fn create_theme() -> cursive::theme::Theme {
        use cursive::theme::*;

        let mut theme = Theme::default();
        theme.palette[PaletteColor::Background] = Color::TerminalDefault;
        theme.palette[PaletteColor::View] = Color::TerminalDefault;
        theme.palette[PaletteColor::Primary] = Color::TerminalDefault;
        theme.palette[PaletteColor::Highlight] = Color::Light(BaseColor::Cyan);
        theme
    }

    pub fn run(&mut self) {
        self.siv.run();
    }
}
```

## 2. View State Management

Separate view state from UI components:

```rust
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
pub struct ViewState {
    // Data
    pub model: Model,

    // Selection state
    pub selected_index: usize,
    pub scroll_offset: usize,

    // Collapse state for tree views
    pub collapsed: HashSet<String>,

    // Filter/sort state
    pub filter: Option<FilterConfig>,
    pub sort_field: Option<FieldId>,
    pub sort_ascending: bool,

    // View mode
    pub view_mode: ViewMode,
}

#[derive(Debug, Clone, Copy)]
pub enum ViewMode {
    Normal,
    Search,
    Help,
    Detail,
}

impl ViewState {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            selected_index: 0,
            scroll_offset: 0,
            collapsed: HashSet::new(),
            filter: None,
            sort_field: None,
            sort_ascending: true,
            view_mode: ViewMode::Normal,
        }
    }

    pub fn toggle_collapse(&mut self, path: &str) {
        if self.collapsed.contains(path) {
            self.collapsed.remove(path);
        } else {
            self.collapsed.insert(path.to_string());
        }
    }

    pub fn set_sort(&mut self, field: FieldId) {
        if self.sort_field == Some(field.clone()) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_field = Some(field);
            self.sort_ascending = false; // Default descending
        }
    }
}

// Thread-safe wrapper
pub type SharedState = Arc<Mutex<ViewState>>;

pub fn create_shared_state(model: Model) -> SharedState {
    Arc::new(Mutex::new(ViewState::new(model)))
}
```

## 3. Event Handling

Map keys to actions with closures:

```rust
use cursive::event::Event;
use cursive::event::Key;

pub struct EventController {
    handlers: HashMap<Event, Box<dyn Fn(&mut Cursive) + Send>>,
}

impl EventController {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, event: Event, handler: F)
    where
        F: Fn(&mut Cursive) + Send + 'static,
    {
        self.handlers.insert(event, Box::new(handler));
    }

    pub fn register_char<F>(&mut self, c: char, handler: F)
    where
        F: Fn(&mut Cursive) + Send + 'static,
    {
        self.register(Event::Char(c), handler);
    }

    pub fn apply(self, siv: &mut Cursive) {
        for (event, handler) in self.handlers {
            siv.add_global_callback(event, handler);
        }
    }
}

fn setup_handlers(state: SharedState) -> EventController {
    let mut ctrl = EventController::new();

    // Quit
    ctrl.register_char('q', |s| s.quit());

    // Navigation
    let state_clone = Arc::clone(&state);
    ctrl.register(Event::Key(Key::Up), move |s| {
        let mut state = state_clone.lock().unwrap();
        if state.selected_index > 0 {
            state.selected_index -= 1;
        }
        refresh_view(s);
    });

    let state_clone = Arc::clone(&state);
    ctrl.register(Event::Key(Key::Down), move |s| {
        let mut state = state_clone.lock().unwrap();
        state.selected_index += 1;
        refresh_view(s);
    });

    // Sorting
    let state_clone = Arc::clone(&state);
    ctrl.register_char('C', move |s| {
        state_clone.lock().unwrap().set_sort(FieldId::Cpu);
        refresh_view(s);
    });

    let state_clone = Arc::clone(&state);
    ctrl.register_char('M', move |s| {
        state_clone.lock().unwrap().set_sort(FieldId::Memory);
        refresh_view(s);
    });

    // Toggle collapse
    let state_clone = Arc::clone(&state);
    ctrl.register(Event::Key(Key::Enter), move |s| {
        let mut state = state_clone.lock().unwrap();
        if let Some(path) = get_selected_path(&state) {
            state.toggle_collapse(&path);
        }
        refresh_view(s);
    });

    ctrl
}
```

## 4. Rendering Pipeline

Build styled output from model:

```rust
use cursive::utils::markup::StyledString;
use cursive::theme::Style;
use cursive::theme::Effect;
use cursive::theme::ColorStyle;

pub struct Renderer {
    config: RenderConfig,
}

impl Renderer {
    pub fn render_row(&self, model: &RowModel, selected: bool) -> StyledString {
        let mut styled = StyledString::new();

        // Apply selection highlight
        let base_style = if selected {
            Style::from(ColorStyle::highlight())
        } else {
            Style::default()
        };

        // Render each column
        for (i, field) in model.fields.iter().enumerate() {
            if i > 0 {
                styled.append_plain(" ");
            }

            let formatted = self.format_field(field);
            let width = self.config.columns[i].width;
            let padded = format!("{:width$}", formatted, width = width);

            styled.append_styled(padded, base_style);
        }

        styled
    }

    pub fn render_header(&self) -> StyledString {
        let mut styled = StyledString::new();
        let header_style = Style::from(Effect::Bold);

        for (i, col) in self.config.columns.iter().enumerate() {
            if i > 0 {
                styled.append_plain(" ");
            }
            let padded = format!("{:width$}", col.title, width = col.width);
            styled.append_styled(padded, header_style);
        }

        styled
    }

    fn format_field(&self, field: &Field) -> String {
        match &field.format {
            RenderFormat::Default => format!("{}", field.value),
            RenderFormat::ReadableSize => format_bytes(field.value.as_u64()),
            RenderFormat::Duration => format_duration(field.value.as_u64()),
            RenderFormat::Percent => format!("{:.1}%", field.value.as_f64()),
            RenderFormat::Precision(n) => format!("{:.1$}", field.value.as_f64(), *n as usize),
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    match bytes {
        b if b >= GB => format!("{:.1}G", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1}M", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1}K", b as f64 / KB as f64),
        b => format!("{}B", b),
    }
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}h{:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m{:02}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
```

## 5. Keyboard Navigation

Vim-style navigation support:

```rust
pub struct Navigator {
    state: SharedState,
    page_size: usize,
}

impl Navigator {
    pub fn new(state: SharedState, page_size: usize) -> Self {
        Self { state, page_size }
    }

    pub fn move_up(&self, count: usize) {
        let mut state = self.state.lock().unwrap();
        state.selected_index = state.selected_index.saturating_sub(count);
        self.ensure_visible(&mut state);
    }

    pub fn move_down(&self, count: usize) {
        let mut state = self.state.lock().unwrap();
        let max = self.get_item_count(&state).saturating_sub(1);
        state.selected_index = (state.selected_index + count).min(max);
        self.ensure_visible(&mut state);
    }

    pub fn page_up(&self) {
        self.move_up(self.page_size);
    }

    pub fn page_down(&self) {
        self.move_down(self.page_size);
    }

    pub fn go_top(&self) {
        let mut state = self.state.lock().unwrap();
        state.selected_index = 0;
        state.scroll_offset = 0;
    }

    pub fn go_bottom(&self) {
        let mut state = self.state.lock().unwrap();
        let max = self.get_item_count(&state).saturating_sub(1);
        state.selected_index = max;
        self.ensure_visible(&mut state);
    }

    fn ensure_visible(&self, state: &mut ViewState) {
        // Scroll up if needed
        if state.selected_index < state.scroll_offset {
            state.scroll_offset = state.selected_index;
        }
        // Scroll down if needed
        if state.selected_index >= state.scroll_offset + self.page_size {
            state.scroll_offset = state.selected_index - self.page_size + 1;
        }
    }

    fn get_item_count(&self, state: &ViewState) -> usize {
        state.model.items.len()
    }
}

// Register vim-style keys
fn register_vim_keys(ctrl: &mut EventController, nav: Arc<Navigator>) {
    let n = Arc::clone(&nav);
    ctrl.register_char('j', move |_| n.move_down(1));

    let n = Arc::clone(&nav);
    ctrl.register_char('k', move |_| n.move_up(1));

    let n = Arc::clone(&nav);
    ctrl.register(Event::CtrlChar('d'), move |_| n.page_down());

    let n = Arc::clone(&nav);
    ctrl.register(Event::CtrlChar('u'), move |_| n.page_up());

    let n = Arc::clone(&nav);
    ctrl.register_char('g', move |_| n.go_top());

    let n = Arc::clone(&nav);
    ctrl.register_char('G', move |_| n.go_bottom());
}
```

## 6. Periodic Refresh

Update display at intervals:

```rust
use std::time::Duration;

const REFRESH_RATE: Duration = Duration::from_millis(250); // 4 FPS

pub fn start_refresh_loop(
    siv: &mut Cursive,
    state: SharedState,
    collector: Arc<Collector>,
) {
    let cb_sink = siv.cb_sink().clone();

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(REFRESH_RATE);

            // Collect new data
            if let Ok(new_model) = collector.collect() {
                state.lock().unwrap().model = new_model;
            }

            // Request UI refresh
            let state_clone = Arc::clone(&state);
            if cb_sink.send(Box::new(move |s| {
                refresh_view_with_state(s, &state_clone);
            })).is_err() {
                break; // UI closed
            }
        }
    });
}

fn refresh_view_with_state(siv: &mut Cursive, state: &SharedState) {
    // Find and update the main view
    siv.call_on_name("main_view", |view: &mut MainView| {
        let state = state.lock().unwrap();
        view.update(&state);
    });
}

// Alternative: Event-based refresh
pub fn setup_event_refresh(siv: &mut Cursive) {
    siv.add_global_callback(Event::Refresh, |s| {
        s.call_on_name("main_view", |view: &mut MainView| {
            view.refresh();
        });
    });

    // Set refresh rate
    siv.set_fps(4);
    siv.set_autorefresh(true);
}
```
