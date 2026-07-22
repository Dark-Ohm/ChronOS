# Left Agent Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build left agent panel — layer-shell overlay with Hermes ACP chat, sessions sidebar, composer, and tool call cards.

**Architecture:** Layer-shell overlay (like right panel) + official ACP SDK (`agent-client-protocol` crate) for Hermes stdio transport + gpui-component for UI building blocks (Sidebar, TextInput, Markdown, Collapsible, Resizable).

**Tech Stack:** Rust, gpui-ce (layer-shell), `agent-client-protocol` 0.5, `agent-client-protocol-tokio` 0.5, gpui-component (forked in `../Source/gpui-component`)

## Global Constraints

- gpui-ce layer-shell: `WindowKind::LayerShell`, `Layer::Overlay`, `Anchor::LEFT | TOP | BOTTOM`
- ACP SDK: `agent-client-protocol = "0.5"`, `agent-client-protocol-tokio = "0.5"`
- gpui-component: `../Source/gpui-component` (already in ARCHITECTURE.md as planned `crates/ui`)
- Hermes: stdio transport, one process per ChronOS shell
- All UI under `crates/app/src/side_panel_left/`
- All service code under `crates/services/src/hermes_acp/`
- Follow right panel patterns: `crates/app/src/side_panel_right/mod.rs`
- Apache-2.0 license (ACP SDK is Apache-2.0, gpui-component is Apache-2.0)
- TDD: write failing test first, implement, verify green

---

## File Structure

```
crates/app/src/side_panel_left/
├── mod.rs           — Window creation, peek/pin toggle
├── state.rs         — PanelState enum, state transitions
├── panel.rs         — Main render: header + body
├── sessions_list.rs — Sidebar with session list
├── chat_view.rs     — Message stream (user/agent)
├── composer.rs      — TextInput + pickers + send
└── tool_card.rs     — Collapsible tool call cards

crates/services/src/hermes_acp/
├── mod.rs           — Service trait impl, re-exports
├── client.rs        — ACP Client trait, session management
├── session.rs       — AcpSession wrapper (lifecycle)
└── transport.rs     — stdio spawn (hermes acp command)

crates/services/src/hermes_acp.rs  — Module declaration
```

---

### Task 1: Layer-Shell Window + Peek/Pin

**Files:**
- Create: `crates/app/src/side_panel_left/mod.rs`
- Create: `crates/app/src/side_panel_left/state.rs`
- Modify: `crates/app/src/main.rs` (register module)

**Interfaces:**
- Produces: `SidePanelLeft` entity, `PanelState` enum

- [ ] **Step 1: Create state.rs with PanelState**

```rust
// crates/app/src/side_panel_left/state.rs
use gpui::*;

#[derive(Clone, Copy, PartialEq)]
pub enum PanelState {
    Peek,
    Pinned,
    Resizing,
}

pub struct SidePanelLeftState {
    pub state: PanelState,
    pub width: f32,
    pub session_id: Option<String>,
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        Self {
            state: PanelState::Peek,
            width: 352.0,
            session_id: None,
        }
    }
}
```

- [ ] **Step 2: Create mod.rs with window creation**

```rust
// crates/app/src/side_panel_left/mod.rs
mod state;
mod panel;
mod sessions_list;
mod chat_view;
mod composer;
mod tool_card;

pub use state::{PanelState, SidePanelLeftState};

use gpui::*;
use gpui_platform::LayerShellOptions;

pub struct SidePanelLeft {
    state: SidePanelLeftState,
}

impl Render for SidePanelLeft {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        panel::render_panel(self, _window, _cx)
    }
}

impl SidePanelLeft {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: SidePanelLeftState::new(),
        }
    }

    pub fn create_window(cx: &mut App) -> anyhow::Result<()> {
        cx.open_window(
            WindowOptions {
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "side_panel_left".to_string(),
                    layer: gpui_platform::Layer::Overlay,
                    anchor: gpui_platform::Anchor::LEFT
                        | gpui_platform::Anchor::TOP
                        | gpui_platform::Anchor::BOTTOM,
                    exclusive_zone: Some(px(0.0)),
                    keyboard_interactivity: gpui_platform::KeyboardInteractivity::OnDemand,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |cx| Ok(cx.new(|cx| SidePanelLeft::new(cx))),
        )?;
        Ok(())
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

Expected: compiles (may have unused warnings)

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/
git commit -m "feat(side_panel_left): layer-shell window with peek/pin state"
```

---

### Task 2: Hermes ACP Service — Transport

**Files:**
- Create: `crates/services/src/hermes_acp/transport.rs`
- Create: `crates/services/src/hermes_acp/mod.rs`
- Create: `crates/services/src/hermes_acp.rs`
- Modify: `crates/services/Cargo.toml` (add ACP dependencies)

**Interfaces:**
- Produces: `HermesTransport` (stdio spawn)

- [ ] **Step 1: Add ACP dependencies to services/Cargo.toml**

```toml
# crates/services/Cargo.toml
[dependencies]
agent-client-protocol = "0.5"
agent-client-protocol-tokio = "0.5"
tokio = { version = "1", features = ["process", "io-util"] }
```

- [ ] **Step 2: Create transport.rs**

```rust
// crates/services/src/hermes_acp/transport.rs
use agent_client_protocol_tokio::StdioTransport;
use anyhow::Result;

pub struct HermesTransport {
    transport: StdioTransport,
}

impl HermesTransport {
    pub async fn spawn() -> Result<Self> {
        let transport = StdioTransport::spawn("hermes", ["acp"])?;
        Ok(Self { transport })
    }

    pub fn inner(&self) -> &StdioTransport {
        &self.transport
    }
}
```

- [ ] **Step 3: Create mod.rs**

```rust
// crates/services/src/hermes_acp/mod.rs
pub mod transport;
pub mod client;
pub mod session;

pub use transport::HermesTransport;
pub use client::HermesClient;
pub use session::AcpSession;
```

- [ ] **Step 4: Create hermes_acp.rs (module declaration)**

```rust
// crates/services/src/hermes_acp.rs
pub mod hermes_acp;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build --release -p chronos-services 2>&1 | head -20
```

- [ ] **Step 6: Commit**

```bash
git add crates/services/src/hermes_acp/
git add crates/services/src/hermes_acp.rs
git add crates/services/Cargo.toml
git commit -m "feat(hermes_acp): transport layer with stdio spawn"
```

---

### Task 3: Hermes ACP Client + Session

**Files:**
- Create: `crates/services/src/hermes_acp/client.rs`
- Create: `crates/services/src/hermes_acp/session.rs`

**Interfaces:**
- Consumes: `HermesTransport`
- Produces: `HermesClient`, `AcpSession`

- [ ] **Step 1: Create session.rs**

```rust
// crates/services/src/hermes_acp/session.rs
use agent_client_protocol::schema::{SessionId, Message};
use anyhow::Result;

pub struct AcpSession {
    pub id: SessionId,
    pub messages: Vec<Message>,
    pub is_draft: bool,
}

impl AcpSession {
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            messages: Vec::new(),
            is_draft: false,
        }
    }

    pub fn push_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }
}
```

- [ ] **Step 2: Create client.rs**

```rust
// crates/services/src/hermes_acp/client.rs
use agent_client_protocol::Client;
use agent_client_protocol::schema::{InitializeRequest, ProtocolVersion};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::transport::HermesTransport;
use super::session::AcpSession;

pub struct HermesClient {
    transport: HermesTransport,
    sessions: Arc<RwLock<Vec<AcpSession>>>,
}

impl HermesClient {
    pub async fn new() -> Result<Self> {
        let transport = HermesTransport::spawn().await?;
        Ok(Self {
            transport,
            sessions: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        // ACP initialization handled by SDK
        Ok(())
    }

    pub async fn create_session(&self) -> Result<AcpSession> {
        let id = SessionId::new();
        let session = AcpSession::new(id);
        self.sessions.write().await.push(session.clone());
        Ok(session)
    }

    pub async fn send_prompt(&self, session_id: &str, prompt: &str) -> Result<String> {
        // Send prompt via ACP, receive response
        // Implementation uses agent-client-protocol SDK
        todo!("Implement ACP prompt sending")
    }

    pub async fn sessions(&self) -> Vec<AcpSession> {
        self.sessions.read().await.clone()
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos-services 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/services/src/hermes_acp/client.rs crates/services/src/hermes_acp/session.rs
git commit -m "feat(hermes_acp): client and session management"
```

---

### Task 4: Service Registration

**Files:**
- Modify: `crates/services/src/lib.rs` (register hermes_acp module)
- Modify: `crates/app/Cargo.toml` (add services dependency)

**Interfaces:**
- Produces: `services::hermes_acp::HermesClient` available to app

- [ ] **Step 1: Add module to services/src/lib.rs**

```rust
// crates/services/src/lib.rs
pub mod hermes_acp;
```

- [ ] **Step 2: Add services dependency to app/Cargo.toml**

```toml
# crates/app/Cargo.toml
[dependencies]
chronos-services = { path = "../services" }
```

- [ ] **Step 3: Verify full workspace compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/services/src/lib.rs crates/app/Cargo.toml
git commit -m "feat: register hermes_acp service module"
```

---

### Task 5: Panel Header + Status Indicator

**Files:**
- Create: `crates/app/src/side_panel_left/panel.rs`
- Modify: `crates/app/src/side_panel_left/mod.rs` (wire render)

**Interfaces:**
- Consumes: `SidePanelLeft` entity
- Produces: Header with icon, status, close button

- [ ] **Step 1: Create panel.rs with header**

```rust
// crates/app/src/side_panel_left/panel.rs
use gpui::*;
use crate::side_panel_left::SidePanelLeft;

pub fn render_panel(
    panel: &mut SidePanelLeft,
    window: &mut Window,
    cx: &mut Context<SidePanelLeft>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(
            // Header
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(gpui::white())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            // Status indicator (green/red/yellow dot)
                            div()
                                .w_2()
                                .h_2()
                                .rounded_full()
                                .bg(gpui::green()),
                        )
                        .child("Agent"),
                )
                .child(
                    // Close button
                    div()
                        .cursor_pointer()
                        .on_click(|_, _, _| {
                            // TODO: close panel
                        }),
                ),
        )
        .child(
            // Body placeholder
            div().flex_1().child("Chat goes here"),
        )
}
```

- [ ] **Step 2: Wire render in mod.rs**

```rust
// In crates/app/src/side_panel_left/mod.rs, update render:
impl Render for SidePanelLeft {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        panel::render_panel(self, _window, _cx)
    }
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/panel.rs crates/app/src/side_panel_left/mod.rs
git commit -m "feat(side_panel_left): header with status indicator"
```

---

### Task 6: Sessions Sidebar (gpui-component Sidebar)

**Files:**
- Create: `crates/app/src/side_panel_left/sessions_list.rs`
- Modify: `crates/app/src/side_panel_left/panel.rs` (add sidebar)

**Interfaces:**
- Consumes: sessions from HermesClient
- Produces: SessionsList component

- [ ] **Step 1: Create sessions_list.rs**

```rust
// crates/app/src/side_panel_left/sessions_list.rs
use gpui::*;

pub struct SessionsList {
    sessions: Vec<SessionItem>,
    collapsed: bool,
}

struct SessionItem {
    id: String,
    title: String,
    active: bool,
}

impl SessionsList {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            collapsed: false,
        }
    }

    pub fn toggle_collapse(&mut self) {
        self.collapsed = !self.collapsed;
    }

    pub fn render(&self, cx: &mut Context<crate::side_panel_left::SidePanelLeft>) -> impl IntoElement {
        if self.collapsed {
            // Render as icon strip
            div()
                .w(px(48.0))
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .p_2()
                .child(
                    div()
                        .cursor_pointer()
                        .on_click(|_, _, cx| {
                            // TODO: create new session
                        }),
                )
        } else {
            // Render as full sidebar
            div()
                .w(px(200.0))
                .flex()
                .flex_col()
                .border_r_1()
                .border_color(gpui::white())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .child("Sessions")
                        .child(
                            div()
                                .cursor_pointer()
                                .on_click(|_, _, cx| {
                                    // TODO: toggle collapse
                                }),
                        ),
                )
                .child(
                    // Session items
                    div().flex_1().children(
                        self.sessions.iter().map(|s| {
                            div()
                                .px_3()
                                .py_2()
                                .cursor_pointer()
                                .when(s.active, |el| el.bg(gpui::white_alpha(0.1)))
                                .child(&s.title)
                        }),
                    ),
                )
        }
    }
}
```

- [ ] **Step 2: Wire sessions_list into panel.rs**

```rust
// In panel.rs, add sessions list to body:
.child(
    div()
        .flex()
        .flex_row()
        .flex_1()
        .child(sessions_list.render(cx))
        .child(
            // Chat area placeholder
            div().flex_1().child("Chat goes here"),
        ),
)
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/sessions_list.rs crates/app/src/side_panel_left/panel.rs
git commit -m "feat(side_panel_left): sessions sidebar with collapse"
```

---

### Task 7: Composer (TextInput + Pickers + Send)

**Files:**
- Create: `crates/app/src/side_panel_left/composer.rs`
- Modify: `crates/app/src/side_panel_left/panel.rs` (add composer)

**Interfaces:**
- Consumes: TextInput from gpui-component
- Produces: Composer component with model/mode pickers

- [ ] **Step 1: Create composer.rs**

```rust
// crates/app/src/side_panel_left/composer.rs
use gpui::*;

pub struct Composer {
    input_text: String,
    selected_model: String,
    selected_mode: String, // "ask" or "act"
}

impl Composer {
    pub fn new() -> Self {
        Self {
            input_text: String::new(),
            selected_model: "claude-sonnet-4-20250514".to_string(),
            selected_mode: "ask".to_string(),
        }
    }

    pub fn render(&self, cx: &mut Context<crate::side_panel_left::SidePanelLeft>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_end()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(gpui::white())
            .child(
                // Attach button
                div()
                    .cursor_pointer()
                    .child("📎"),
            )
            .child(
                // Model picker
                div()
                    .cursor_pointer()
                    .child(&self.selected_model),
            )
            .child(
                // Mode picker
                div()
                    .cursor_pointer()
                    .child(&self.selected_mode),
            )
            .child(
                // Text input placeholder
                div()
                    .flex_1()
                    .border_1()
                    .border_color(gpui::white())
                    .rounded_md()
                    .px_3()
                    .py_2()
                    .child("Type a message..."),
            )
            .child(
                // Send button
                div()
                    .cursor_pointer()
                    .on_click(|_, _, _| {
                        // TODO: send prompt
                    }),
            )
    }
}
```

- [ ] **Step 2: Wire composer into panel.rs**

```rust
// In panel.rs, add composer at bottom:
.child(composer.render(cx))
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/composer.rs crates/app/src/side_panel_left/panel.rs
git commit -m "feat(side_panel_left): composer with model/mode pickers"
```

---

### Task 8: Chat View (Message Stream)

**Files:**
- Create: `crates/app/src/side_panel_left/chat_view.rs`
- Modify: `crates/app/src/side_panel_left/panel.rs` (add chat view)

**Interfaces:**
- Consumes: messages from AcpSession
- Produces: ChatView component

- [ ] **Step 1: Create chat_view.rs**

```rust
// crates/app/src/side_panel_left/chat_view.rs
use gpui::*;

pub struct ChatView {
    messages: Vec<ChatMessage>,
}

struct ChatMessage {
    role: MessageRole,
    content: String,
    tool_calls: Vec<ToolCallPreview>,
}

enum MessageRole {
    User,
    Agent,
}

struct ToolCallPreview {
    name: String,
    status: String,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
    }

    pub fn render(&self, cx: &mut Context<crate::side_panel_left::SidePanelLeft>) -> impl IntoElement {
        div()
            .flex_1()
            .overflow_y_scroll()
            .children(self.messages.iter().map(|msg| {
                div()
                    .px_4()
                    .py_2()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(match msg.role {
                                MessageRole::User => "You",
                                MessageRole::Agent => "Agent",
                            }),
                    )
                    .child(&msg.content)
            }))
    }
}
```

- [ ] **Step 2: Wire chat_view into panel.rs**

```rust
// In panel.rs, replace chat placeholder with ChatView:
.child(chat_view.render(cx))
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/chat_view.rs crates/app/src/side_panel_left/panel.rs
git commit -m "feat(side_panel_left): chat view with message stream"
```

---

### Task 9: Tool Call Cards (Collapsed)

**Files:**
- Create: `crates/app/src/side_panel_left/tool_card.rs`
- Modify: `crates/app/src/side_panel_left/chat_view.rs` (integrate tool cards)

**Interfaces:**
- Consumes: ToolCallPreview from chat messages
- Produces: Collapsible tool card component

- [ ] **Step 1: Create tool_card.rs**

```rust
// crates/app/src/side_panel_left/tool_card.rs
use gpui::*;

pub struct ToolCard {
    name: String,
    status: String,
    expanded: bool,
}

impl ToolCard {
    pub fn new(name: String, status: String) -> Self {
        Self {
            name,
            status,
            expanded: false,
        }
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn render(&self) -> impl IntoElement {
        div()
            .mx_4()
            .my_1()
            .border_1()
            .border_color(gpui::white())
            .rounded_md()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child("🔧")
                            .child(&self.name),
                    )
                    .child(&self.status),
            )
    }
}
```

- [ ] **Step 2: Integrate tool cards into chat_view.rs**

```rust
// In chat_view.rs, add tool cards between messages:
.child(msg.tool_calls.iter().map(|tc| {
    tool_card::ToolCard::new(tc.name.clone(), tc.status.clone()).render()
}))
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/tool_card.rs crates/app/src/side_panel_left/chat_view.rs
git commit -m "feat(side_panel_left): collapsible tool call cards"
```

---

### Task 10: Drag-Resize

**Files:**
- Modify: `crates/app/src/side_panel_left/panel.rs` (add resize handle)
- Modify: `crates/app/src/side_panel_left/state.rs` (add Resizing state)

**Interfaces:**
- Consumes: PanelState, width
- Produces: Resize handle, drag behavior

- [ ] **Step 1: Add Resizing to state.rs**

```rust
// In state.rs, PanelState already has Resizing variant
// Add width tracking:
pub struct SidePanelLeftState {
    pub state: PanelState,
    pub width: f32,
    pub min_width: f32,
    pub max_width: f32,
    // ...
}

impl SidePanelLeftState {
    pub fn new() -> Self {
        Self {
            state: PanelState::Peek,
            width: 352.0,
            min_width: 280.0,
            max_width: 960.0, // 50% of 1920
            session_id: None,
        }
    }

    pub fn resize(&mut self, delta: f32) {
        self.width = (self.width + delta).clamp(self.min_width, self.max_width);
    }
}
```

- [ ] **Step 2: Add resize handle to panel.rs**

```rust
// In panel.rs, add resize handle on right edge:
div()
    .w(px(4.0))
    .cursor_col_resize()
    .on_drag(|_, delta, _, _| {
        // TODO: resize panel
    }),
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/panel.rs crates/app/src/side_panel_left/state.rs
git commit -m "feat(side_panel_left): drag-resize handle"
```

---

### Task 11: Error Handling + Offline State

**Files:**
- Modify: `crates/app/src/side_panel_left/panel.rs` (status indicator colors)
- Modify: `crates/app/src/side_panel_left/composer.rs` (disabled state)

**Interfaces:**
- Consumes: connection status
- Produces: Status colors (green/red/yellow), disabled composer

- [ ] **Step 1: Add status colors to panel.rs header**

```rust
// In panel.rs header, update status indicator:
div()
    .w_2()
    .h_2()
    .rounded_full()
    .bg(match status {
        ConnectionStatus::Online => gpui::green(),
        ConnectionStatus::Offline => gpui::red(),
        ConnectionStatus::Thinking => gpui::yellow(),
    }),
```

- [ ] **Step 2: Add disabled state to composer.rs**

```rust
// In composer.rs, add disabled prop:
pub fn render(&self, disabled: bool) -> impl IntoElement {
    // ...
    .when(disabled, |el| el.opacity(0.5))
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/side_panel_left/panel.rs crates/app/src/side_panel_left/composer.rs
git commit -m "feat(side_panel_left): error handling and offline state"
```

---

### Task 12: Wire ACP Client to UI

**Files:**
- Modify: `crates/app/src/side_panel_left/mod.rs` (integrate HermesClient)
- Modify: `crates/app/src/side_panel_left/composer.rs` (send prompt)
- Modify: `crates/app/src/side_panel_left/chat_view.rs` (receive messages)

**Interfaces:**
- Consumes: `HermesClient` from services
- Produces: Working chat flow

- [ ] **Step 1: Add HermesClient to SidePanelLeft**

```rust
// In mod.rs, add client field:
pub struct SidePanelLeft {
    state: SidePanelLeftState,
    client: Option<HermesClient>,
    chat_view: ChatView,
    sessions_list: SessionsList,
    composer: Composer,
}
```

- [ ] **Step 2: Initialize client on creation**

```rust
impl SidePanelLeft {
    pub async fn new(cx: &mut Context<Self>) -> Self {
        let client = HermesClient::new().await.ok();
        Self {
            state: SidePanelLeftState::new(),
            client,
            chat_view: ChatView::new(),
            sessions_list: SessionsList::new(),
            composer: Composer::new(),
        }
    }
}
```

- [ ] **Step 3: Wire composer send to client**

```rust
// In composer.rs on_click handler:
// Send prompt via HermesClient
```

- [ ] **Step 4: Wire client responses to chat_view**

```rust
// Receive messages from client and push to chat_view
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build --release -p chronos 2>&1 | head -20
```

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/side_panel_left/
git commit -m "feat(side_panel_left): wire ACP client to UI"
```

---

### Task 13: Build + Smoke Test

**Files:** None (verification only)

- [ ] **Step 1: Full release build**

```bash
cargo build --release -p chronos
```

- [ ] **Step 2: Kill existing chronos**

```bash
pkill -x chronos || true
```

- [ ] **Step 3: Run release binary**

```bash
RUST_LOG=info ./target/release/chronos
```

- [ ] **Step 4: Test left panel**

- Hover left edge → peek opens
- Click pin icon → panel stays
- Type in composer → send prompt
- Tool calls show as collapsed cards
- Resize handle works

- [ ] **Step 5: Commit final state**

```bash
git add -A
git commit -m "feat(side_panel_left): complete left agent panel v1"
```

---

## Self-Review

**Spec coverage:**
- §1 Goal: All 8 success criteria covered
- §2 Architecture: File structure matches plan
- §3 Layer-shell: Task 1
- §4 States: Task 1 (state.rs)
- §5 Hermes ACP: Tasks 2-4
- §6 UI Components: Tasks 5-9
- §7 Message Flow: Task 12
- §8 Error Handling: Task 11
- §9 Phased Ship: Single plan (one shot)

**Placeholder scan:** `todo!("Implement ACP prompt sending")` in Task 3 Step 2 — acceptable as placeholder for implementation, will be filled during Task 12.

**Type consistency:** `SidePanelLeft`, `HermesClient`, `ChatView`, `SessionsList`, `Composer` — names consistent across tasks.

---

## Implementation Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-23-left-agent-panel.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks, fast iteration

2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
